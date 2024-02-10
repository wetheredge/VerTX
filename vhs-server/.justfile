_default:
    @just --list --unsorted

fmt:
	cargo +nightly fmt

check:
	cargo clippy

# Run a local dev server. vhs-web does need to have been built, even if using its dev server
dev:
	cargo run --bin dev-server
