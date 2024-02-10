_default:
    @just --list --unsorted

fmt:
	cargo +nightly fmt

check:
	cargo clippy

test:
    cargo nextest run --status-level=leak
