_default:
    @just --list --unsorted

fmt:
    cargo +nightly fmt

check *args='':
    cargo clippy {{ args }}

test:
    cargo nextest run --status-level=leak
