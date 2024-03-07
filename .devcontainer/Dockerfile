FROM docker.io/espressif/idf-rust:esp32s3_latest

RUN rustup toolchain install stable --component clippy && rustup default stable

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y cargo-run-bin just \
    && echo "#!/bin/sh\ncargo bin dprint \$@" > ~/.cargo/bin/dprint \
    && chmod +x ~/.cargo/bin/dprint

RUN rustup toolchain install nightly --profile minimal --component rustfmt
