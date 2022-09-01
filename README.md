# Skia backend for egui

This is a drawing backend for [egui](https://github.com/emilk/egui) that uses [skia-safe](https://crates.io/crates/skia-safe).

## Usage

Have a look at the metal or cpu examples to get started.

## Run the examples

```bash
cargo run --example metal --features winit,skia-safe/metal
cargo run --example cpu --features winit,cpu_fix
```

## Status
Rendering on the gpu works great, only the dancing strings example doesn't work for some reason.

For rendering on the cpu to look correct, the cpu_fix feature needs to be enabled. See https://github.com/lucasmerlin/egui_skia/issues/1 for more information.

## Preview:

https://user-images.githubusercontent.com/8009393/184211263-13d1f2d5-0125-4187-98a6-e95f003e7e75.mov
