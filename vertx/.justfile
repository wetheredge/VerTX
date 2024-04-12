set dotenv-load := true

export VERTX_TARGET := "devkit"

_default:
    @just --list --unsorted

fmt:
    cargo +nightly fmt

check $VERTX_TARGET='devkit' *args='':
    cargo +esp clippy {{ args }}

build $VERTX_TARGET: && make-bin
    cargo +esp build
    cp ../target/xtensa-esp32s3-none-elf/debug/vertx ../target/vertx

build-release $VERTX_TARGET: && make-bin
    cargo +esp build --release
    cp ../target/xtensa-esp32s3-none-elf/release/vertx ../target/vertx

erase-config:
    cargo bin espflash erase-parts --partition-table partitions.csv config

flash:
    cargo bin espflash flash --partition-table partitions.csv --flash-size 16mb --baud 460800 --monitor ../target/vertx

[private]
make-bin:
    cargo bin espflash save-image --flash-size 16mb --chip esp32s3 ../target/vertx ../target/vertx.bin

monitor:
    cargo bin espflash monitor

cargo *args:
    cargo +esp {{ args }}
