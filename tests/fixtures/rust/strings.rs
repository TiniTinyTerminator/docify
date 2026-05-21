/// Count the number of Unicode scalar values (not bytes) in a string.
///
/// # Examples
/// ```
/// assert_eq!(char_count("hello"), 5);
/// assert_eq!(char_count("héllo"), 5); // é is two bytes but one char
/// ```
pub fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Truncate `s` to at most `max_chars` Unicode characters.
///
/// Appends a `…` suffix when the string is shortened.
/// Returns a copy of `s` unchanged when `s.chars().count() <= max_chars`.
///
/// # Panics
/// Never panics; a `max_chars` of zero returns an empty string.
pub fn truncate(s: &str, max_chars: usize) -> String {
    todo!()
}

/// Repeat `s` exactly `n` times.
///
/// Returns an empty string when `n == 0`.
///
/// # Examples
/// ```
/// assert_eq!(repeat_str("ab", 3), "ababab");
/// assert_eq!(repeat_str("x", 0), "");
/// ```
pub fn repeat_str(s: &str, n: usize) -> String {
    todo!()
}

/// Centre-align `s` in a field of width `width`, padding with `pad`.
///
/// When `s` is already as wide or wider than `width` the string is returned
/// unchanged (no truncation is performed).
///
/// # Arguments
/// - `s`: The string to centre.
/// - `width`: Target field width in characters.
/// - `pad`: The padding character.
///
/// # Returns
/// The padded string.
pub fn centre(s: &str, width: usize, pad: char) -> String {
    todo!()
}

/// A simple string builder for constructing delimited lists.
pub struct ListBuilder {
    buf:  String,
    sep:  String,
    first: bool,
}

impl ListBuilder {
    /// Create a new builder with the given separator.
    pub fn new(sep: &str) -> Self {
        todo!()
    }

    /// Append one item to the list.
    pub fn push(&mut self, item: &str) {
        todo!()
    }

    /// Finish building and return the joined string.
    ///
    /// # Returns
    /// The items joined by the separator supplied to [`new`].
    pub fn finish(self) -> String {
        todo!()
    }
}
