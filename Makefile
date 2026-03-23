.PHONY: test lint doc

test:
	cargo test --all-features

lint:
	cargo clippy -- -D warnings && cargo fmt --check

doc:
	cargo doc --no-deps --all-features --open