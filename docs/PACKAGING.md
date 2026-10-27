# Packaging

Vectify ships as a single static Rust binary. No runtime, no system
dependencies beyond libc, no container required.

## Binary

```bash
cargo build --release
# target/release/vectify
```

The default build has no optional features: `image` is compiled with `png` and
`jpeg` only, and `clap`/`clap_complete` are pure Rust. That keeps the binary
around 4 MB and makes cross-compilation straightforward.

```bash
$ ls -lh target/release/vectify
-rw-r--r--  1 user  staff  4.1M ... vectify
```

Strip debug symbols for a smaller artifact:

```bash
cargo build --release
strip target/release/vectify
```

## Cross-compilation

No `build.rs`, no C toolchain, no vendored native dependency, so the standard
Cargo targets work with nothing extra installed:

```bash
rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
cargo build --release --target aarch64-unknown-linux-musl
```

## Container

`Dockerfile` builds the binary on `rust:1.98.1-alpine` and ships it on
`alpine:3.22` as user `vectify` (uid 10001). The default command starts the HTTP
server, so the image is self-hosting:

```bash
docker build -t vectify .
docker run --rm -p 8080:8080 vectify

curl http://localhost:8080/health
```

The builder keeps the cache-friendly dependency layer: dependencies compile
against a stub crate, so editing `src/` does not invalidate them. The runtime
stage carries busybox `wget` for the `HEALTHCHECK` and nothing else.
`.dockerignore` keeps `target/` (about 1 GB) out of the build context.

The image also works as a one-shot tracer, since `ENTRYPOINT` is the binary:

```bash
docker run --rm -v "$PWD:/out" vectify /in.png /out/traced.svg
```

## CI Pipeline

**Not wired yet.** No workflow references this package yet. The intent is to
follow the shared template used by sibling packages in this workspace, e.g.
`.github/workflows/ci-app-headless-kevin.yaml`:

```yaml
jobs:
  ci:
    uses: ./.github/workflows/ci-app-headless-template.yaml
    with:
      rustPackage: packages/app/headless/vectify/languages/rust
      releaseName: app-headless-vectify-latest
      rustArtifactPath: packages/app/headless/vectify/languages/rust/target/release/vectify
```

Planned artifact names, following the sibling naming convention:

- `app-headless-vectify-rust-vectify-linux-amd64`
- `app-headless-vectify-rust-vectify-linux-arm64`
- `app-headless-vectify-rust-vectify-darwin-amd64`
- `app-headless-vectify-rust-vectify-darwin-arm64`

Until that workflow exists, the download URLs in [DOWNLOADS](DOWNLOADS) are not
live. Do not advertise prebuilt binaries until CI has published them.

## Dependency pinning

Every direct dependency is pinned with `=` in `Cargo.toml`, and `Cargo.lock` is
committed:

```toml
anyhow       = "=1.0.102"
axum         = { version = "=0.8.9", features = ["multipart"] }
base64       = "=0.23.1"
clap         = { version = "=4.6.7", features = ["derive"] }
clap_complete = "=4.6.1"
image        = { version = "=0.25.10", default-features = false, features = ["png", "jpeg"] }
serde        = { version = "=1.0.229", features = ["derive"] }
serde_json   = "=1.0.151"
thiserror    = "=2.0.18"
tokio        = { version = "=1.53.1", features = ["rt-multi-thread", "macros", "net", "signal"] }
tower-http   = { version = "=0.7.1", features = ["cors", "limit", "trace"] }
```

Transitive versions live in `Cargo.lock`. Bump through `cargo update -p <crate>`
and keep the lockfile in the same commit.

The `axum`/`tokio`/`tower-http` set exists only for the `serve` subcommand; the
tracer itself stays free of any async runtime.

## Release checklist

1. `cargo fmt --all -- --check` clean
2. `cargo clippy --all-targets --all-features -- -D warnings` clean
3. `cargo test --all-targets` green (89 unit + 31 integration)
4. `cargo build --release` succeeds
5. Smoke test the real binary on the bundled example:

   ```bash
   ./target/release/vectify ../../examples/images/vietinbank.png /tmp/out.svg
   grep -q '<svg' /tmp/out.svg
   ```

6. `cargo package --list` shows only files meant to ship; `target/`,
   `Cargo.lock`-adjacent scratch files, and `examples/` output must not leak in
