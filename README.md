# Skia backend for egui

This is a drawing backend for [egui](https://github.com/emilk/egui) that uses [skia-safe](https://crates.io/crates/skia-safe).

## Usage

Have a look at the metal or cpu examples to get started.

## Run the examples

```bash
cargo run --example metal --features winit,skia-safe/metal
cargo run --example cpu --features winit
```

## Status
Rendering on the gpu works great, only the dancing strings example doesn't work for some reason.
Rendering on the cpu doesn't look correct yet, I'm not sure why.

## Preview:

https://user-images.githubusercontent.com/8009393/184211263-13d1f2d5-0125-4187-98a6-e95f003e7e75.mov
