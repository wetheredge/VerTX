set dotenv-load
set dotenv-path := "../.env"

_default:
	@just --list --unsorted

fmt:
	cargo +nightly fmt

check *args='':
	cargo clippy {{ args }}

build: && (_last-build 'debug')
	cargo build

build-release: && (_last-build 'release')
	cargo build --release

flash:
    espflash flash --monitor {{`cat ../target/last_build`}}

_last-build name:
    @echo "../target/xtensa-esp32s3-none-elf/{{ name }}/vhs" > target/last_build
