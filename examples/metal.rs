use egui_skia::{EguiSkia, EguiSkiaPaintCallback, EguiSkiaWinit};
use metal::{Device, MTLPixelFormat, MetalLayer};
use skia_safe::{scalar, Canvas, Color4f, ColorSpace, ColorType, Paint, Point, Rect, Size, Surface, ConditionallySend, PictureRecorder, Font, Color, PathEffect};
use std::sync::Arc;
use egui::ScrollArea;

#[cfg(feature = "winit")]
fn main() {
    use cocoa::{appkit::NSView, base::id as cocoa_id};

    use core_graphics_types::geometry::CGSize;

    use foreign_types_shared::{ForeignType, ForeignTypeRef};
    use metal::{Device, MTLPixelFormat, MetalLayer};
    use objc::{rc::autoreleasepool, runtime::YES};

    use egui_winit::winit::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        platform::macos::WindowExtMacOS,
        window::WindowBuilder,
    };
    use skia_safe::gpu::{mtl, BackendRenderTarget, DirectContext, SurfaceOrigin};

    let size = LogicalSize::new(800, 600);

    let events_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_inner_size(size)
        .with_title("Egui Skia Metal Winit Example".to_string())
        .build(&events_loop)
        .unwrap();

    let device = Device::system_default().expect("no device found");

    let metal_layer = {
        let draw_size = window.inner_size();
        let layer = MetalLayer::new();
        layer.set_device(&device);
        layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);

        unsafe {
            let view = window.ns_view() as cocoa_id;
            view.setWantsLayer(YES);
            view.setLayer(layer.as_ref() as *const _ as _);
        }
        layer.set_drawable_size(CGSize::new(draw_size.width as f64, draw_size.height as f64));
        layer
    };

    let command_queue = device.new_command_queue();

    let backend = unsafe {
        mtl::BackendContext::new(
            device.as_ptr() as mtl::Handle,
            command_queue.as_ptr() as mtl::Handle,
            std::ptr::null(),
        )
    };

    let mut context = DirectContext::new_metal(&backend, None).unwrap();

    let mut gui = EguiSkiaWinit::new(&events_loop);

    gui.egui_winit
        .set_pixels_per_point(window.scale_factor() as f32);

    let mut demo = egui_demo_lib::DemoWindows::default();

    let mut frame = 0;

    events_loop.run(move |event, _, control_flow| {
        autoreleasepool(|| {
            *control_flow = ControlFlow::Wait;

            let mut quit = false;
            frame += 1;

            let repaint_after = gui.run(&window, |egui_ctx| {
                demo.ui(egui_ctx);
                egui::Window::new("Draw to skia")
                    .show(egui_ctx, |ui| {
                        ScrollArea::horizontal()
                            .show(ui, |ui| {
                                let (rect, _) =
                                    ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag());
                                egui_ctx.request_repaint();
                                let si = (frame as f32 / 120.0).sin();
                                let frame = frame.clone();

                                ui.painter().add(egui::PaintCallback {
                                    rect: rect.clone(),
                                    callback: Arc::new(EguiSkiaPaintCallback::new(move |canvas| {
                                        let center = Point::new(150.0, 150.0);
                                        canvas.save();
                                        canvas.rotate(si * 5.0, Some(center));
                                        let mut paint = Paint::default();
                                        paint.set_color(Color::from_argb(255, 255, 255, 255));
                                        canvas.draw_str(
                                            "Hello Skia!",
                                            Point::new(100.0, 150.0),
                                            &Font::default().with_size(20.0).unwrap(),
                                            &paint,
                                        );

                                        let mut circle_paint = Paint::default();
                                        circle_paint.set_path_effect(PathEffect::dash(&[si * 5.0 + 20.0, si * 5.0 + 20.0], 1.0));
                                        circle_paint.set_style(skia_safe::PaintStyle::Stroke);
                                        circle_paint.set_stroke_width(10.0);
                                        circle_paint.set_stroke_cap(skia_safe::PaintCap::Round);
                                        canvas.draw_circle(
                                            center,
                                            100.0,
                                            &circle_paint,
                                        );
                                    }))
                                })
                            });
                    });
            });

            *control_flow = if quit {
                ControlFlow::Exit
            } else if repaint_after.is_zero() {
                window.request_redraw();
                ControlFlow::Poll
            } else if let Some(repaint_after_instant) =
                std::time::Instant::now().checked_add(repaint_after)
            {
                ControlFlow::WaitUntil(repaint_after_instant)
            } else {
                ControlFlow::Wait
            };

            match event {
                Event::WindowEvent { event, .. } => {
                    // Update Egui integration so the UI works!
                    let _pass_events_to_game = !gui.on_event(&event);
                    match event {
                        WindowEvent::ScaleFactorChanged {
                            new_inner_size,
                            ..
                        } => {
                            metal_layer.set_drawable_size(CGSize::new(
                                new_inner_size.width as f64,
                                new_inner_size.height as f64,
                            ));
                            window.request_redraw()
                        }
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(mut size) => {
                            metal_layer.set_drawable_size(CGSize::new(
                                size.width as f64,
                                size.height as f64,
                            ));
                            window.request_redraw()
                        }
                        _ => (),
                    }
                }
                Event::RedrawRequested(_) => {
                    if let Some(drawable) = metal_layer.next_drawable() {
                        let drawable_size = {
                            let size = metal_layer.drawable_size();
                            Size::new(size.width as scalar, size.height as scalar)
                        };

                        let mut surface = unsafe {
                            let texture_info =
                                mtl::TextureInfo::new(drawable.texture().as_ptr() as mtl::Handle);

                            let backend_render_target = BackendRenderTarget::new_metal(
                                (drawable_size.width as i32, drawable_size.height as i32),
                                1,
                                &texture_info,
                            );

                            Surface::from_backend_render_target(
                                &mut context,
                                &backend_render_target,
                                SurfaceOrigin::TopLeft,
                                ColorType::BGRA8888,
                                None,
                                None,
                            )
                            .unwrap()
                        };

                        surface.canvas().clear(skia_safe::colors::TRANSPARENT);

                        gui.paint(&mut surface);

                        surface.flush_and_submit();
                        drop(surface);

                        let command_buffer = command_queue.new_command_buffer();
                        command_buffer.present_drawable(drawable);
                        command_buffer.commit();
                    }
                }
                _ => {}
            }
        });
    });
}
