_default:
    @just --list --unsorted

# Delete all build artifacts
[confirm]
clean:
    cargo clean
    cd vertx-configurator && rm -r dist

# Format everything
fmt:
    cargo +nightly fmt --all
    @echo
    cargo bin dprint fmt
    @echo
    cd vertx-configurator && pnpm biome format --write .

# Check all subprojects
check:
    just vertx/check
    just vertx-api/check
    just vertx-crsf/check
    just vertx-configurator/check

setup:
    cargo bin --install
    cargo bin --sync-aliases
    cd vertx-configurator && pnpm install
    @echo
    @echo "The WIFI_SSID & WIFI_PASSWORD environment variables are required at build time."
    @echo "They will be automatically loaded from .env in the project root, if present."
