pub mod agent;
pub mod extract;
#[cfg(feature = "clang")]
pub mod extract_clang;
pub mod markdown;
pub mod project;
pub mod render_md;
pub mod render_tui;
pub mod tui;
pub mod util;

use extract::DocSet;
use std::path::Path;

/// Render `set` into `out_dir` as GitHub-Flavored Markdown.
pub fn render(set: &DocSet, out_dir: &Path) -> std::io::Result<()> {
    render_md::render_markdown(set, out_dir)
}
