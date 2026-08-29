import { extractElementSelectors } from './selector';

// Send DOM event to background service worker
function emitEvent(eventType: string, target: HTMLElement, valueOverride?: string, mutationInfo?: any) {
  try {
    const isPassword = target.tagName === 'INPUT' && (target as HTMLInputElement).type === 'password';
    const selectors = extractElementSelectors(target);
    const value = isPassword
      ? '[PASSWORD_REDACTED]'
      : valueOverride !== undefined
      ? valueOverride
      : (target as any).value || null;

    const payload = {
      tab_id: 0, // Injected by background
      url: window.location.href,
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
      value: value ? String(value).substring(0, 500) : null,
      css_selector: selectors.cssSelector,
      xpath: selectors.xpath,
      timestamp_ms: Date.now(),
      is_password: isPassword,
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

// 2. Change / Input Listener
document.addEventListener('change', (e) => {
  const target = e.target as HTMLElement;
  if (target) {
    emitEvent('CHANGE', target);
  }
}, true);

// 3. Form Submit Listener
document.addEventListener('submit', (e) => {
  const target = e.target as HTMLElement;
  if (target) {
    emitEvent('SUBMIT', target);
  }
}, true);

// 4. MutationObserver for dynamic dialogs, modals, and toasts
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
            emitEvent('MUTATION', el, undefined, {
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

// 5. SPA Navigation Detection (pushState / replaceState / popstate / hashchange)
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
