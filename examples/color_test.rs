use egui_skia::rasterize;
use skia_safe::{EncodedImageFormat};
use std::fs::File;
use std::io::Write;

pub fn main() {
    let mut demo = egui_demo_lib::ColorTest::default();

    let mut surface = rasterize(
        (800, 2000),
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                demo.ui(ui);
            });
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
