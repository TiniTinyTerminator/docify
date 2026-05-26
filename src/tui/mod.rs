//! Terminal UI for `docify browse`.
//!
//! Structure:
//!   - `browser`  — interactive symbol browser (search, select, view doc/source)
//!   - `common`   — shared terminal helpers, theme constants, and text utilities

pub mod browser;
pub mod common;

pub use browser::run_doc_browser;
