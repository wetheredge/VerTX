set dotenv-load := true

_default:
    @just --list --unsorted

fmt:
    cargo +nightly fmt

check *args='':
    cargo +esp clippy {{ args }}

build:
    cargo +esp build
    cp ../target/xtensa-esp32s3-none-elf/debug/vhs ../target/vhs

build-release:
    cargo +esp build --release
    cp ../target/xtensa-esp32s3-none-elf/release/vhs ../target/vhs

erase-config:
    cargo bin espflash erase-parts --partition-table partitions.csv config

flash:
    cargo bin espflash flash --partition-table partitions.csv --flash-size 16mb --baud 460800 --monitor ../target/vhs

monitor:
    cargo bin espflash monitor

cargo *args:
    cargo +esp {{ args }}
