#[cfg(feature = "gl")]
extern crate gl;
#[cfg(feature = "sdl2")]
extern crate sdl2;

#[cfg(feature = "sdl2")]
use egui_sdl2_event::EguiSDL2State;
#[cfg(feature = "sdl2")]
use sdl2::{event::{Event, WindowEvent}, keyboard::Keycode, sys::Uint32, video::{GLProfile, Window}};
#[cfg(feature = "gl")]
use skia_safe::gpu::{gl::FramebufferInfo, {BackendRenderTarget, SurfaceOrigin}};
#[cfg(feature = "gl")]
use skia_safe::{Color, ColorType, Surface};

#[cfg(feature = "gl")]
use egui_skia::EguiSkia;
#[cfg(feature = "sdl2")]
use egui_skia::GetDpi as _;

/// This is a mix of the rust-sdl2 opengl example,
/// the skia-safe gl window example: https://github.com/rust-skia/rust-skia/blob/master/skia-safe/examples/gl-window/main.rs
/// and the egui-sdl2-event example: https://github.com/kaphula/egui-sdl2-event-example

#[cfg(feature = "sdl2")]
const WINSIZE: (u32, u32) = (1024, 720);

#[cfg(not(all(feature = "sdl2", feature = "gl")))]
fn main() {
    eprintln!("This example must be run with --features gl,sdl2")
}

#[cfg(all(feature = "sdl2", feature = "gl"))]
fn main() {
    #[cfg(target_os = "linux")]
    std::env::set_var("SDL2_VIDEODRIVER", "x11");
    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(3, 3);

    let mut window = video_subsystem
        .window("egui-skia demo", WINSIZE.0, WINSIZE.1)
        .opengl()
        .resizable()
        .allow_highdpi()
        .build()
        .unwrap();
    let dpi = window.infallible_dpi();
    window.set_size((WINSIZE.0 as f32 * dpi) as u32, (WINSIZE.1 as f32 * dpi) as u32).unwrap();

    // Unlike the other example above, nobody created a context for your window, so you need to create one.
    let _ctx = window.gl_create_context().unwrap();
    gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

    debug_assert_eq!(gl_attr.context_profile(), GLProfile::Core);
    debug_assert_eq!(gl_attr.context_version(), (3, 3));

    let mut gr_context = skia_safe::gpu::DirectContext::new_gl(None, None).unwrap();

    let fb_info = {
        let mut fboid = 0;
        unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

        FramebufferInfo {
            fboid: fboid.try_into().unwrap(),
            format: skia_safe::gpu::gl::Format::RGBA8.into(),
        }
    };

    fn create_surface(
        window: &Window,
        fb_info: &FramebufferInfo,
        gr_context: &mut skia_safe::gpu::DirectContext,
    ) -> skia_safe::Surface {
        let (width, height) = window.size();

        let backend_render_target =
            BackendRenderTarget::new_gl((width as i32, height as i32), 0, 8, *fb_info);
        Surface::from_backend_render_target(
            gr_context,
            &backend_render_target,
            SurfaceOrigin::BottomLeft,
            ColorType::RGBA8888,
            None,
            None,
        )
        .unwrap()
    }

    let mut surface = create_surface(&window, &fb_info, &mut gr_context);

    let mut egui_sdl2_state = EguiSDL2State::new(window.size().0, window.size().1, dpi);
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
                Event::Window {
                    window_id,
                    win_event:
                        WindowEvent::SizeChanged(_width, _height)
                        | WindowEvent::Resized(_width, _height),
                    ..
                } => {
                    if *window_id == window.id() {
                        surface = create_surface(&window, &fb_info, &mut gr_context);
                    }
                }
                _ => {}
            }
            egui_sdl2_state.sdl2_input_to_egui(&window, &event)
        }

        let (_duration, full_output) = egui_skia.run(egui_sdl2_state.raw_input.take(), |ctx| {
            demo_ui.ui(ctx);
        });
        egui_sdl2_state.process_output(&window, &full_output);

        let canvas = surface.canvas();
        canvas.clear(Color::BLACK);
        egui_skia.paint(canvas);
        surface.flush();
        window.gl_swap_window();

        frame_timer.time_stop();
    }
}

#[cfg(feature = "sdl2")]
pub struct FrameTimer {
    frame_time: u32,
    delta: f32,
    start: u32,
    stop: u32,
}

pub const MS_TO_SECONDS: f32 = 1.0 / 1000.0;

#[cfg(feature = "sdl2")]
impl FrameTimer {
    pub fn new() -> FrameTimer {
        FrameTimer {
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
