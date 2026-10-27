# Contributing

## Setup

```bash
cd packages/app/headless/vectify/languages/rust
cargo build
cargo test
```

Requires a stable Rust toolchain. `Cargo.toml` declares `rust-version = "1.75"`;
CI uses a current stable.

Dependencies are pinned with `=` in `Cargo.toml`, and `Cargo.lock` is
committed. Do not bump a version without a reason in the commit message.

## Before you push

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

All three must be clean. `-D warnings` is not negotiable; if clippy is right,
fix the code rather than silencing the lint.

## Working on the tracer

The pipeline is one-way, so **change one stage at a time**:

```text
raster → preprocess → segment → contour → simplify → bezier → vector → svg
```

Every stage has a matching entry in `src/`, a unit test that pins down one
property, and a debug artefact from `--debug-dir`:

```bash
vectify logo.png out.svg --debug-dir ./trace
ls ./trace   # 01-quantized 02-regions 03-curves 04-curves.svg
```

Start from `02-regions.png`. If the shapes are wrong there, no amount of curve
tuning will help.

Rules that hold throughout:

- **Do not jump from pixels to SVG.** Answer "what are the pixels", then "what
  regions", then "what boundaries", then "which points matter", then "which
  curves", then "how to write it".
- **Keep abstractions distinct.** `Raster`, `LabelMap`, `Region`, `Contour`, and
  `VectorImage` are different things. A function that needs two of them takes
  both as arguments.
- **No new tracing dependencies.** `image` decodes pixels and writes debug PNGs.
  That is its whole job. The algorithms are the point of the project.
- **Stay deterministic.** Same input, same output, no RNG, no floating-point
  iteration order that depends on hashing.
- **Errors are explicit.** Return `Error`, do not silently drop a shape.

## Tests

Unit tests go in a `#[cfg(test)] mod tests` at the bottom of the file they
cover. Integration tests go in `tests/`.

Name tests as documentation of the contract:

```rust
#[test]
fn one_region_of_a_shared_label_yields_only_its_own_loop() { }
```

not `test_region_2`. If a test fails, the name should already tell you what
broke.

Add a test with every behaviour change. Bug fixes get a test that fails before
the fix.

## Style

From the repository conventions, which also apply here:

- Files ≤ 200 lines. Functions ≤ 30 lines. If you have to scroll to see a whole
  function, split it.
- Explicit types on public signatures.
- Prefer pure functions that take inputs and return outputs over mutation.
- Comment the _why_. Names should carry the _what_.

## Commit messages

One concern per commit. Say which stage you touched and what changed about the
output, for example:

```text
trace per region instead of per label

Two disconnected blobs sharing a palette colour each received a copy of
every boundary. trace_region now takes the region's pixel list.
```

## Adding a stage or a backend

- New colour strategy: add a `ColorMode` variant in `preprocess.rs`.
- New output format: implement a serialiser over `VectorImage`. Do not touch
  the tracer.
- New segmentation strategy: emit a `LabelMap`. Stages below are unchanged.

See `docs/ARCHITECTURE.md` for the data flow and `docs/ROADMAP.md` for what is
planned.
