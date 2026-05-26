use ratatui::style::{Color, Modifier, Style};

// ── Palette ───────────────────────────────────────────────────────────────────

/// Active search / focus border colour.
pub const COLOR_SEARCH: Color = Color::Cyan;

/// Inactive panel border colour.
pub const COLOR_BORDER: Color = Color::DarkGray;

/// Panel title text colour.
pub const COLOR_TITLE: Color = Color::White;

/// Body / secondary text colour.
pub const COLOR_CONTENT: Color = Color::Gray;

/// Status-bar / hint text colour.
pub const COLOR_HINT: Color = Color::DarkGray;

/// Section-header label colour (Parameters, Returns, …).
pub const COLOR_SECTION: Color = Color::Yellow;

// ── Compound styles ───────────────────────────────────────────────────────────

/// Standard list-row highlight (navy background, white+bold text).
pub fn highlight_style() -> Style {
    Style::default()
        .bg(Color::Rgb(30, 50, 80))
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}
