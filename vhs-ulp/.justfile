set dotenv-load := true

_default:
    @just --list --unsorted

fmt:
    cargo +nightly fmt

check *args='':
    cargo +esp clippy {{ args }}

build:
    cargo +esp build --release
    cp ../target/riscv32imc-unknown-none-elf/release/vhs-ulp ../target

disassemble:
    riscv64-linux-gnu-objdump -d -l ../target/vhs-ulp
