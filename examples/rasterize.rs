use egui_skia::rasterize;
use skia_safe::{EncodedImageFormat, Paint, Point, Surface};
use std::fs::File;
use std::io::Write;

pub fn main() {
    let mut demo = egui_demo_lib::DemoWindows::default();

    let mut surface = rasterize(
        (1024, 756),
        |ctx| {
            demo.ui(ctx);
        },
        None,
    );

    let data = surface
        .image_snapshot()
        .encode_to_data(EncodedImageFormat::PNG)
        .expect("Failed to encode image");

    File::create("output.png")
        .unwrap()
        .write_all(&data)
        .unwrap();

    println!("wrote output.png");
}
