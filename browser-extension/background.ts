/**
 * Manifest V3 service worker. It is the sole bridge from extension telemetry
 * to the native host and keeps a bounded, loss-visible queue while that host
 * is reconnecting.
 */

const NATIVE_HOST_NAME = 'com.trajectory.recorder.browser_host';
const MAX_PENDING_EVENTS = 1_000;
const UNOBSERVED_TEXT = '[UNOBSERVED_TEXT]';
const PASSWORD_REDACTED = '[PASSWORD_REDACTED]';
const SENSITIVE_QUERY_PARAM = /(?:pass(?:word)?|secret|token|api[_-]?key|auth|code|otp|cc(?:num|number)?|card|cvv|cvc)/i;

let nativePort: chrome.runtime.Port | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
const pendingEvents: Record<string, unknown>[] = [];
const tabMetadata = new Map<number, { url: string; title: string }>();

function sanitizedUrl(rawUrl: string): string {
  try {
    const url = new URL(rawUrl);
    for (const key of Array.from(url.searchParams.keys())) {
      if (SENSITIVE_QUERY_PARAM.test(key)) url.searchParams.set(key, '[REDACTED]');
    }
    if (url.password) url.password = '[REDACTED]';
    return url.toString();
  } catch {
    return 'about:blank#unparseable-url';
  }
}

function sanitizedPayload(candidate: Record<string, any>): Record<string, unknown> {
  const hasValue = candidate.value !== null && candidate.value !== undefined;
  const valueLength = Number.isSafeInteger(candidate.value_length) && candidate.value_length >= 0
    ? candidate.value_length
    : null;

  // Defence in depth: content scripts must already have scrubbed values, but
  // the service worker refuses to forward plaintext from any sender.
  return {
    ...candidate,
    url: typeof candidate.url === 'string' ? sanitizedUrl(candidate.url) : 'about:blank#missing-url',
    value: hasValue || valueLength !== null
      ? (candidate.is_password ? PASSWORD_REDACTED : UNOBSERVED_TEXT)
      : null,
    value_length: valueLength,
  };
}

function queueEvent(payload: Record<string, unknown>) {
  if (pendingEvents.length >= MAX_PENDING_EVENTS) {
    pendingEvents.shift();
    console.warn('Trajectory browser telemetry queue full; dropped oldest event');
  }
  pendingEvents.push(payload);
}

function sendOrQueue(payload: Record<string, unknown>): 'sent' | 'queued' {
  if (!nativePort) {
    queueEvent(payload);
    return 'queued';
  }
  try {
    nativePort.postMessage(payload);
    return 'sent';
  } catch (err) {
    console.warn('Native host send failed; queuing browser event', err);
    queueEvent(payload);
    return 'queued';
  }
}

function flushPendingEvents() {
  while (nativePort && pendingEvents.length > 0) {
    const next = pendingEvents[0];
    try {
      nativePort.postMessage(next);
      pendingEvents.shift();
    } catch (err) {
      console.warn('Native host flush failed; retaining browser telemetry', err);
      return;
    }
  }
}

function connectNativeHost() {
  try {
    nativePort = chrome.runtime.connectNative(NATIVE_HOST_NAME);
    nativePort.onMessage.addListener(() => {
      // Native messaging does not provide a durable agent acknowledgement.
    });
    nativePort.onDisconnect.addListener(() => {
      nativePort = null;
      if (!reconnectTimer) {
        reconnectTimer = setTimeout(() => {
          reconnectTimer = null;
          connectNativeHost();
        }, 5_000);
      }
    });
    flushPendingEvents();
  } catch (err) {
    console.error('Failed to connect to native messaging host:', err);
  }
}

function lifecyclePayload(eventType: string, tabId: number, url: string, pageTitle: string): Record<string, unknown> {
  return {
    tab_id: tabId,
    url,
    page_title: pageTitle,
    event_type: eventType,
    tag: eventType.startsWith('TAB_') ? 'tab' : 'document',
    role: null,
    visible_text: null,
    aria_label: null,
    element_id: null,
    class_name: null,
    href: null,
    placeholder: null,
    input_type: null,
    value: null,
    value_length: null,
    css_selector: null,
    xpath: null,
    timestamp_ms: Date.now(),
    is_password: false,
    mutation_info: null,
  };
}

function emitLifecycle(eventType: string, tabId: number, url: string, pageTitle: string) {
  const safeUrl = url ? sanitizedUrl(url) : '';
  if (safeUrl) tabMetadata.set(tabId, { url: safeUrl, title: pageTitle });
  sendOrQueue(lifecyclePayload(eventType, tabId, safeUrl, pageTitle));
}

connectNativeHost();

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type !== 'TRAJECTORY_DOM_EVENT') return false;

  const tabId = sender.tab?.id;
  if (tabId === undefined) {
    sendResponse({ status: 'rejected', error: 'missing tab id' });
    return false;
  }

  const payload = sanitizedPayload({ ...message.payload, tab_id: tabId });
  const url = typeof payload.url === 'string' ? payload.url : '';
  const title = typeof payload.page_title === 'string' ? payload.page_title : '';
  if (url) tabMetadata.set(tabId, { url, title });
  sendResponse({ status: sendOrQueue(payload) });
  return false;
});

chrome.tabs.onCreated.addListener((tab) => {
  emitLifecycle('TAB_CREATED', tab.id!, tab.url || '', tab.title || '');
});

chrome.tabs.onRemoved.addListener((tabId) => {
  const previous = tabMetadata.get(tabId);
  emitLifecycle('TAB_CLOSED', tabId, previous?.url || '', previous?.title || '');
  tabMetadata.delete(tabId);
});

chrome.tabs.onActivated.addListener(async (activeInfo) => {
  try {
    const tab = await chrome.tabs.get(activeInfo.tabId);
    emitLifecycle('TAB_ACTIVATED', tab.id!, tab.url || '', tab.title || '');
  } catch {
    // The tab can close between activation and lookup.
  }
});

chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.title !== undefined) {
    emitLifecycle('TAB_TITLE_UPDATED', tabId, tab.url || '', changeInfo.title);
  }
});

chrome.webNavigation.onCommitted.addListener(async (details) => {
  if (details.frameId !== 0) return;
  try {
    const tab = await chrome.tabs.get(details.tabId);
    emitLifecycle('NAVIGATION_COMMITTED', details.tabId, details.url, tab.title || '');
  } catch {
    emitLifecycle('NAVIGATION_COMMITTED', details.tabId, details.url, '');
  }
});

chrome.webNavigation.onCompleted.addListener(async (details) => {
  if (details.frameId !== 0) return;
  try {
    const tab = await chrome.tabs.get(details.tabId);
    emitLifecycle('NAVIGATION_COMPLETED', details.tabId, details.url, tab.title || '');
  } catch {
    emitLifecycle('NAVIGATION_COMPLETED', details.tabId, details.url, '');
  }
});

chrome.webNavigation.onErrorOccurred.addListener((details) => {
  if (details.frameId === 0) {
    emitLifecycle('NAVIGATION_ERROR', details.tabId, details.url, '');
  }
});
