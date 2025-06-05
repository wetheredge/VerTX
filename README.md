# VerTX

> [!important]
>
> This is still in **very** early pre-alpha development and is missing most core
> functionality. It's not yet ready for external contributions; I just got tired
> of hitting the CI limits in private repos. I'll update this note as it gets
> closer to being ready!

A new RC handset[^1] firmware written from scratch aiming to provide a simpler
and more modern alternative to [OpenTX]/[EdgeTX]. It is written in Rust (with
TypeScript, [Astro], and some React for the configurator) and runs on an
ESP32-S3 or RP2040 using [Embassy]. STM32 support [is in progress][stm32].

The name is derived from **Edge**TX, but simpler and smaller. Please do not
abbreviate it to VTX, that would introduce confusion with video transmitters and
this project was supposed to _reduce_ confusion… 😅

Goals:

- Easy configuration through a mobile friendly webpage served over WiFi by the
  handset itself, without needing any software other than a web browser
  - The mixer will be configured through a node interface inspired by Blender
    that will hopefully make complex setups easier to understand and modify
- Support for multicopters & airplanes, ground vehicles, and helicopters that
  have flight controllers, in roughly that order

Non-goals:

- RC links other than [ExpressLRS]
- External transmitter modules (at least for the foreseeable future)
- Lua scripts (probably)
  - ExpressLRS configuration will be implemented natively

[^1]: This project uses the term _handset_ to mean the hardware & software
    responsible for reading the control inputs, mixing, etc and _transmitter_
    exclusively for the ELRS transmitter module.

## Development

### Pre-requisites

- [`rustup`](https://rustup.rs/)
- [`mise`](https://mise.jdx.dev/installing-mise.html)
- Optionally
  [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall#installation)
  for faster installs

After installing the above list:

```shell
$ mise install
$ task setup
```

### Workflow

Most development tasks are run using [`task`](https://taskfile.dev). Run `task`
without any arguments to get a list of all the tasks available in the current
directory.

`task :foo` runs a task `foo` defined in the project root from anywhere in the
repository. When in the project root, `task foo/bar` runs the task `bar` defined
in the directory `vertx-foo`. This can be combined with `:` (ie
`task :foo/bar`).

### Simulator

The simulator runs VerTX as a web app. Most functionality works, even the
configurator, although you will need to allow popups. It's deployed at
<https://simulator.vertx.cc/>. To start it locally, run:

```shell
$ task :configurator/run

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

[Astro]: https://astro.build/
[EdgeTX]: https://edgetx.org/
[Embassy]: https://embassy.dev/
[ExpressLRS]: https://www.expresslrs.org/
[OpenTX]: https://github.com/opentx/opentx
[stm32]: https://github.com/wetheredge/VerTX/pull/102
