# VerTX

A new RC handset[^1] firmware written from scratch aiming to provide a simpler
and more modern alternative to [OpenTX]/[EdgeTX]. It is written in Rust (with
TypeScript & [Solid] for the configurator) and runs on an ESP32-S3 with
[`esp-hal`] and [Embassy].

The name is derived from **Edge**TX, but simpler and smaller. Please do not
abbreviate it to VTX, that would introduce confusion with video transmitters and
this project was supposed to _reduce_ confusion… 😅

Goals:

- Easy configuration through a mobile friendly webpage served over WiFi by the
  handset itself, without needing any software other than a current web browser
- Support for multicopters & airplanes, ground vehicles, and helicopters that
  have flight controllers, in roughly that order

Non-goals:

- RC links other than [ExpressLRS]
- External transmitter modules (at least for the foreseeable future)
- Complicated custom mixing for large planes without flight controllers

[^1]: This project uses the term _handset_ to mean the hardware & software
responsible for reading the control inputs, mixing, etc and _transmitter_
exclusively for the ELRS transmitter module.

## Development

### Pre-requisites

- Latest stable Rust toolchain: <https://rustup.rs/>
- Nightly rustfmt:
  `rustup toolchain install nightly --component rustfmt --profile minimal`
- Latest Rust esp toolchain with the esp32s3 target:
  <https://github.com/esp-rs/espup#installation>
- [`just`](https://github.com/casey/just#installation)
- [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall#installation)
  (optional)
- [`cargo-run-bin`](https://github.com/dustinblackman/cargo-run-bin#install)
- Node.js and [pnpm](https://pnpm.io/installation)

#### Or, use the devcontainer

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/wetheredge/vertx)

This repo also comes with a devcontainer set up for GitHub Codespaces, VSCode,
etc. Using a 4+ core machine may provide a better experience. It _should_ be
fully set up with all global tools installed and VSCode configured, but I don't
use VSCode so I may have missed something. Pull requests welcome.

### Workflow

Once all prerequisites are installed or the devcontainer is started, run
`just setup` in the project root. This will install the remaining dev tools
inside the project directory.

Most tasks are run using `just` (ie `just fmt`, `just check`, etc). Each
`vertx*` subproject has its own set of tasks. There are a few project-wide tasks
defined in project root. Run `just` without any arguments to get a list of the
tasks available in that directory.

The configurator/web ui must to be built before building the main VerTX binary.
This can be done by running `just build` inside the `vertx-configurator/`
directory, or `just vertx-configurator/build` from the root.

## License

All code outside the `vertx-crsf` subdirectory is licensed under the
[Mozilla Public License 2.0](./LICENSE-MPL).

The `vertx-crsf` library is licensed under either of

- [Apache License, Version 2.0](./LICENSE-APACHE)
- [MIT license](./LICENSE-MIT)

at your option.

[EdgeTX]: https://edgetx.org/
[Embassy]: https://embassy.dev/
[ExpressLRS]: https://www.expresslrs.org/
[OpenTX]: https://github.com/opentx/opentx
[Solid]: https://www.solidjs.com/
[`esp-hal`]: https://github.com/esp-rs/esp-hal
