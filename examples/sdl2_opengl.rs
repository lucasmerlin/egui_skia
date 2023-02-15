use egui_sdl2_event::DpiMode;

/// This is a mix of the rust-sdl2 opengl example,
/// the skia-safe gl window example: https://github.com/rust-skia/rust-skia/blob/master/skia-safe/examples/gl-window/main.rs
/// and the egui-sdl2-event example: https://github.com/kaphula/egui-sdl2-event-example
#[cfg(feature = "gl")]
fn main() {
    extern crate gl;
    extern crate sdl2;

    use egui_sdl2_event::EguiSDL2State;
    use sdl2::event::{Event, WindowEvent};
    use sdl2::keyboard::Keycode;
    use sdl2::video::{GLProfile, Window};
    use skia_safe::gpu::gl::FramebufferInfo;
    use skia_safe::gpu::{BackendRenderTarget, SurfaceOrigin};
    use skia_safe::{Color, ColorType, Surface};

    use egui_skia::EguiSkia;

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(3, 3);

    let window = video_subsystem
        .window("Window", 800, 600)
        .opengl()
        .resizable()
        .allow_highdpi()
        .build()
        .unwrap();

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
    ) -> Surface {
        let (width, height) = window.drawable_size();

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

    let mut egui_sdl2_state = EguiSDL2State::new(&window, &video_subsystem, DpiMode::Auto);
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

        let (_duration, full_output) =
            egui_skia.run(egui_sdl2_state.take_egui_input(&window), |ctx| {
                demo_ui.ui(ctx);
            });
        egui_sdl2_state.process_output(&window, &full_output);

        let canvas = surface.canvas();
        canvas.clear(Color::BLACK);
        egui_skia.paint(canvas);
        surface.flush();
        window.gl_swap_window();
    }
}

#[cfg(not(feature = "gl"))]
fn main() {
    println!("This example requires the `gl` feature to be enabled.");
}
