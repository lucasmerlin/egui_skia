#[cfg(feature = "winit")]
use egui::Context;

#[cfg(feature = "winit")]
fn run_software(mut ui: impl FnMut(&Context) + 'static) {
    use skia_safe::{Paint, Surface};

    use egui_skia::EguiSkiaWinit;
    use egui_winit::winit::dpi::LogicalSize;
    use egui_winit::winit::event::{Event, WindowEvent};
    use egui_winit::winit::event_loop::{ControlFlow, EventLoop};
    use egui_winit::winit::window::WindowBuilder;

    let ev_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Test Render")
        .with_inner_size(LogicalSize::new(1024.0, 768.0))
        .build(&ev_loop)
        .unwrap();

    let mut gc = unsafe { softbuffer::Context::new(&window) }.unwrap();
    let mut softbuffer_surface = unsafe { softbuffer::Surface::new(&gc, &window).unwrap() };
    let mut egui_skia = EguiSkiaWinit::new(&ev_loop);

    egui_skia
        .egui_winit
        .set_pixels_per_point(window.scale_factor() as f32);

    let size = window.inner_size();
    let size = size.to_logical::<i32>(window.scale_factor());
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
                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => {
                let response = egui_skia.on_event(&event);
                if response.repaint {
                    window.request_redraw();
                }
            }
            Event::RedrawRequested(_) => {
                let canvas = surface.canvas();
                canvas.clear(skia_safe::Color::TRANSPARENT);

                let repaint_after = egui_skia.run(&window, &mut ui);

                *control_flow = if repaint_after.is_zero() {
                    window.request_redraw();
                    ControlFlow::Poll
                } else if let Some(repaint_after_instant) =
                    std::time::Instant::now().checked_add(repaint_after)
                {
                    ControlFlow::WaitUntil(repaint_after_instant)
                } else {
                    ControlFlow::Wait
                };

                egui_skia.paint(canvas);

                let snapshot = surface.image_snapshot();

                let peek = snapshot.peek_pixels().unwrap();
                let pixels: &[u32] = peek.pixels().unwrap();

                // No idea why R and B have to be swapped
                let transformed = pixels
                    .iter()
                    .map(|x| {
                        (x & 0xFF000000)
                            | ((x & 0x00FF0000) >> 16)
                            | (x & 0x0000FF00)
                            | ((x & 0x000000FF) << 16)
                    })
                    .collect::<Vec<u32>>();

                softbuffer_surface.set_buffer(
                    &transformed,
                    surface.width() as u16,
                    surface.height() as u16,
                );
            }
            _ => {}
        }
    })
}

#[cfg(feature = "winit")]
fn main() {
    use std::sync::Arc;

    use egui::ScrollArea;
    use skia_safe::{Paint, Point};

    use egui_skia::EguiSkiaPaintCallback;
    #[cfg(not(feature = "cpu_fix"))]
    eprintln!("Warning! Feature cpu_fix should be enabled when using raster surfaces. See https://github.com/lucasmerlin/egui_skia/issues/1");

    let mut demos = egui_demo_lib::DemoWindows::default();
    run_software(move |ctx| {
        egui::TopBottomPanel::top("global_menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();
            })
        });

        demos.ui(ctx);
        egui::Window::new("Draw to skia").show(ctx, |ui| {
            ScrollArea::horizontal().show(ui, |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag());
                ui.painter().add(egui::PaintCallback {
                    rect: rect.clone(),
                    callback: Arc::new(EguiSkiaPaintCallback::new(move |canvas| {
                        canvas.draw_circle(Point::new(150.0, 150.0), 150.0, &Paint::default());
                    })),
                })
            });
        });
    });
}

#[cfg(not(feature = "winit"))]
pub fn main() {
    println!("This example requires the winit feature to be enabled");
}
