/**
 * Background Service Worker for Manifest V3 Extension.
 * Manages Native Messaging Port connection to trajectory-browser-host.exe.
 */

const NATIVE_HOST_NAME = 'com.trajectory.recorder.browser_host';
let nativePort: chrome.runtime.Port | null = null;
let reconnectTimer: any = null;

function connectNativeHost() {
  try {
    nativePort = chrome.runtime.connectNative(NATIVE_HOST_NAME);
    
    nativePort.onMessage.addListener((msg) => {
      // Received response from native host
    });

    nativePort.onDisconnect.addListener(() => {
      console.warn('Native host disconnected. Will retry in 5s...');
      nativePort = null;
      if (!reconnectTimer) {
        reconnectTimer = setTimeout(() => {
          reconnectTimer = null;
          connectNativeHost();
        }, 5000);
      }
    });
  } catch (err) {
    console.error('Failed to connect to native messaging host:', err);
  }
}

connectNativeHost();

// Forward content script DOM messages to Native Host
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'TRAJECTORY_DOM_EVENT') {
    const payload = message.payload;
    if (sender.tab && sender.tab.id) {
      payload.tab_id = sender.tab.id;
    }
    
    if (nativePort) {
      try {
        nativePort.postMessage(payload);
        sendResponse({ status: 'sent' });
      } catch (e) {
        sendResponse({ status: 'error', error: String(e) });
      }
    } else {
      sendResponse({ status: 'unconnected' });
    }
  }
  return true;
});

// Listen to Tab Lifecycle Events
chrome.tabs.onActivated.addListener(async (activeInfo) => {
  try {
    const tab = await chrome.tabs.get(activeInfo.tabId);
    if (tab && tab.url && nativePort) {
      nativePort.postMessage({
        tab_id: tab.id,
        url: tab.url,
        page_title: tab.title || '',
        event_type: 'TAB_ACTIVATED',
        tag: 'tab',
        role: null,
        visible_text: null,
        aria_label: null,
        element_id: null,
        class_name: null,
        href: null,
        placeholder: null,
        input_type: null,
        value: null,
        css_selector: null,
        xpath: null,
        timestamp_ms: Date.now(),
        is_password: false,
        mutation_info: null,
      });
    }
  } catch (err) {
    // Tab may be closing
  }
});
