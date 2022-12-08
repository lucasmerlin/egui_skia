// This example shows how to use the renderer with SDL2 directly.

#[cfg(feature = "sdl2")]
use egui_sdl2_event::EguiSDL2State;
#[cfg(feature = "sdl2")]
use sdl2::{event::Event, keyboard::Keycode};
#[cfg(feature = "vulkan")]
use skulpin::{rafx::api::RafxExtents2D, skia_safe::{self, Color}, {LogicalSize, PhysicalSize, RendererBuilder}};
#[cfg(feature = "vulkan")]
use egui_skia::EguiSkia;
#[cfg(feature = "sdl2")]
use egui_skia::GetDpi as _;

#[cfg(not(all(feature = "sdl2", feature = "vulkan")))]
fn main() {
    eprintln!("This example must be built with --features sdl2,vulkan");
}
#[cfg(all(feature = "sdl2", feature = "vulkan"))]
fn main() {
    // Setup SDL
    let sdl_context = sdl2::init().expect("Failed to initialize sdl2");
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to create sdl video subsystem");
    let logical_size = LogicalSize { width: 900, height: 600 };

    let mut window = video_subsystem
        .window("Skulpin", logical_size.width, logical_size.height)
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .expect("Failed to create window");
    let dpi = window.infallible_dpi();
    println!("window created");

    // Set up the coordinate system to be fixed at 900x600, and use this as the default window size
    // This means the drawing code can be written as though the window is always 900x600. The
    // output will be automatically scaled so that it's always visible.
    let physical_size = PhysicalSize {
        width: (900. * dpi) as u32,
        height: (600. * dpi) as u32,
    };
    window.set_size(physical_size.width, physical_size.height);
    let scale_to_fit = skulpin::skia_safe::matrix::ScaleToFit::Center;
    let visible_range = skulpin::skia_safe::Rect {
        left: 0.0,
        right: physical_size.width as f32,
        top: 0.0,
        bottom: physical_size.height as f32,
    };

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

    let mut egui_sdl2_state =
        EguiSDL2State::new(window.drawable_size().0, window.drawable_size().1, dpi);
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

        let (_duration, full_output) = egui_skia.run(egui_sdl2_state.raw_input.take(), |ctx| {
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

        frame_timer.time_stop();
    }
}

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

    fn time_now(&self) -> sdl2::sys::Uint32 {
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
