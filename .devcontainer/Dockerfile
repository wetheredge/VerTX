FROM docker.io/espressif/idf-rust:esp32s3_1.80.0.0

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y cargo-run-bin

RUN git clone --depth 1 https://github.com/asdf-vm/asdf ~/.asdf --branch v0.14.1 \
    && echo '. "$HOME/.asdf/asdf.sh"' >> ~/.bashrc \
    && echo '. "$HOME/.asdf/completions/asdf.bash"' >> ~/.bashrc \
    && bash -ic 'asdf plugin add actionlint https://github.com/crazy-matt/asdf-actionlint' \
    && bash -ic 'asdf plugin add bun https://github.com/cometkim/asdf-bun' \
    && bash -ic 'asdf plugin add task https://github.com/particledecay/asdf-task' \
    && bash -ic 'asdf plugin add typos https://github.com/aschiavon91/asdf-typos'
