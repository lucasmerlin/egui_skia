use egui_skia::rasterize;
use skia_safe::{EncodedImageFormat, Paint, Point};
use std::fs::File;
use std::io::Write;

pub fn main() {
    let mut demo = egui_demo_lib::DemoWindows::default();

    let mut surface = rasterize(
        (1024, 756),
        |ctx| {
            demo.ui(ctx);

            egui::Window::new("Draw to skia").show(ctx, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag());
                    ui.painter().add(egui::PaintCallback {
                        rect,
                        callback: std::sync::Arc::new(egui_skia::EguiSkiaPaintCallback::new(
                            move |canvas| {
                                canvas.draw_circle(
                                    Point::new(150.0, 150.0),
                                    150.0,
                                    &Paint::default(),
                                );
                            },
                        )),
                    })
                });
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
