# Skia backend for egui

This is a drawing backend for [egui](https://github.com/emilk/egui) that uses [skia-safe](https://crates.io/crates/skia-safe).

## Usage

Have a look at the metal or cpu examples to get started.

## Run the examples

```bash
cargo run --example metal --features winit,metal
cargo run --example cpu --features winit,cpu_fix
cargo run --example rasterize --features winit,cpu_fix

# Make sure sdl2 is installed
# Follow instructions here: https://github.com/Rust-SDL2/rust-sdl2
cargo run --example sdl2_opengl --features gl
cargo run --example sdl2_vulkan --features vulkan
```

## Status
Rendering on the gpu works great, only the dancing strings example doesn't work for some reason.

For rendering on the cpu to look correct, the cpu_fix feature needs to be enabled. See https://github.com/lucasmerlin/egui_skia/issues/1 for more information.

## Preview:

https://user-images.githubusercontent.com/8009393/184211263-13d1f2d5-0125-4187-98a6-e95f003e7e75.mov
