FROM docker.io/espressif/idf-rust:esp32s3_latest

RUN rustup toolchain install stable --component clippy && rustup default stable

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y cargo-run-bin

RUN git clone --depth 1 https://github.com/asdf-vm/asdf ~/.asdf --branch v0.14.0 \
    && echo '. "$HOME/.asdf/asdf.sh"' >> ~/.bashrc \
    && echo '. "$HOME/.asdf/completions/asdf.bash"' >> ~/.bashrc \
    && bash -ic 'asdf plugin add bun https://github.com/cometkim/asdf-bun' \
    && bash -ic 'asdf plugin add task https://github.com/particledecay/asdf-task' \
    && bash -ic 'asdf plugin add typos https://github.com/aschiavon91/asdf-typos'

RUN rustup toolchain install nightly --profile minimal --component rustfmt
