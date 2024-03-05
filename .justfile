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
    cd vhs-web && just build
    @echo
    cd vhs && just build

# Complete release build
build-release:
    cd vhs-web && just build
    @echo
    cd vhs && just build-release

# Flash the most recently built firmware over USB using espflash and open the serial monitor
flash:
    just vhs/flash

monitor:
    espflash monitor
