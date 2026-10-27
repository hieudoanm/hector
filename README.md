# hector

Raster to vector image tracer: PNG/JPG in, SVG out. CLI binary `hector`.

## Packages

- `packages/module` — core Rust library (intended for WASM)
- `packages/cli` — CLI wrapping the module

## Build

```sh
cd packages/module && make build
cd packages/cli && make build
```

## Usage

```sh
hector input.png output.svg
```
