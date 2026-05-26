pub mod term;
pub mod theme;
pub mod widgets;

pub use term::{enter_tui, leave_tui};
pub use theme::{
    highlight_style, COLOR_BORDER, COLOR_CONTENT, COLOR_HINT, COLOR_SEARCH, COLOR_SECTION,
    COLOR_TITLE,
};
pub use widgets::{
    render_empty_panel, render_error_bar, render_hint_bar, render_search_bar, truncate, word_wrap,
};
