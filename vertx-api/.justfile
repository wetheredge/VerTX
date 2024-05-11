import '../.justfile.base'

_default:
    @just --list --unsorted

# Run a local dev server
dev:
    cargo run --example dev-server
