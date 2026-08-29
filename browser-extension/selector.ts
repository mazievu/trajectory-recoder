/**
 * Robust Multi-Strategy DOM Selector Engine.
 * Generates unique, resilient selectors for web elements.
 */

export interface ElementSelectors {
  id: string | null;
  tag: string;
  role: string | null;
  visibleText: string | null;
  ariaLabel: string | null;
  className: string | null;
  href: string | null;
  placeholder: string | null;
  inputType: string | null;
  cssSelector: string;
  xpath: string;
}

export function extractElementSelectors(el: HTMLElement): ElementSelectors {
  const tag = el.tagName.toLowerCase();
  const id = el.id ? el.id : null;
  const role = el.getAttribute('role');
  const ariaLabel = el.getAttribute('aria-label') || el.getAttribute('aria-labelledby');
  const className = el.className && typeof el.className === 'string' ? el.className.trim() : null;
  const href = el.getAttribute('href');
  const placeholder = el.getAttribute('placeholder');
  const inputType = el.getAttribute('type');
  
  // Extract visible text (truncated to 100 chars)
  let visibleText: string | null = el.innerText || el.textContent || null;
  if (visibleText) {
    visibleText = visibleText.trim().substring(0, 100);
    if (visibleText.length === 0) {
      visibleText = null;
    }
  }

  const cssSelector = getCssSelector(el);
  const xpath = getXPath(el);

  return {
    id,
    tag,
    role,
    visibleText,
    ariaLabel,
    className,
    href,
    placeholder,
    inputType,
    cssSelector,
    xpath,
  };
}

export function getCssSelector(el: HTMLElement): string {
  if (el.id) {
    return `#${CSS.escape(el.id)}`;
  }

  const path: string[] = [];
  let current: HTMLElement | null = el;

  while (current && current.nodeType === Node.ELEMENT_NODE) {
    let selector = current.tagName.toLowerCase();
    
    if (current.id) {
      selector += `#${CSS.escape(current.id)}`;
      path.unshift(selector);
      break;
    }

    if (current.className && typeof current.className === 'string') {
      const classes = current.className.trim().split(/\s+/).filter(c => c.length > 0 && !c.includes(':'));
      if (classes.length > 0) {
        selector += `.${classes.map(c => CSS.escape(c)).join('.')}`;
      }
    }

    const parent: HTMLElement | null = current.parentElement;
    if (parent) {
      const siblings = Array.from(parent.children).filter(c => c.tagName === current!.tagName);
      if (siblings.length > 1) {
        const index = siblings.indexOf(current) + 1;
        selector += `:nth-of-type(${index})`;
      }
    }

    path.unshift(selector);
    current = parent;
  }

  return path.join(' > ');
}

export function getXPath(el: HTMLElement): string {
  if (el.id) {
    return `//*[@id="${el.id}"]`;
  }

  const segments: string[] = [];
  let current: HTMLElement | null = el;

  while (current && current.nodeType === Node.ELEMENT_NODE) {
    let index = 1;
    let sibling = current.previousElementSibling;
    while (sibling) {
      if (sibling.tagName === current.tagName) {
        index++;
      }
      sibling = sibling.previousElementSibling;
    }

    const tagName = current.tagName.toLowerCase();
    const segment = `${tagName}[${index}]`;
    segments.unshift(segment);
    current = current.parentElement;
  }

  return `/${segments.join('/')}`;
}
