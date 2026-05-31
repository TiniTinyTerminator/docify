use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value as TomlValue;

use crate::extract::PackageId;

// ── Manifest reading ──────────────────────────────────────────────────────────

/// Walk upward from `start` looking for a recognised project manifest and
/// return the `(name, version)` pair declared in it.
///
/// Stops at the filesystem root.  The nearest (deepest) manifest wins so
/// that workspace members shadow their workspace root.
pub fn package_for_file(start: &Path) -> Option<PackageId> {
    let dir = if start.is_file() { start.parent()? } else { start };
    for ancestor in dir.ancestors() {
        if let Some(pkg) = read_manifest_in(ancestor) {
            return Some(pkg);
        }
    }
    None
}

fn read_manifest_in(dir: &Path) -> Option<PackageId> {
    // Priority order: most specific first.
    read_cargo_toml(dir)
        .or_else(|| read_freight_toml(dir))
        .or_else(|| read_package_json_name(dir))
        .or_else(|| read_pyproject_toml(dir))
        .or_else(|| read_go_mod(dir))
        .or_else(|| read_composer_json(dir))
        .or_else(|| read_gemspec(dir))
        .or_else(|| read_stack_yaml(dir))
}

fn toml_pkg_str<'a>(value: &'a TomlValue, key: &str) -> Option<&'a str> {
    value.get("package")?.get(key)?.as_str()
}

fn read_cargo_toml(dir: &Path) -> Option<PackageId> {
    let text = fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    let v = text.parse::<TomlValue>().ok()?;
    let name = toml_pkg_str(&v, "name")?.to_owned();
    let version = toml_pkg_str(&v, "version").unwrap_or("0.0.0").to_owned();
    Some(PackageId { name, version })
}

fn read_freight_toml(dir: &Path) -> Option<PackageId> {
    let text = fs::read_to_string(dir.join("freight.toml")).ok()?;
    let v = text.parse::<TomlValue>().ok()?;
    let name = toml_pkg_str(&v, "name")?.to_owned();
    let version = toml_pkg_str(&v, "version").unwrap_or("0.0.0").to_owned();
    Some(PackageId { name, version })
}

fn read_package_json_name(dir: &Path) -> Option<PackageId> {
    let text = fs::read_to_string(dir.join("package.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let name = v.get("name")?.as_str()?.to_owned();
    let version = v.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0").to_owned();
    Some(PackageId { name, version })
}

fn read_pyproject_toml(dir: &Path) -> Option<PackageId> {
    let text = fs::read_to_string(dir.join("pyproject.toml")).ok()?;
    let v = text.parse::<TomlValue>().ok()?;
    // [project] section (PEP 621)
    let name = v.get("project")?.get("name")?.as_str()?.to_owned();
    let version = v.get("project")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_owned();
    Some(PackageId { name, version })
}

fn read_go_mod(dir: &Path) -> Option<PackageId> {
    let text = fs::read_to_string(dir.join("go.mod")).ok()?;
    // `module github.com/user/repo` — use the last path segment as the name
    let module_line = text.lines().find(|l| l.trim_start().starts_with("module "))?;
    let module_path = module_line.trim_start().strip_prefix("module ")?.trim();
    let name = module_path.rsplit('/').next().unwrap_or(module_path).to_owned();
    // `go 1.22` as version
    let version = text.lines()
        .find(|l| l.trim_start().starts_with("go "))
        .and_then(|l| l.trim_start().strip_prefix("go ").map(str::trim))
        .unwrap_or("0.0.0")
        .to_owned();
    Some(PackageId { name, version })
}

fn read_composer_json(dir: &Path) -> Option<PackageId> {
    let text = fs::read_to_string(dir.join("composer.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    // composer name is "vendor/package"; use the package part
    let full_name = v.get("name")?.as_str()?;
    let name = full_name.rsplit('/').next().unwrap_or(full_name).to_owned();
    let version = v.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0").to_owned();
    Some(PackageId { name, version })
}

fn read_gemspec(dir: &Path) -> Option<PackageId> {
    // Look for *.gemspec or Gemfile
    let gemspec = fs::read_dir(dir).ok()?
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|x| x.to_str()) == Some("gemspec"))?
        .path();
    let text = fs::read_to_string(gemspec).ok()?;
    // s.name = "my-gem"
    let name = text.lines()
        .find_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("s.name").or_else(|| l.strip_prefix("spec.name"))?;
            let rest = rest.trim_start().strip_prefix('=')?.trim();
            Some(rest.trim_matches(|c| c == '"' || c == '\'').to_owned())
        })?;
    let version = text.lines()
        .find_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("s.version").or_else(|| l.strip_prefix("spec.version"))?;
            let rest = rest.trim_start().strip_prefix('=')?.trim();
            Some(rest.trim_matches(|c| c == '"' || c == '\'').to_owned())
        })
        .unwrap_or_else(|| "0.0.0".to_owned());
    Some(PackageId { name, version })
}

fn read_stack_yaml(dir: &Path) -> Option<PackageId> {
    // package.yaml (hpack) takes priority over stack.yaml
    let text = fs::read_to_string(dir.join("package.yaml"))
        .or_else(|_| fs::read_to_string(dir.join("stack.yaml")))
        .ok()?;
    let v = text.parse::<TomlValue>().ok()?;  // hpack uses YAML but basic fields parse as TOML
    // Fallback: just use directory name
    let name = v.get("name").and_then(|v| v.as_str())
        .unwrap_or_else(|| dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"))
        .to_owned();
    let version = v.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0").to_owned();
    Some(PackageId { name, version })
}

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

#[cfg(test)]
mod manifest_tests {
    use super::*;

    #[test]
    fn package_for_file_finds_cargo_toml() {
        let path = std::path::Path::new("examples/multi-project/string-proc/src/lib.rs");
        let pkg = package_for_file(path);
        assert!(pkg.is_some(), "should find Cargo.toml, got None");
        let pkg = pkg.unwrap();
        assert_eq!(pkg.name, "string-proc");
        assert_eq!(pkg.version, "0.3.1");
    }

    #[test]
    fn package_for_file_finds_package_json() {
        let path = std::path::Path::new("examples/multi-project/web-utils/src/url.ts");
        let pkg = package_for_file(path).expect("should find package.json");
        assert_eq!(pkg.name, "web-utils");
        assert_eq!(pkg.version, "1.2.0");
    }

    #[test]
    fn package_for_file_finds_freight_toml() {
        let path = std::path::Path::new("examples/multi-project/native-math/src/interp.h");
        let pkg = package_for_file(path).expect("should find freight.toml");
        assert_eq!(pkg.name, "native-math");
    }

    #[test]
    fn package_for_file_finds_pyproject_toml() {
        let path = std::path::Path::new("examples/multi-project/analysis/src/frame.py");
        let pkg = package_for_file(path).expect("should find pyproject.toml");
        assert_eq!(pkg.name, "analysis");
    }

    #[test]
    fn package_for_file_finds_go_mod() {
        let path = std::path::Path::new("examples/multi-project/http-kit/src/retry.go");
        let pkg = package_for_file(path).expect("should find go.mod");
        assert_eq!(pkg.name, "http-kit");
    }

    #[test]
    fn package_for_file_finds_composer_json() {
        let path = std::path::Path::new("examples/multi-project/phputils/src/Collection.php");
        let pkg = package_for_file(path).expect("should find composer.json");
        assert_eq!(pkg.name, "phputils");
    }

    #[test]
    fn extract_dir_stamps_package_on_items() {
        let dir = std::path::Path::new("examples/multi-project/string-proc");
        let items = crate::extract::extract_dir(dir).items;
        assert!(!items.is_empty(), "expected items from string-proc");
        for item in &items {
            let pkg = item.meta.package.as_ref()
                .unwrap_or_else(|| panic!("item '{}' has no package", item.name));
            assert_eq!(pkg.name, "string-proc");
        }
    }

    #[test]
    fn extract_dir_groups_multi_project() {
        let dir = std::path::Path::new("examples/multi-project");
        let items = crate::extract::extract_dir(dir).items;
        let pkgs: std::collections::HashSet<_> = items.iter()
            .filter_map(|i| i.meta.package.as_ref().map(|p| p.name.as_str()))
            .collect();
        for expected in ["string-proc", "web-utils", "analysis", "http-kit", "phputils"] {
            assert!(pkgs.contains(expected), "missing package '{expected}' in {pkgs:?}");
        }
    }
}
