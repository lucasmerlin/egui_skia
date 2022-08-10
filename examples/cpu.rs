use egui::Context;
use raw_window_handle::HasRawWindowHandle;
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{AlphaType, ColorSpace, ColorType, EncodedImageFormat, ImageInfo, Paint, Surface};
use std::fs::File;
use std::io::Write;

#[cfg(feature = "winit")]
fn run_software(mut ui: impl FnMut(&Context) + 'static) {
    use egui_skia::EguiSkiaWinit;
    use egui_winit::winit::dpi::LogicalSize;
    use egui_winit::winit::event::{Event, WindowEvent};
    use egui_winit::winit::event_loop::{ControlFlow, EventLoop};
    use egui_winit::winit::window::WindowBuilder;
    use softbuffer::GraphicsContext;

    let ev_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Test Render")
        .with_inner_size(LogicalSize::new(1024.0, 768.0))
        .build(&ev_loop)
        .unwrap();

    let mut gc = unsafe { GraphicsContext::new(window) }.unwrap();
    let mut egui_skia = EguiSkiaWinit::new(&ev_loop);

    egui_skia
        .egui_winit
        .set_pixels_per_point(gc.window().scale_factor() as f32);

    let size = gc.window().inner_size();
    let size = size.to_logical::<i32>(gc.window().scale_factor());
    let mut surface =
        Surface::new_raster_n32_premul((size.width as i32, size.height as i32)).unwrap();

    ev_loop.run(move |ev, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match ev {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                surface = Surface::new_raster_n32_premul((size.width as i32, size.height as i32))
                    .unwrap();
                gc.window().request_redraw();
            }
            Event::WindowEvent { event, .. } => {
                egui_skia.on_event(&event);
                gc.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                surface.canvas().clear(skia_safe::Color::TRANSPARENT);

                egui_skia.run(gc.window(), &mut ui);
                egui_skia.paint(&mut surface);

                let snapshot = surface.image_snapshot();

                let size = gc
                    .window()
                    .inner_size()
                    .to_logical::<i32>(gc.window().scale_factor());

                let mut small_surface =
                    Surface::new_raster_n32_premul((size.width, size.height)).unwrap();

                small_surface.canvas().draw_image_rect(
                    &snapshot,
                    None,
                    &skia_safe::Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
                    &Paint::default(),
                );

                let snapshot = small_surface.image_snapshot();

                let peek = snapshot.peek_pixels().unwrap();
                let pixels: &[u32] = peek.pixels().unwrap();

                gc.set_buffer(
                    pixels,
                    small_surface.width() as u16,
                    small_surface.height() as u16,
                );
            }
            _ => {}
        }
    })
}

#[cfg(feature = "winit")]
fn main() {
    let mut demos = egui_demo_lib::DemoWindows::default();
    run_software(move |ctx| {
        egui::TopBottomPanel::top("global_menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();
            })
        });

        demos.ui(ctx);
    });
}

#[cfg(not(feature = "winit"))]
pub fn main() {
    println!("This example requires the winit feature to be enabled");
}
