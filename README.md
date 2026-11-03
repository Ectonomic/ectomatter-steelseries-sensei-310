# SteelSeries Sensei 310 for Ectomatter

Buttons, lighting and sensitivity of the SteelSeries Sensei 310.

This is a core for [Ectomatter](https://github.com/Ectonomic/Ectomatter), one app for the devices on your desk. A core is the part of Ectomatter that talks to one device. It runs inside Ectomatter as a WebAssembly module and reaches only the USB devices its manifest (`core.json`) names.

It is independent: not made, endorsed or supported by SteelSeries.

## Adding it

In Ectomatter's Settings, under Devices, choose **Add a device core**, then pick SteelSeries Sensei 310 and press **Add**. Ectomatter shows what the core can reach before you add it, and takes the package only if it matches the checksum in Ectonomic's list of reviewed packages, [ectomatter-packages](https://github.com/Ectonomic/ectomatter-packages).

## Building it

The core is built inside Ectomatter's repository, which links this one at `cores/steelseries-sensei-310` (a Git submodule) and holds what every package is built from: the library of the core protocol's names (`crates/ectomatter-protocol`) and the interface between a module and Ectomatter (`wit/core.wit`).

```sh
git clone --recurse-submodules https://github.com/Ectonomic/Ectomatter
cd Ectomatter
rustup target add wasm32-wasip2
tools/build-module.sh cores/steelseries-sensei-310/module/Cargo.toml cores/steelseries-sensei-310/core.wasm
```

Ectonomic builds its releases with Rust 1.98.1.

- `core.json`, `glyph.svg`, `light.json`, `art/`: the core as Ectomatter installs it, its manifest and the files it carries
- `module/`: the module's source

Its commands were written from rivalcfg's profile of the mouse and libratbag's SteelSeries driver, used as references; neither is copied.

## License

Copyright (C) 2026 Ectonomic LLC. Made by Grant Garrison and Claude.

This core is free software under the GNU General Public License, version 3 only (`LICENSE`): you may share and change it under its terms. It comes with no warranty.
