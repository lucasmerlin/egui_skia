// This example shows how to use the renderer with SDL2 directly.

use egui_sdl2_event::EguiSDL2State;
use skulpin::skia_safe;
use skulpin::{CoordinateSystemHelper, RendererBuilder, LogicalSize};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use skulpin::rafx::api::RafxExtents2D;

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


    let dpi_scale = video_subsystem.display_dpi(window.display_index().unwrap()).unwrap().0;
    let dpi_scale = dpi_scale / 72.0;

    println!("DPI: {}", dpi_scale);

    let mut egui_sdl2_state = EguiSDL2State::new(window.drawable_size().0, window.drawable_size().1, 1.0);
    let mut egui_skia = EguiSkia::new();

    let mut demo_ui = egui_demo_lib::DemoWindows::default();


    let mut frame_timer = FrameTimer::new();
    let mut running_time: f64 = 0.0;


    'running: loop {

        frame_timer.time_start();
        let delta = frame_timer.delta();
        running_time += delta as f64;

        egui_sdl2_state.update_time(Some(running_time), delta);


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

        let (duration, full_output) = egui_skia.run(egui_sdl2_state.raw_input.take(), |ctx| {
            demo_ui.ui(ctx);
        });
        egui_sdl2_state.process_output(&window, &full_output);

        let (window_width, window_height) = window.vulkan_drawable_size();
        let extents = RafxExtents2D {
            width: window_width,
            height: window_height,
        };
        renderer
            .draw(extents, 1.0, |canvas, coordinate_system_helper| {
                canvas.clear(Color::BLACK);
                egui_skia.paint(canvas);
            })
            .unwrap();

        frame_timer.time_stop();
    }
}


use sdl2::sys::Uint32;
use skia_safe::Color;
use egui_skia::EguiSkia;

pub struct FrameTimer {
    last_time: u32,
    frame_time: u32,
    delta: f32,
    start: u32,
    stop: u32,
}

pub const MS_TO_SECONDS: f32 = 1.0 / 1000.0;
impl FrameTimer {
    pub fn new() -> FrameTimer {
        FrameTimer {
            last_time: 0,
            frame_time: 0,
            delta: 0.0,
            start: 0,
            stop: 0,
        }
    }

    fn time_now(&self) -> Uint32 {
        #[allow(unsafe_code)]
        unsafe {
            sdl2::sys::SDL_GetTicks()
        }
    }

    pub fn time_start(&mut self) {
        self.frame_time = self.stop - self.start;
        self.delta = self.frame_time as f32 * MS_TO_SECONDS;
        self.start = self.time_now();
    }
    pub fn time_stop(&mut self) {
        self.stop = self.time_now();
    }

    pub fn delta(&self) -> f32 {
        self.delta
    }
}
