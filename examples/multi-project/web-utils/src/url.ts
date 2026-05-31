/**
 * @file url.ts
 * @brief URL manipulation and query-string helpers.
 */

/**
 * Parse the query string of a URL into a plain key-value object.
 *
 * Handles repeated keys by returning the last value; use
 * {@link parseQueryAll} when multiple values per key are needed.
 *
 * @param url - Absolute or relative URL.  A bare query string
 *              starting with `?` is also accepted.
 * @returns Object whose keys are parameter names and values are
 *          decoded parameter values.
 *
 * @example
 * parseQuery('https://example.com/search?q=hello&page=2')
 * // => { q: 'hello', page: '2' }
 */
export function parseQuery(url: string): Record<string, string> {
    const qs = url.includes('?') ? url.slice(url.indexOf('?') + 1) : url.replace(/^\?/, '');
    if (!qs) return {};
    return Object.fromEntries(
        qs.split('&').map(pair => {
            const [k, v = ''] = pair.split('=');
            return [decodeURIComponent(k), decodeURIComponent(v)];
        })
    );
}

/**
 * Like {@link parseQuery} but preserves all values for repeated keys.
 *
 * @param url - URL or bare query string.
 * @returns Object mapping each key to an array of values.
 */
export function parseQueryAll(url: string): Record<string, string[]> {
    const qs = url.includes('?') ? url.slice(url.indexOf('?') + 1) : url.replace(/^\?/, '');
    const result: Record<string, string[]> = {};
    if (!qs) return result;
    for (const pair of qs.split('&')) {
        const [k, v = ''] = pair.split('=');
        const key = decodeURIComponent(k);
        (result[key] ??= []).push(decodeURIComponent(v));
    }
    return result;
}

/**
 * Serialise an object into a URL query string (without leading `?`).
 *
 * @param params - Key-value pairs to encode.  Values are coerced to
 *                 strings via `String()`; `null` / `undefined` values
 *                 are omitted.
 * @returns Percent-encoded query string, e.g. `"q=hello&page=2"`.
 * @see parseQuery
 */
export function buildQuery(params: Record<string, unknown>): string {
    return Object.entries(params)
        .filter(([, v]) => v != null)
        .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`)
        .join('&');
}

/**
 * Join URL path segments, collapsing repeated slashes.
 *
 * @param segments - Path parts.  Leading and trailing slashes on each
 *                   segment are trimmed before joining.
 * @returns Joined path with a single `/` between segments.
 *
 * @example
 * joinPath('/api', '/users/', '42')  // => '/api/users/42'
 */
export function joinPath(...segments: string[]): string {
    return '/' + segments.map(s => s.replace(/^\/|\/$/g, '')).filter(Boolean).join('/');
}
