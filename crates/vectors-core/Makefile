.PHONY: lint format build test wasm

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
