.PHONY: lint format build test wasm build-wasm

lint:
	cargo clippy --all-targets --all-features -- -D warnings

format:
	cargo fmt

build:
	cargo build --release

test:
	cargo test
	cargo test --features raster

wasm:
	cargo build --target wasm32-unknown-unknown
	cargo build --target wasm32-unknown-unknown --features raster

build-wasm:
	cargo build --release --target wasm32-unknown-unknown
	cargo install wasm-bindgen-cli --version 0.2.130 --locked
	wasm-bindgen --target web --out-dir pkg ../target/wasm32-unknown-unknown/release/vectors.wasm
