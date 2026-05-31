/**
 * @file dom.ts
 * @brief Lightweight DOM helpers (browser-only).
 */

/**
 * Select a single element matching `selector`, throwing if absent.
 *
 * @param selector - CSS selector string.
 * @param root     - Search root; defaults to `document`.
 * @returns The first matching element.
 * @throws {Error} When no element matches `selector`.
 */
export function qs<T extends Element = Element>(
    selector: string,
    root: ParentNode = document,
): T {
    const el = root.querySelector<T>(selector);
    if (!el) throw new Error(`No element matches "${selector}"`);
    return el;
}

/**
 * Select all elements matching `selector` as a plain array.
 *
 * @param selector - CSS selector string.
 * @param root     - Search root; defaults to `document`.
 * @returns Possibly-empty array of matching elements.
 */
export function qsa<T extends Element = Element>(
    selector: string,
    root: ParentNode = document,
): T[] {
    return Array.from(root.querySelectorAll<T>(selector));
}

/**
 * Add a delegated event listener to `root`.
 *
 * Fires `handler` whenever an event matching `selector` bubbles up to
 * `root`.  Returns a cleanup function that removes the listener.
 *
 * @param root     - Element that receives bubbled events.
 * @param event    - Event type, e.g. `"click"`.
 * @param selector - CSS selector for the target element.
 * @param handler  - Called with the matching target cast to `T`.
 * @returns Zero-argument cleanup function.
 */
export function delegate<T extends Element>(
    root: Element,
    event: string,
    selector: string,
    handler: (target: T, ev: Event) => void,
): () => void {
    const listener = (ev: Event) => {
        const target = (ev.target as Element).closest<T>(selector);
        if (target && root.contains(target)) handler(target, ev);
    };
    root.addEventListener(event, listener);
    return () => root.removeEventListener(event, listener);
}
