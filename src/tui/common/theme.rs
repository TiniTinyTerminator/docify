use ratatui::style::{Color, Style};

// ── Palette ───────────────────────────────────────────────────────────────────

/// Active search / focus border colour.
pub const COLOR_SEARCH: Color = Color::Cyan;

/// Inactive panel border colour.
pub const COLOR_BORDER: Color = Color::DarkGray;

/// Panel title text colour. Reset lets the user's terminal theme decide.
pub const COLOR_TITLE: Color = Color::Reset;

/// Body / secondary text colour. Reset lets the user's terminal theme decide.
pub const COLOR_CONTENT: Color = Color::Reset;

/// Status-bar / hint text colour.
pub const COLOR_HINT: Color = Color::DarkGray;

/// Section-header label colour (Parameters, Returns, …).
pub const COLOR_SECTION: Color = Color::Yellow;

// ── Compound styles ───────────────────────────────────────────────────────────

/// Standard list-row highlight. Selection is indicated by the moving list
/// marker, not by repainting the row background.
pub fn highlight_style() -> Style {
    Style::default()
}
