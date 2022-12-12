// This example shows how to use the renderer with SDL2 directly.

use egui_sdl2_event::EguiSDL2State;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use skia_safe::Color;
use skulpin::{LogicalSize, RendererBuilder};
use skulpin::rafx::api::RafxExtents2D;
use skulpin::skia_safe;

use egui_skia::EguiSkia;

fn main() {
    // Setup SDL
    let sdl_context = sdl2::init().expect("Failed to initialize sdl2");
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to create sdl video subsystem");

    // Set up the coordinate system to be fixed at 900x600, and use this as the default window size
    // This means the drawing code can be written as though the window is always 900x600. The
    // output will be automatically scaled so that it's always visible.
    let logical_size = LogicalSize {
        width: 900,
        height: 600,
    };
    let scale_to_fit = skulpin::skia_safe::matrix::ScaleToFit::Center;
    let visible_range = skulpin::skia_safe::Rect {
        left: 0.0,
        right: logical_size.width as f32,
        top: 0.0,
        bottom: logical_size.height as f32,
    };

    let window = video_subsystem
        .window("Skulpin", logical_size.width, logical_size.height)
        .position_centered()
        .allow_highdpi()
        .resizable()
        .build()
        .expect("Failed to create window");
    println!("window created");

    let (window_width, window_height) = window.vulkan_drawable_size();

    let extents = RafxExtents2D {
        width: window_width,
        height: window_height,
    };

    let renderer = RendererBuilder::new()
        .coordinate_system(skulpin::CoordinateSystem::VisibleRange(
            visible_range,
            scale_to_fit,
        ))
        .build(&window, extents);

    // Check if there were error setting up vulkan
    if let Err(e) = renderer {
        println!("Error during renderer construction: {:?}", e);
        return;
    }

    println!("renderer created");

    let mut renderer = renderer.unwrap();

    println!("Starting window event loop");
    let mut event_pump = sdl_context
        .event_pump()
        .expect("Could not create sdl event pump");

    let dpi = egui_sdl2_event::get_dpi(&window, &video_subsystem);

    let mut egui_sdl2_state =
        EguiSDL2State::new(dpi);
    let mut egui_skia = EguiSkia::new();

    let mut demo_ui = egui_demo_lib::DemoWindows::default();

    'running: loop {
        for event in event_pump.poll_iter() {
            match &event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                _ => {}
            }
            egui_sdl2_state.sdl2_input_to_egui(&window, &event)
        }

        let (_duration, full_output) = egui_skia.run(egui_sdl2_state.take_egui_input(&window), |ctx| {
            demo_ui.ui(ctx);
        });
        egui_sdl2_state.process_output(&window, &full_output);

        let (window_width, window_height) = window.vulkan_drawable_size();
        let extents = RafxExtents2D {
            width: window_width,
            height: window_height,
        };
        renderer
            .draw(extents, 1.0, |canvas, _coordinate_system_helper| {
                canvas.clear(Color::BLACK);
                egui_skia.paint(canvas);
            })
            .unwrap();

    }
}
