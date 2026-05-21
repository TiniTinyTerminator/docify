use std::path::PathBuf;

use clap::{Parser, Subcommand};

use docify::agent::{
    all_symbols, extract_source, find_symbol, search_symbols, ContextJson, OutlineItem, SymbolJson,
};
use docify::extract::{extract_dir, DocSet};
use docify::render;
use docify::tui::run_doc_browser;

/// Multi-language doc comment extractor.
#[derive(Parser)]
#[command(name = "docify", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate Markdown documentation (default when no command given)
    Gen {
        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,

        /// Output directory for generated docs
        #[arg(short, long, value_name = "DIR", default_value = "target/doc")]
        out: PathBuf,

        /// List extracted items without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Get full documentation for a single symbol (JSON)
    Get {
        /// Symbol name — exact, qualified, or unqualified suffix (e.g. "mean", "stats::mean")
        name: String,

        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },

    /// Print the source code of a symbol
    Source {
        /// Symbol name
        name: String,

        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,

        /// Maximum number of lines to extract
        #[arg(long, default_value_t = 120)]
        max_lines: usize,
    },

    /// Get documentation + source code for a symbol in one response (JSON)
    Context {
        /// Symbol name
        name: String,

        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,

        /// Maximum lines of source to include
        #[arg(long, default_value_t = 120)]
        max_lines: usize,
    },

    /// Search symbols by name or description (JSON array)
    Search {
        /// Search query (case-insensitive substring)
        query: String,

        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },

    /// List all documented symbols as a compact JSON outline
    Outline {
        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },

    /// Browse documentation interactively
    Browse {
        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command.unwrap_or_else(|| Cmd::Gen {
        dirs: vec![],
        out: PathBuf::from("target/doc"),
        dry_run: false,
    }) {
        Cmd::Gen { dirs, out, dry_run }          => cmd_gen(dirs, out, dry_run),
        Cmd::Get { name, dirs }                  => cmd_get(&name, dirs),
        Cmd::Source { name, dirs, max_lines }    => cmd_source(&name, dirs, max_lines),
        Cmd::Context { name, dirs, max_lines }   => cmd_context(&name, dirs, max_lines),
        Cmd::Search { query, dirs }              => cmd_search(&query, dirs),
        Cmd::Outline { dirs }                    => cmd_outline(dirs),
        Cmd::Browse { dirs }                     => cmd_browse(dirs),
    }
}

// ── gen ───────────────────────────────────────────────────────────────────────

fn cmd_gen(raw_dirs: Vec<PathBuf>, out: PathBuf, dry_run: bool) {
    let scan_dirs = resolve_dirs(raw_dirs);
    let mut all_items = Vec::new();
    let source_root = common_ancestor(&scan_dirs);

    for dir in &scan_dirs {
        if !dir.is_dir() {
            eprintln!("warning: skipping missing directory: {}", dir.display());
            continue;
        }
        eprintln!("  Scanning {}", dir.display());
        all_items.extend(extract_dir(dir).items);
    }

    if all_items.is_empty() {
        eprintln!("warning: no documented items found");
        eprintln!("  Add doc comments (///, /** */, !>, --!, …) to your sources");
        std::process::exit(1);
    }

    let total = all_items.len();

    if dry_run {
        println!("{total} documented items found:");
        for item in &all_items {
            let rel = item.file
                .strip_prefix(&source_root)
                .unwrap_or(&item.file)
                .display()
                .to_string();
            println!(
                "  [{lang}] {kind} {name} ({rel}:{line})",
                lang = item.lang.label(),
                kind = item.kind.label(),
                name = if item.name.is_empty() { "(anonymous)" } else { &item.name },
                line = item.line,
            );
        }
        return;
    }

    let set = DocSet { items: all_items, source_root };

    if let Err(e) = render(&set, &out) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
    println!("✓ {total} items → {}", out.join("index.md").display());
}

// ── get ───────────────────────────────────────────────────────────────────────

fn cmd_get(name: &str, raw_dirs: Vec<PathBuf>) {
    let dirs = resolve_dirs(raw_dirs);
    let dir_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();

    match find_symbol(name, &dir_refs) {
        Some(item) => {
            let json: SymbolJson = item.into();
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        None => {
            eprintln!("error: symbol '{name}' not found");
            std::process::exit(1);
        }
    }
}

// ── source ────────────────────────────────────────────────────────────────────

fn cmd_source(name: &str, raw_dirs: Vec<PathBuf>, max_lines: usize) {
    let dirs = resolve_dirs(raw_dirs);
    let dir_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();

    match find_symbol(name, &dir_refs) {
        Some(item) => {
            let src = extract_source(&item, max_lines);
            if src.is_empty() {
                eprintln!("error: could not read source for '{name}'");
                std::process::exit(1);
            }
            println!("{src}");
        }
        None => {
            eprintln!("error: symbol '{name}' not found");
            std::process::exit(1);
        }
    }
}

// ── context ───────────────────────────────────────────────────────────────────

fn cmd_context(name: &str, raw_dirs: Vec<PathBuf>, max_lines: usize) {
    let dirs = resolve_dirs(raw_dirs);
    let dir_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();

    match find_symbol(name, &dir_refs) {
        Some(item) => {
            let source = extract_source(&item, max_lines);
            let ctx = ContextJson {
                source,
                doc: item.into(),
            };
            println!("{}", serde_json::to_string_pretty(&ctx).unwrap());
        }
        None => {
            eprintln!("error: symbol '{name}' not found");
            std::process::exit(1);
        }
    }
}

// ── search ────────────────────────────────────────────────────────────────────

fn cmd_search(query: &str, raw_dirs: Vec<PathBuf>) {
    let dirs = resolve_dirs(raw_dirs);
    let dir_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();

    let results: Vec<OutlineItem> = search_symbols(query, &dir_refs)
        .iter()
        .map(OutlineItem::from)
        .collect();

    if results.is_empty() {
        eprintln!("no matches for '{query}'");
        std::process::exit(1);
    }
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
}

// ── outline ───────────────────────────────────────────────────────────────────

fn cmd_outline(raw_dirs: Vec<PathBuf>) {
    let dirs = resolve_dirs(raw_dirs);
    let dir_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();

    let outline: Vec<OutlineItem> = all_symbols(&dir_refs)
        .iter()
        .map(OutlineItem::from)
        .collect();

    println!("{}", serde_json::to_string_pretty(&outline).unwrap());
}

// ── browse ────────────────────────────────────────────────────────────────────

fn cmd_browse(raw_dirs: Vec<PathBuf>) {
    let dirs = resolve_dirs(raw_dirs);
    let dir_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();
    if let Err(e) = run_doc_browser(&dir_refs) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn resolve_dirs(dirs: Vec<PathBuf>) -> Vec<PathBuf> {
    if dirs.is_empty() {
        vec![std::env::current_dir().expect("cannot read cwd")]
    } else {
        dirs
    }
}

fn common_ancestor(dirs: &[PathBuf]) -> PathBuf {
    if dirs.len() == 1 {
        return dirs[0].clone();
    }
    let mut components: Vec<&std::path::Path> = dirs[0].ancestors().collect();
    for dir in &dirs[1..] {
        components.retain(|&ancestor| dir.starts_with(ancestor));
    }
    components.first().map(|p| p.to_path_buf()).unwrap_or_else(|| dirs[0].clone())
}
