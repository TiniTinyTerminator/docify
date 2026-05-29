use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use docify::agent::{extract_source, ContextJson, Envelope, OutlineItem, SymbolJson};
use docify::cache::DocCache;
use docify::extract::{extract_dir, DocItem, DocSet};
use docify::render;
use docify::tui::run_doc_browser_with_hidden;

/// Multi-language doc comment extractor.
#[derive(Parser)]
#[command(name = "docify", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate Markdown documentation
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

        #[command(flatten)]
        cache: CacheInput,

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

        #[command(flatten)]
        cache: CacheInput,
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

        #[command(flatten)]
        cache: CacheInput,
    },

    /// Search symbols by name or description (JSON array)
    Search {
        /// Search query (case-insensitive substring)
        query: String,

        #[command(flatten)]
        cache: CacheInput,

        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },

    /// List all documented symbols as a compact JSON outline
    Outline {
        #[command(flatten)]
        cache: CacheInput,

        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },

    /// Browse documentation interactively
    Browse {
        #[command(flatten)]
        cache: CacheInput,

        /// Source directories to scan (default: current directory)
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },

    /// Manage precompiled docsets in the docify cache
    Cache {
        #[command(subcommand)]
        command: CacheCmd,
    },
}

#[derive(Args, Clone, Default)]
struct CacheInput {
    /// Include a named cached docset, e.g. --cache rust-std --cache cpp-std
    #[arg(long = "cache", value_name = "NAME")]
    caches: Vec<String>,

    /// Include every cached docset from the docify cache
    #[arg(long)]
    all_caches: bool,
}

#[derive(Subcommand)]
enum CacheCmd {
    /// Build a named cached docset from source directories
    Build {
        /// Cache entry name, e.g. rust-std or cpp-std
        name: String,

        /// Source directories to scan
        #[arg(value_name = "DIR")]
        dirs: Vec<PathBuf>,
    },

    /// List cached docsets
    List,

    /// Print the cache directory path
    Path,
}

fn main() {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Cmd::Browse {
        cache: CacheInput::default(),
        dirs: vec![],
    }) {
        Cmd::Gen { dirs, out, dry_run } => cmd_gen(dirs, out, dry_run),
        Cmd::Get { name, cache, dirs } => cmd_get(&name, dirs, cache),
        Cmd::Source {
            name,
            dirs,
            max_lines,
            cache,
        } => cmd_source(&name, dirs, max_lines, cache),
        Cmd::Context {
            name,
            dirs,
            max_lines,
            cache,
        } => cmd_context(&name, dirs, max_lines, cache),
        Cmd::Search { query, cache, dirs } => cmd_search(&query, dirs, cache),
        Cmd::Outline { cache, dirs } => cmd_outline(dirs, cache),
        Cmd::Browse { cache, dirs } => cmd_browse(dirs, cache),
        Cmd::Cache { command } => cmd_cache(command),
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
            let rel = item
                .file
                .strip_prefix(&source_root)
                .unwrap_or(&item.file)
                .display()
                .to_string();
            println!(
                "  [{lang}] {kind} {name} ({rel}:{line})",
                lang = item.lang.label(),
                kind = item.kind.label(),
                name = if item.name.is_empty() {
                    "(anonymous)"
                } else {
                    &item.name
                },
                line = item.line,
            );
        }
        return;
    }

    let set = DocSet {
        items: all_items,
        source_root,
    };

    if let Err(e) = render(&set, &out) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
    println!("✓ {total} items → {}", out.join("index.md").display());
}

// ── get ───────────────────────────────────────────────────────────────────────

fn cmd_get(name: &str, raw_dirs: Vec<PathBuf>, cache: CacheInput) {
    let items = collect_items_with_cache(raw_dirs, &cache);

    match find_symbol_in_items(name, &items) {
        Some(item) => {
            let json: SymbolJson = item.into();
            println!(
                "{}",
                serde_json::to_string_pretty(&Envelope::new(json)).unwrap()
            );
        }
        None => {
            eprintln!("error: symbol '{name}' not found");
            std::process::exit(1);
        }
    }
}

// ── source ────────────────────────────────────────────────────────────────────

fn cmd_source(name: &str, raw_dirs: Vec<PathBuf>, max_lines: usize, cache: CacheInput) {
    let items = collect_items_with_cache(raw_dirs, &cache);

    match find_symbol_in_items(name, &items) {
        Some(item) => {
            let src = extract_source(&item, max_lines);
            if src.is_empty() {
                eprintln!("error: could not read source for '{name}'");
                std::process::exit(1);
            }
            println!("{}:{}", item.file.display(), item.line);
            println!("{src}");
        }
        None => {
            eprintln!("error: symbol '{name}' not found");
            std::process::exit(1);
        }
    }
}

// ── context ───────────────────────────────────────────────────────────────────

fn cmd_context(name: &str, raw_dirs: Vec<PathBuf>, max_lines: usize, cache: CacheInput) {
    let items = collect_items_with_cache(raw_dirs, &cache);

    match find_symbol_in_items(name, &items) {
        Some(item) => {
            let source = extract_source(&item, max_lines);
            let ctx = ContextJson {
                source,
                doc: item.into(),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&Envelope::new(ctx)).unwrap()
            );
        }
        None => {
            eprintln!("error: symbol '{name}' not found");
            std::process::exit(1);
        }
    }
}

// ── search ────────────────────────────────────────────────────────────────────

fn cmd_search(query: &str, raw_dirs: Vec<PathBuf>, cache: CacheInput) {
    let items = collect_items_with_cache(raw_dirs, &cache);
    let lower = query.to_ascii_lowercase();

    let results: Vec<OutlineItem> = items
        .iter()
        .filter(|i| {
            i.name.to_ascii_lowercase().contains(&lower)
                || i.brief.to_ascii_lowercase().contains(&lower)
        })
        .map(OutlineItem::from)
        .collect();

    if results.is_empty() {
        eprintln!("no matches for '{query}'");
        std::process::exit(1);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&Envelope::new(results)).unwrap()
    );
}

// ── outline ───────────────────────────────────────────────────────────────────

fn cmd_outline(raw_dirs: Vec<PathBuf>, cache: CacheInput) {
    let items = collect_items_with_cache(raw_dirs, &cache);

    let outline: Vec<OutlineItem> = items.iter().map(OutlineItem::from).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&Envelope::new(outline)).unwrap()
    );
}

// ── browse ────────────────────────────────────────────────────────────────────

fn cmd_browse(raw_dirs: Vec<PathBuf>, cache: CacheInput) {
    let visible_items = collect_project_items(raw_dirs);
    let mut hidden_items = load_language_caches(&visible_items);
    hidden_items.extend(load_cached_items(&cache));
    if let Err(e) = run_doc_browser_with_hidden(visible_items, hidden_items) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

// ── cache ────────────────────────────────────────────────────────────────────

fn cmd_cache(command: CacheCmd) {
    let cache = DocCache::new();
    match command {
        CacheCmd::Build { name, dirs } => {
            if dirs.is_empty() {
                eprintln!("error: cache build requires at least one source directory");
                std::process::exit(1);
            }
            let scan_dirs = resolve_dirs(dirs);
            let set = extract_docset(&scan_dirs);
            let total = set.items.len();
            if total == 0 {
                eprintln!("warning: no documented items found");
                std::process::exit(1);
            }
            match cache.save_docset(&name, &set) {
                Ok(path) => println!("cached {total} items as '{name}' at {}", path.display()),
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
        CacheCmd::List => match cache.list_docsets() {
            Ok(entries) if entries.is_empty() => {
                println!("no cached docsets in {}", cache.root().display());
            }
            Ok(entries) => {
                for entry in entries {
                    println!(
                        "{}\t{} items\t{}",
                        entry.name,
                        entry.items,
                        entry.path.display()
                    );
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        CacheCmd::Path => println!("{}", cache.root().display()),
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

fn collect_items_with_cache(raw_dirs: Vec<PathBuf>, cache_input: &CacheInput) -> Vec<DocItem> {
    let mut items = collect_project_items(raw_dirs);
    items.extend(load_cached_items(cache_input));
    items
}

fn collect_project_items(raw_dirs: Vec<PathBuf>) -> Vec<DocItem> {
    let dirs = resolve_dirs(raw_dirs);
    let scan_dirs = docify::project::expand_scan_dirs(&dir_refs(&dirs));
    let mut items = Vec::new();
    for dir in &scan_dirs {
        items.extend(extract_dir(dir).items);
    }
    items
}

fn dir_refs(dirs: &[PathBuf]) -> Vec<&std::path::Path> {
    dirs.iter().map(|p| p.as_path()).collect()
}

fn load_cached_items(input: &CacheInput) -> Vec<DocItem> {
    if input.caches.is_empty() && !input.all_caches {
        return Vec::new();
    }
    let cache = DocCache::new();
    let names = if input.all_caches {
        match cache.list_docsets() {
            Ok(entries) => entries.into_iter().map(|entry| entry.name).collect(),
            Err(e) => {
                eprintln!("warning: could not list docify cache: {e}");
                Vec::new()
            }
        }
    } else {
        input.caches.clone()
    };

    load_cached_docsets(&cache, names, true)
}

fn load_language_caches(project_items: &[DocItem]) -> Vec<DocItem> {
    let mut names = Vec::new();
    for item in project_items {
        let name = match item.lang {
            docify::extract::DocLanguage::C => Some("c-std"),
            docify::extract::DocLanguage::Cpp => Some("cpp-std"),
            docify::extract::DocLanguage::Rust => Some("rust-std"),
            docify::extract::DocLanguage::Go => Some("go-std"),
            docify::extract::DocLanguage::Java => Some("java-std"),
            docify::extract::DocLanguage::Kotlin => Some("kotlin-std"),
            docify::extract::DocLanguage::Swift => Some("swift-std"),
            docify::extract::DocLanguage::D => Some("d-std"),
            docify::extract::DocLanguage::Ada
            | docify::extract::DocLanguage::Fortran
            | docify::extract::DocLanguage::Zig
            | docify::extract::DocLanguage::Unknown => None,
        };
        if let Some(name) = name {
            if !names.contains(&name.to_string()) {
                names.push(name.to_string());
            }
        }
    }
    load_cached_docsets(&DocCache::new(), names, false)
}

fn load_cached_docsets(cache: &DocCache, names: Vec<String>, warn_missing: bool) -> Vec<DocItem> {
    let mut items = Vec::new();
    for name in names {
        match cache.load_docset(&name) {
            Ok(cached) => items.extend(cached.set.items),
            Err(e) if warn_missing => {
                eprintln!("warning: could not load cached docset '{name}': {e}")
            }
            Err(_) => {}
        }
    }
    items
}

fn find_symbol_in_items(name: &str, items: &[DocItem]) -> Option<DocItem> {
    if let Some(item) = items.iter().find(|i| i.name == name) {
        return Some(item.clone());
    }
    let suffix = format!("::{name}");
    if let Some(item) = items.iter().find(|i| i.name.ends_with(&suffix)) {
        return Some(item.clone());
    }
    let lower = name.to_ascii_lowercase();
    items
        .iter()
        .find(|i| i.name.to_ascii_lowercase().contains(&lower))
        .cloned()
}

fn extract_docset(scan_dirs: &[PathBuf]) -> DocSet {
    let mut all_items = Vec::new();
    let source_root = common_ancestor(scan_dirs);

    for dir in scan_dirs {
        if !dir.is_dir() {
            eprintln!("warning: skipping missing directory: {}", dir.display());
            continue;
        }
        eprintln!("  Scanning {}", dir.display());
        all_items.extend(extract_dir(dir).items);
    }

    DocSet {
        items: all_items,
        source_root,
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
    components
        .first()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| dirs[0].clone())
}
