use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::extract::DocSet;

pub const CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDocSet {
    pub schema_version: u32,
    pub name: String,
    pub generated_at_unix: u64,
    pub set: DocSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedDocSetSummary {
    pub name: String,
    pub path: PathBuf,
    pub items: usize,
    pub generated_at_unix: u64,
}

#[derive(Debug, Clone)]
pub struct DocCache {
    root: PathBuf,
}

impl DocCache {
    pub fn default_root() -> PathBuf {
        if let Some(path) = std::env::var_os("DOCIFY_HOME") {
            return PathBuf::from(path);
        }
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".docify"))
            .unwrap_or_else(|| PathBuf::from(".docify"))
    }

    pub fn new() -> Self {
        Self::at(Self::default_root())
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn save_docset(&self, name: &str, set: &DocSet) -> io::Result<PathBuf> {
        fs::create_dir_all(self.docsets_dir())?;
        let cached = CachedDocSet {
            schema_version: CACHE_SCHEMA_VERSION,
            name: name.to_string(),
            generated_at_unix: now_unix(),
            set: set.clone(),
        };
        let path = self.docset_path(name);
        let json = serde_json::to_vec_pretty(&cached).map_err(io::Error::other)?;
        fs::write(&path, json)?;
        Ok(path)
    }

    pub fn load_docset(&self, name: &str) -> io::Result<CachedDocSet> {
        let path = self.docset_path(name);
        let json = fs::read(path)?;
        let cached: CachedDocSet = serde_json::from_slice(&json).map_err(io::Error::other)?;
        if cached.schema_version != CACHE_SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported docify cache schema {} for '{}'",
                    cached.schema_version, cached.name
                ),
            ));
        }
        Ok(cached)
    }

    pub fn list_docsets(&self) -> io::Result<Vec<CachedDocSetSummary>> {
        let mut out = Vec::new();
        let dir = self.docsets_dir();
        let Ok(entries) = fs::read_dir(&dir) else {
            return Ok(out);
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Ok(json) = fs::read(&path) else { continue };
            let Ok(cached) = serde_json::from_slice::<CachedDocSet>(&json) else {
                continue;
            };
            if cached.schema_version != CACHE_SCHEMA_VERSION {
                continue;
            }
            out.push(CachedDocSetSummary {
                name: cached.name,
                path,
                items: cached.set.items.len(),
                generated_at_unix: cached.generated_at_unix,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn docsets_dir(&self) -> PathBuf {
        self.root.join("docsets")
    }

    fn docset_path(&self, name: &str) -> PathBuf {
        self.docsets_dir().join(format!("{}.json", cache_key(name)))
    }
}

impl Default for DocCache {
    fn default() -> Self {
        Self::new()
    }
}

fn cache_key(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "docset".to_string()
    } else {
        out
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::{DocItem, DocKind, DocLanguage, DocMeta};

    #[test]
    fn cache_round_trips_docset() {
        let root = temp_root("roundtrip");
        let cache = DocCache::at(&root);
        let set = DocSet {
            items: vec![DocItem {
                name: "Vec".into(),
                kind: DocKind::Struct,
                brief: "Growable vector.".into(),
                body: String::new(),
                tags: vec![],
                file: PathBuf::from("library/alloc/src/vec/mod.rs"),
                line: 1,
                lang: DocLanguage::Rust,
                signature: "pub struct Vec<T>".into(),
                meta: DocMeta::default(),
            }],
            source_root: PathBuf::from("library"),
        };

        let path = cache.save_docset("rust/std", &set).unwrap();
        assert!(path.ends_with("rust_std.json"));

        let loaded = cache.load_docset("rust/std").unwrap();
        assert_eq!(loaded.name, "rust/std");
        assert_eq!(loaded.set.items[0].name, "Vec");

        let listed = cache.list_docsets().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "rust/std");

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_root(label: &str) -> PathBuf {
        let stamp = now_unix();
        let root = std::env::temp_dir().join(format!(
            "docify-cache-{label}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
