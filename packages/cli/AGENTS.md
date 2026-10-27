# AGENTS.md — `packages/cli`

Scope: the `hector-cli` package, which builds the `hector` binary and owns every
way of driving the tracer: clap definitions, flag → config mapping, file I/O,
exit codes, the HTTP and MCP transports, debug artefacts, and the measurement
harness. The algorithms live in `packages/module`; if a change makes the tracer
smarter, it belongs there.

## Commands

Run from `packages/cli`.

```bash
cargo build                                    # tracer + SVG → PNG + all surfaces
cargo test                                     # unit + integration, no harness
cargo test --features eval                     # adds eval/baseline/golden/sweep
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

The `eval` feature is local to this crate. It gates `src/eval/`, `src/commands/`,
and `tests/eval.rs`; it no longer turns on a feature in `hector`.

## Layout

| Path                     | Holds                                                   |
| ------------------------ | ------------------------------------------------------- |
| `src/lib.rs`             | The support crate: exports the modules below            |
| `src/args.rs`            | The clap `Cli`, `Command`, and `Detail` definitions     |
| `src/options.rs`         | Flags → `TraceConfig`, `Detail` floor, `render_options` |
| `src/bin/hector.rs`      | Dispatch, I/O, exit codes                               |
| `src/debug.rs`           | `--debug-dir` per-stage PNG/SVG artefacts               |
| `src/server/`            | axum routes, request/response types, decoding           |
| `src/mcp.rs`, `src/mcp/` | Newline-delimited JSON-RPC MCP server                   |
| `src/eval/`              | Render-and-compare harness (`eval` feature)             |
| `src/commands/`          | `eval`/`baseline`/`golden`/`sweep` drivers (`eval`)     |
| `tests/end_to_end.rs`    | Trace → SVG over the library API                        |
| `tests/api.rs`           | axum routes, status codes, overrides                    |
| `tests/artifacts.rs`     | `--debug-dir` output                                    |
| `tests/render.rs`        | SVG → PNG sizing, background, error paths               |
| `tests/eval.rs`          | Measurement workflow, behind `--features eval`          |
| `tests/golden/`          | Committed images and `baseline.json`                    |
| `optimization/`          | Manual harness: ground-truth SVG vs traced SVG          |
| `scripts/install.sh`     | Release download helper                                 |
| `Dockerfile`             | Container image (**stale**, see below)                  |

Keep `main` returning `ExitCode`; a helper returning `Ok(false)` means "baseline
regression", which becomes exit 1.

## Conversion routing

The extension of the output decides the format, not a flag:

| Input        | Output           | Behaviour                                      |
| ------------ | ---------------- | ---------------------------------------------- |
| PNG/JPEG     | `.svg`           | Trace the raster and serialise                 |
| PNG/JPEG     | omitted          | Trace; print SVG to stdout (composable)        |
| PNG/JPEG     | `.png`           | Trace to SVG, then rasterize at the requested size |
| SVG          | `.png`           | Parse the document and rasterize               |
| SVG          | anything else    | Error: rendering needs a `.png` output         |

PNG never goes to stdout: without an output path, size/background flags are a
`Config` error rather than silently ignored. Extension checks are
case-insensitive (`LOGO.PNG` works).

## Flags that matter for rendering

| Flag                 | Meaning                                                          |
| -------------------- | ---------------------------------------------------------------- |
| `--width PX`         | Target width; alone it scales an SVG proportionally              |
| `--height PX`        | Target height; together with width it sets the exact grid        |
| `--background COLOR` | Opaque backdrop under transparent pixels (default: keep alpha)   |
| `--quiet`            | Suppress the stderr summary                                      |
| `--precision N`      | Decimal places in the *SVG*; unrelated to PNG size               |

`--width 0` / `--height 0` is rejected by `render_options` before allocation.
The tracing flags (`--colors`, `--detail`, …) are irrelevant for SVG input but
still parse, because clap treats them as global.

## Tests

Integration tests call the library exactly as the binary does. When adding CLI
behaviour, add the test to the file that names the surface
(`tests/render.rs` for PNG output) rather than re-testing argument parsing here;
parser tests live next to the flags in `src/args.rs`.

Temp paths use `std::env::temp_dir()` with a unique tag, matching the existing
tests.

## Known sharp edges

- **Name drift.** The crate is `hector-cli`, the binary is `hector`, but the
  clap `name`, help text, server banner, and several error messages still say
  `vectify`. User-visible strings and docs assume the old name. Prefer `hector`
  when touching them; do not rename the binary without updating
  `scripts/install.sh` and release assets.
- **`Dockerfile` is stale.** It copies `Cargo.toml` alone and expects a
  self-contained crate with `src/bin/vectify.rs`, but this package depends on
  `../module` and builds `src/bin/hector.rs`. Do not present the image as
  working until the build context and binary name are fixed.
- **Docs are stale.** `docs/CONTRIBUTING.md` points at
  `packages/app/headless/vectify/...`; this file is the current map.
- **Licence.** `README.md` historically claimed GPL-3.0 while `LICENSE` and
  `Cargo.toml` say MIT. `LICENSE` is authoritative.
- **`--min-area` floor.** `--detail balanced|bold` raises the effective
  `--min-area` (12 / 96) in `src/options.rs`. A colour present in `info` can
  vanish from the SVG because of this, not because of a tracing bug.
- **Eval gating.** Without `--features eval`, the eval subcommands are still
  parsed but return a `Config` error telling you to rebuild. That is intended;
  add new gated commands the same way (variant always present, handler under
  `#[cfg]`).
- **The library is the wasm target.** Do not add `std::fs`, threads, or a
  transport to `packages/module`. `cargo build --target wasm32-unknown-unknown`
  there must stay green.
