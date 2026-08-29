import { extractElementSelectors } from './selector';

const SENSITIVE_FIELD = /(?:pass(?:word)?|secret|token|api[_-]?key|auth|otp|one[\s_-]?time|cc(?:num|number)?|card(?:number)?|cvv|cvc)/i;
const SENSITIVE_AUTOCOMPLETE = /(?:current-password|new-password|one-time-code|cc-|webauthn)/i;
const SENSITIVE_QUERY_PARAM = /(?:pass(?:word)?|secret|token|api[_-]?key|auth|code|otp|cc(?:num|number)?|card|cvv|cvc)/i;
const UNOBSERVED_TEXT = '[UNOBSERVED_TEXT]';
const PASSWORD_REDACTED = '[PASSWORD_REDACTED]';

function sanitizedUrl(rawUrl: string): string {
  try {
    const url = new URL(rawUrl);
    for (const key of Array.from(url.searchParams.keys())) {
      if (SENSITIVE_QUERY_PARAM.test(key)) {
        url.searchParams.set(key, '[REDACTED]');
      }
    }
    if (url.password) {
      url.password = '[REDACTED]';
    }
    return url.toString();
  } catch {
    // An invalid URL may contain credentials; do not emit it.
    return 'about:blank#unparseable-url';
  }
}

function fieldIsSensitive(target: HTMLElement): boolean {
  const input = target as HTMLInputElement;
  const attributes = [
    target.getAttribute('name'),
    target.id,
    target.getAttribute('aria-label'),
    target.getAttribute('autocomplete'),
    target.getAttribute('type'),
  ].filter(Boolean).join(' ');
  return input.type === 'password'
    || SENSITIVE_FIELD.test(attributes)
    || SENSITIVE_AUTOCOMPLETE.test(target.getAttribute('autocomplete') || '');
}

function currentValueLength(target: HTMLElement): number | null {
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) {
    return target.value.length;
  }
  return null;
}

// Form values are always represented by a fixed marker plus their length.
// Never transport plaintext typed by the user to the service worker or host.
function emitEvent(eventType: string, target: HTMLElement, mutationInfo?: any) {
  try {
    const isSensitive = fieldIsSensitive(target);
    const selectors = extractElementSelectors(target);
    const valueLength = currentValueLength(target);
    const value = valueLength === null
      ? null
      : (isSensitive ? PASSWORD_REDACTED : UNOBSERVED_TEXT);

    const payload = {
      tab_id: 0, // Injected by background
      url: sanitizedUrl(window.location.href),
      page_title: document.title,
      event_type: eventType,
      tag: selectors.tag,
      role: selectors.role,
      visible_text: selectors.visibleText,
      aria_label: selectors.ariaLabel,
      element_id: selectors.id,
      class_name: selectors.className,
      href: selectors.href,
      placeholder: selectors.placeholder,
      input_type: selectors.inputType,
      value,
      value_length: valueLength,
      css_selector: selectors.cssSelector,
      xpath: selectors.xpath,
      timestamp_ms: Date.now(),
      is_password: isSensitive,
      mutation_info: mutationInfo || null,
    };

    chrome.runtime.sendMessage({ type: 'TRAJECTORY_DOM_EVENT', payload });
  } catch (err) {
    console.debug('Trajectory capture error:', err);
  }
}

// 1. Click Listener (Capture phase)
document.addEventListener('click', (e) => {
  const target = e.target as HTMLElement;
  if (target) {
    emitEvent('CLICK', target);
  }
}, true);

document.addEventListener('dblclick', (e) => {
  const target = e.target as HTMLElement;
  if (target) emitEvent('DOUBLE_CLICK', target);
}, true);

// 2. Form edits are debounced. This records that a field changed and its safe
// length marker, instead of a per-keystroke stream or plaintext content.
const pendingInputTimers = new WeakMap<HTMLElement, ReturnType<typeof setTimeout>>();
document.addEventListener('input', (e) => {
  const target = e.target as HTMLElement;
  if (!target) return;
  const previous = pendingInputTimers.get(target);
  if (previous) clearTimeout(previous);
  pendingInputTimers.set(target, setTimeout(() => {
    pendingInputTimers.delete(target);
    emitEvent('INPUT', target);
  }, 500));
}, true);

document.addEventListener('focusin', (e) => {
  const target = e.target as HTMLElement;
  if (target) emitEvent('FOCUS', target);
}, true);

document.addEventListener('focusout', (e) => {
  const target = e.target as HTMLElement;
  if (target) emitEvent('BLUR', target);
}, true);

// 3. Change Listener
document.addEventListener('change', (e) => {
  const target = e.target as HTMLElement;
  if (target) {
    emitEvent('CHANGE', target);
  }
}, true);

// 4. Form Submit Listener
document.addEventListener('submit', (e) => {
  const target = e.target as HTMLElement;
  if (target) {
    emitEvent('SUBMIT', target);
  }
}, true);

document.addEventListener('dragstart', (e) => {
  const target = e.target as HTMLElement;
  if (target) emitEvent('DRAG_START', target);
}, true);

document.addEventListener('drop', (e) => {
  const target = e.target as HTMLElement;
  if (target) emitEvent('DROP', target);
}, true);

let pendingScrollTimer: ReturnType<typeof setTimeout> | null = null;
let pendingScrollTarget: HTMLElement | null = null;
document.addEventListener('scroll', (e) => {
  pendingScrollTarget = e.target instanceof HTMLElement ? e.target : document.body;
  if (pendingScrollTimer) clearTimeout(pendingScrollTimer);
  pendingScrollTimer = setTimeout(() => {
    if (pendingScrollTarget) emitEvent('SCROLL', pendingScrollTarget);
    pendingScrollTarget = null;
    pendingScrollTimer = null;
  }, 250);
}, true);

// 5. MutationObserver for dynamic dialogs, modals, and toasts
const observer = new MutationObserver((mutations) => {
  for (const mutation of mutations) {
    if (mutation.type === 'childList' && mutation.addedNodes.length > 0) {
      for (let i = 0; i < mutation.addedNodes.length; i++) {
        const node = mutation.addedNodes[i];
        if (node.nodeType === Node.ELEMENT_NODE) {
          const el = node as HTMLElement;
          const role = el.getAttribute('role');
          const isModal = role === 'dialog' || role === 'alertdialog' || role === 'alert' || el.classList.contains('modal') || el.classList.contains('toast');
          if (isModal) {
            emitEvent('MUTATION', el, {
              mutation_type: 'childList',
              added_nodes_count: mutation.addedNodes.length,
              removed_nodes_count: mutation.removedNodes.length,
              attribute_name: null,
              target_summary: el.tagName,
            });
          }
        }
      }
    }
  }
});

observer.observe(document.documentElement, {
  childList: true,
  subtree: true,
  attributes: false,
});

// 6. SPA Navigation Detection (pushState / replaceState / popstate / hashchange)
let lastUrl = window.location.href;
function checkSpaNavigation() {
  if (window.location.href !== lastUrl) {
    lastUrl = window.location.href;
    emitEvent('SPA_NAVIGATION', document.body);
  }
}

window.addEventListener('popstate', checkSpaNavigation);
window.addEventListener('hashchange', checkSpaNavigation);

const originalPushState = history.pushState;
history.pushState = function (...args) {
  originalPushState.apply(this, args);
  checkSpaNavigation();
};

const originalReplaceState = history.replaceState;
history.replaceState = function (...args) {
  originalReplaceState.apply(this, args);
  checkSpaNavigation();
};
