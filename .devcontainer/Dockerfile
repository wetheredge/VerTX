FROM docker.io/espressif/idf-rust:esp32s3_latest

RUN rustup toolchain install stable --component clippy && rustup default stable

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y cargo-run-bin just \
    && echo "#!/bin/sh\ncargo bin dprint \$@" > ~/.cargo/bin/dprint \
    && chmod +x ~/.cargo/bin/dprint

RUN rustup toolchain install nightly --profile minimal --component rustfmt

# Configurator build deps
USER root
RUN apt-get update \
    && apt-get install -y dirmngr gpg gawk

USER esp
RUN git clone https://github.com/asdf-vm/asdf.git $HOME/.asdf --branch v0.14.0 \
    && echo ' . "$HOME/.asdf/asdf.sh"' >> $HOME/.bashrc 
RUN bash -i -c "asdf plugin add nodejs https://github.com/asdf-vm/asdf-nodejs.git \
    && asdf install nodejs latest \
    && asdf global nodejs latest"

RUN bash -i -c "corepack enable pnpm \
    && env ENV=$HOME/.bashrc SHELL=$(which bash) corepack pnpm setup \
    && echo \"alias pnpm=\\\"corepack pnpm\\\"\" >> $HOME/.bashrc"

RUN bash -i -c "pnpm add -g just-install"

USER esp
