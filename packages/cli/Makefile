.PHONY: lint format build test

lint:
	cargo clippy --all-targets --all-features -- -D warnings

format:
	cargo fmt

build:
	cargo build --release

test:
	cargo test
