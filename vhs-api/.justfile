_default:
    @just --list --unsorted

fmt:
	cargo +nightly fmt

check *args='':
	cargo clippy --all-targets {{ args }}

# Run a local dev server
dev:
	cargo run --example dev-server
