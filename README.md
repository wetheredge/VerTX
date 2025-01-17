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
- Lua scripts
  - ExpressLRS configuration will be implemented natively

[^1]: This project uses the term _handset_ to mean the hardware & software
    responsible for reading the control inputs, mixing, etc and _transmitter_
    exclusively for the ELRS transmitter module.

## Development

### Pre-requisites

- [`rustup`](https://rustup.rs/)
- [`espup`](https://github.com/esp-rs/espup#installation) with the latest Rust
  esp toolchain and the esp32s3 target
- [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall#installation)
  (optional, for faster installs)
- [`cargo-run-bin`](https://github.com/dustinblackman/cargo-run-bin#install)
- [`asdf`](https://asdf-vm.com/guide/getting-started.html)

After installing the above list:

```shell
# If using a different name, add VERTX_ESP_TOOLCHAIN=<name> to <repo root>/.env
$ espup install --toolchain-version 1.82.0.3 --targets esp32,esp32s3 --name esp-vertx

# Install asdf plugins
$ asdf plugin add actionlint https://github.com/crazy-matt/asdf-actionlint
$ asdf plugin add bun https://github.com/cometkim/asdf-bun
$ asdf plugin add task https://github.com/particledecay/asdf-task
$ asdf plugin add typos https://github.com/aschiavon91/asdf-typos

$ asdf install
$ task setup

# On Linux, remember to load the esp toolchain environment before building vertx
$ . ~/export-esp.sh
# Or, add it to .env
$ sed -E 's/^export |"//g' ~/export-esp.sh >> .env
```

#### Or, use the devcontainer

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/wetheredge/vertx)

This repo also comes with a devcontainer set up for GitHub Codespaces, VSCode,
[DevPod](https://devpod.sh), etc. Using a 4+ core machine may provide a better
experience. It _should_ be fully set up with all global tools installed and
VSCode configured, but I don't use VSCode so I may have missed something. Pull
requests welcome.

### Workflow

Most development tasks are run using [`task`](https://taskfile.dev). Run `task`
without any arguments to get a list of all the tasks available in the current
directory.

`task :foo` runs a task `foo` defined in the project root from anywhere in the
repository. When in the project root, `task vertx-foo/bar` or `task foo/bar`
runs the task `bar` defined in the directory `vertx-foo`. This can be combined
with `:` (ie `task :foo/bar`).

### Simulator

The simulator runs VerTX as a web app. Most functionality works, even the
configurator, although you will need to allow popups. To start it, run:

```shell
$ task :configurator/simulator:run

$ task :simulator/run
```

## License

The `postcard-ts` library is licensed under the [MIT license](./LICENSE-MIT).

The `vertx-crsf` library is licensed under either of

- [Apache License, Version 2.0](./LICENSE-APACHE)
- [MIT license](./LICENSE-MIT)

at your option.

All other code is licensed under the
[Mozilla Public License 2.0](./LICENSE-MPL).

[EdgeTX]: https://edgetx.org/
[Embassy]: https://embassy.dev/
[ExpressLRS]: https://www.expresslrs.org/
[OpenTX]: https://github.com/opentx/opentx
[Solid]: https://www.solidjs.com/
[`esp-hal`]: https://github.com/esp-rs/esp-hal
[smoltcp-interface]: https://github.com/smoltcp-rs/smoltcp#hosted-usage-examples
