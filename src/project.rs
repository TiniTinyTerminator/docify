use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value as TomlValue;

/// Expand scan roots with locally available dependency source directories.
///
/// This is intentionally offline-only: project files can point us at Cargo,
/// freight, npm, CMake, Go, or Python dependencies, but missing packages are
/// ignored rather than fetched.
pub fn expand_scan_dirs(roots: &[&Path]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for root in roots {
        push_existing_dir(&mut out, &mut seen, root);
        let Some(project_root) = find_project_root(root) else {
            continue;
        };
        discover_from_project(&project_root, &mut out, &mut seen);
    }

    out
}

fn find_project_root(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    start
        .ancestors()
        .find_map(|candidate| has_project_manifest(candidate).then(|| candidate.to_path_buf()))
}

fn has_project_manifest(dir: &Path) -> bool {
    [
        "Cargo.toml",
        "freight.toml",
        "package.json",
        "CMakeLists.txt",
        "go.mod",
        "pyproject.toml",
        "requirements.txt",
    ]
    .iter()
    .any(|name| dir.join(name).is_file())
}

fn discover_from_project(root: &Path, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    discover_toml_paths(&root.join("Cargo.toml"), root, out, seen);
    discover_toml_paths(&root.join("freight.toml"), root, out, seen);
    discover_toml_paths(&root.join("pyproject.toml"), root, out, seen);
    discover_package_json(root, out, seen);
    discover_cmake(root, out, seen);
    discover_go_mod(root, out, seen);
    discover_requirements(root, out, seen);
}

fn discover_toml_paths(
    manifest: &Path,
    base: &Path,
    out: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) {
    let Ok(text) = fs::read_to_string(manifest) else {
        return;
    };
    let Ok(value) = text.parse::<TomlValue>() else {
        return;
    };

    collect_toml_path_values(&value, base, out, seen);
    if manifest.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
        collect_cargo_workspace_members(&value, base, out, seen);
    }
}

fn collect_toml_path_values(
    value: &TomlValue,
    base: &Path,
    out: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) {
    match value {
        TomlValue::Table(table) => {
            if let Some(path) = table.get("path").and_then(TomlValue::as_str) {
                push_existing_dir(out, seen, &base.join(path));
            }
            if let Some(path) = table.get("local").and_then(TomlValue::as_str) {
                push_existing_dir(out, seen, &base.join(path));
            }
            for child in table.values() {
                collect_toml_path_values(child, base, out, seen);
            }
        }
        TomlValue::Array(items) => {
            for child in items {
                collect_toml_path_values(child, base, out, seen);
            }
        }
        _ => {}
    }
}

fn collect_cargo_workspace_members(
    value: &TomlValue,
    base: &Path,
    out: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) {
    let Some(members) = value
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(TomlValue::as_array)
    else {
        return;
    };

    for member in members.iter().filter_map(TomlValue::as_str) {
        if let Some(prefix) = member.strip_suffix("/*") {
            push_immediate_child_dirs(out, seen, &base.join(prefix));
        } else {
            push_existing_dir(out, seen, &base.join(member));
        }
    }
}

fn discover_package_json(root: &Path, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    let Ok(text) = fs::read_to_string(root.join("package.json")) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };

    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        let Some(deps) = value.get(section).and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (name, spec) in deps {
            if let Some(path) = spec.as_str().and_then(|spec| spec.strip_prefix("file:")) {
                push_existing_dir(out, seen, &root.join(path));
            } else {
                push_existing_dir(out, seen, &root.join("node_modules").join(name));
            }
        }
    }

    if let Some(workspaces) = value.get("workspaces") {
        collect_npm_workspaces(workspaces, root, out, seen);
    }
}

fn collect_npm_workspaces(
    value: &serde_json::Value,
    root: &Path,
    out: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) {
    let patterns: Vec<&str> = if let Some(items) = value.as_array() {
        items.iter().filter_map(serde_json::Value::as_str).collect()
    } else {
        value
            .get("packages")
            .and_then(serde_json::Value::as_array)
            .map(|items| items.iter().filter_map(serde_json::Value::as_str).collect())
            .unwrap_or_default()
    };

    for pattern in patterns {
        if let Some(prefix) = pattern.strip_suffix("/*") {
            push_immediate_child_dirs(out, seen, &root.join(prefix));
        } else {
            push_existing_dir(out, seen, &root.join(pattern));
        }
    }
}

fn discover_cmake(root: &Path, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    let Ok(text) = fs::read_to_string(root.join("CMakeLists.txt")) else {
        return;
    };

    for line in text.lines().map(str::trim) {
        if let Some(name) = cmake_call_name(line, "find_package") {
            push_cmake_package_candidates(root, name, out, seen);
        }
        if let Some(name) = cmake_call_name(line, "FetchContent_Declare") {
            push_cmake_package_candidates(root, name, out, seen);
        }
        if let Some(path) = cmake_source_dir(line) {
            push_existing_dir(out, seen, &root.join(path));
        }
        if let Some(path) = cmake_add_subdirectory(line) {
            push_existing_dir(out, seen, &root.join(path));
        }
    }
}

fn cmake_call_name<'a>(line: &'a str, call: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(call)?.trim_start();
    let rest = rest.strip_prefix('(')?.trim_start();
    rest.split(|ch: char| ch.is_whitespace() || ch == ')')
        .next()
        .filter(|name| !name.is_empty())
}

fn cmake_source_dir(line: &str) -> Option<&str> {
    let (_, rest) = line.split_once("SOURCE_DIR")?;
    rest.split(|ch: char| ch.is_whitespace() || ch == ')')
        .find(|part| !part.is_empty())
        .map(|part| part.trim_matches('"'))
}

fn cmake_add_subdirectory(line: &str) -> Option<&str> {
    cmake_call_name(line, "add_subdirectory")
}

fn push_cmake_package_candidates(
    root: &Path,
    name: &str,
    out: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) {
    let lower = name.to_ascii_lowercase();
    for prefix in ["vendor", "third_party", "external", "deps", "subprojects"] {
        for candidate in [name, lower.as_str()] {
            push_existing_dir(out, seen, &root.join(prefix).join(candidate));
        }
    }
}

fn discover_go_mod(root: &Path, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    let Ok(text) = fs::read_to_string(root.join("go.mod")) else {
        return;
    };
    for line in text.lines().map(str::trim) {
        let Some((_, replacement)) = line.split_once("=>") else {
            continue;
        };
        let path = replacement.split_whitespace().next().unwrap_or("");
        if path.starts_with('.') {
            push_existing_dir(out, seen, &root.join(path));
        }
    }
}

fn discover_requirements(root: &Path, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    let Ok(text) = fs::read_to_string(root.join("requirements.txt")) else {
        return;
    };
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let path = line
            .strip_prefix("-e ")
            .or_else(|| line.strip_prefix("--editable "))
            .unwrap_or(line);
        if path.starts_with('.') || path.starts_with('/') {
            push_existing_dir(out, seen, &root.join(path));
        }
    }
}

fn push_immediate_child_dirs(out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        push_existing_dir(out, seen, &entry.path());
    }
}

fn push_existing_dir(out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, path: &Path) {
    if !path.is_dir() {
        return;
    }
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if seen.insert(canonical.clone()) {
        out.push(canonical);
    }
}

#[cfg(test)]
mod tests {
    use super::expand_scan_dirs;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cargo_path_dependencies_and_workspace_members_are_added() {
        let root = temp_root("cargo");
        fs::create_dir_all(root.join("app")).unwrap();
        fs::create_dir_all(root.join("vendor/mathlib")).unwrap();
        fs::create_dir_all(root.join("crates/helper")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"
                [workspace]
                members = ["crates/*"]

                [dependencies]
                mathlib = { path = "vendor/mathlib" }
            "#,
        )
        .unwrap();

        let dirs = expand_scan_dirs(&[root.join("app").as_path()]);

        assert_contains(&dirs, &root.join("vendor/mathlib"));
        assert_contains(&dirs, &root.join("crates/helper"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn npm_dependencies_only_include_installed_or_file_packages() {
        let root = temp_root("npm");
        fs::create_dir_all(root.join("node_modules/left-pad")).unwrap();
        fs::create_dir_all(root.join("packages/local-widget")).unwrap();
        fs::write(
            root.join("package.json"),
            r#"
                {
                  "dependencies": {
                    "left-pad": "^1.0.0",
                    "missing-lib": "^2.0.0",
                    "local-widget": "file:packages/local-widget"
                  }
                }
            "#,
        )
        .unwrap();

        let dirs = expand_scan_dirs(&[root.as_path()]);

        assert_contains(&dirs, &root.join("node_modules/left-pad"));
        assert_contains(&dirs, &root.join("packages/local-widget"));
        assert!(!contains_path(
            &dirs,
            &root.join("node_modules/missing-lib")
        ));
        fs::remove_dir_all(root).unwrap();
    }

    fn temp_root(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "docify-project-{label}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn assert_contains(dirs: &[PathBuf], path: &Path) {
        assert!(
            contains_path(dirs, path),
            "expected {} in {dirs:?}",
            path.display()
        );
    }

    fn contains_path(dirs: &[PathBuf], path: &Path) -> bool {
        let Ok(path) = path.canonicalize() else {
            return false;
        };
        dirs.iter().any(|dir| dir == &path)
    }
}
