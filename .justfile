_default:
    @just --list --unsorted
    @echo
    @echo 'See DEVELOPMENT.md in the project root for details'

# Delete all build artifacts
[confirm]
clean:
    cargo clean
    cd vhs-web && rm -r dist

# Format everything
fmt:
    cargo +nightly fmt --all
    @echo
    cargo bin dprint fmt
    @echo
    cd vhs-web && pnpm biome format --write .

# Complete debug build
build:
    just vhs-web/build
    @echo
    just vhs/build

# Complete release build
build-release:
    just vhs-web/build
    @echo
    just vhs/build-release

# Flash the most recently built firmware over USB using espflash and open the serial monitor
flash:
    just vhs/flash

monitor:
    cargo bin espflash monitor

setup:
    cargo bin --install
    cargo bin --sync-aliases
    cd vhs-web && pnpm install
    @echo
    @echo "Make sure the latest esp toolchain is installed with the esp32s3 target."
    @echo "See <https://github.com/esp-rs/espup/#installation>"
    @echo
    @echo "The WIFI_SSID & WIFI_PASSWORD environment variables are required at build time."
    @echo "They will be automatically loaded from .env in the project root, if present."
