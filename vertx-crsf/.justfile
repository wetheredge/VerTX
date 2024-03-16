_default:
    @just --list --unsorted

fmt:
    cargo +nightly fmt

check *args='':
    cargo clippy {{ args }}

test:
    cargo bin cargo-nextest run --status-level=leak
