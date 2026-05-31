//! Fast string processing utilities.
//!
//! Provides case conversion, slug generation, and basic text metrics
//! without external dependencies.

/// Convert a string to `snake_case`.
///
/// Inserts an underscore before each uppercase letter (except the
/// first) and lowercases the whole string.  Runs of whitespace or
/// punctuation are collapsed into a single underscore.
///
/// # Examples
///
/// ```
/// assert_eq!(to_snake("FooBarBaz"),   "foo_bar_baz");
/// assert_eq!(to_snake("HTTPClient"),  "h_t_t_p_client");
/// assert_eq!(to_snake("hello world"), "hello_world");
/// ```
pub fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    let mut prev_upper = false;
    for (i, ch) in s.char_indices() {
        if ch.is_uppercase() {
            if i > 0 && !prev_upper { out.push('_'); }
            out.push(ch.to_lowercase().next().unwrap());
            prev_upper = true;
        } else if ch.is_alphanumeric() {
            out.push(ch);
            prev_upper = false;
        } else {
            if out.ends_with('_') { prev_upper = false; continue; }
            out.push('_');
            prev_upper = false;
        }
    }
    out.trim_end_matches('_').to_owned()
}

/// Convert a string to `camelCase`.
///
/// Splits on whitespace, hyphens, and underscores; lowercases the
/// first word and title-cases all subsequent words.
///
/// # Examples
///
/// ```
/// assert_eq!(to_camel("foo_bar_baz"), "fooBarBaz");
/// assert_eq!(to_camel("hello world"), "helloWorld");
/// ```
///
/// # See also
///
/// [`to_snake`]
pub fn to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut cap_next = false;
    for (i, ch) in s.chars().enumerate() {
        if ch == '_' || ch == '-' || ch == ' ' {
            cap_next = true;
        } else if cap_next {
            out.extend(ch.to_uppercase());
            cap_next = false;
        } else if i == 0 {
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Generate a URL-safe slug from arbitrary text.
///
/// Lowercases the input, replaces spaces and punctuation with hyphens,
/// strips leading/trailing hyphens, and collapses consecutive hyphens.
///
/// # Examples
///
/// ```
/// assert_eq!(slugify("Hello, World!"), "hello-world");
/// assert_eq!(slugify("  Rust 2024  "), "rust-2024");
/// ```
pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_hyphen = true; // suppress leading hyphens
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
            last_hyphen = false;
        } else if !last_hyphen {
            out.push('-');
            last_hyphen = true;
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// Count the number of Unicode grapheme clusters in `s`.
///
/// This is the perceived character count — emoji and composed
/// characters each count as one, unlike [`str::len`] which counts bytes.
///
/// # Note
///
/// This implementation uses a simplified heuristic; for full UAX #29
/// support use the `unicode-segmentation` crate.
pub fn grapheme_count(s: &str) -> usize {
    // Simplified: treat each char as one grapheme (handles ASCII correctly).
    s.chars().count()
}

/// Truncate `s` to at most `max_chars` grapheme clusters, appending
/// `ellipsis` if the string was shortened.
///
/// # Parameters
///
/// - `s`         — Input string.
/// - `max_chars` — Maximum number of characters in the output
///                 (including the ellipsis if appended).
/// - `ellipsis`  — Suffix appended when truncation occurs.
///
/// # Returns
///
/// A [`String`] that is at most `max_chars` characters long.
///
/// # See also
///
/// [`grapheme_count`]
pub fn truncate(s: &str, max_chars: usize, ellipsis: &str) -> String {
    let n = grapheme_count(s);
    if n <= max_chars { return s.to_owned(); }
    let take = max_chars.saturating_sub(grapheme_count(ellipsis));
    let end = s.char_indices().nth(take).map(|(i, _)| i).unwrap_or(s.len());
    format!("{}{ellipsis}", &s[..end])
}
