# multi-project

A workspace containing six independent sub-projects, each using a
different package manager.  Used to test and demonstrate docify's
project-boundary detection and per-project grouping.

## Sub-projects

| Directory | Package manager | Manifest | Language |
|-----------|----------------|----------|----------|
| `web-utils/` | **npm** | `package.json` | TypeScript |
| `string-proc/` | **Cargo** | `Cargo.toml` | Rust |
| `native-math/` | **freight** | `freight.toml` | C++ |
| `analysis/` | **Hatch / pip** | `pyproject.toml` | Python |
| `http-kit/` | **Go modules** | `go.mod` | Go |
| `phputils/` | **Composer** | `composer.json` | PHP |

## Browsing

```sh
# From the docify crate root:
cargo run -- browse examples/multi-project
```

Each sub-project becomes a top-level node in the TUI tree, grouped by
package name.  Symbols are nested under their language and then their
module / class / file.
