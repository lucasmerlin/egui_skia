use std::ops::Deref;
use std::time::Duration;

use egui::{Context, PaintCallbackInfo};
pub use egui_winit;
use egui_winit::winit::event_loop::EventLoopWindowTarget;
use egui_winit::winit::window::Window;
use objc::sel;
use skia_safe::Surface;

use crate::EguiSkia;

pub struct EguiSkiaWinit {
    pub egui_skia: EguiSkia,
    pub egui_winit: egui_winit::State,
}

impl EguiSkiaWinit {
    pub fn new<T>(el: &EventLoopWindowTarget<T>) -> Self {
        let mut egui_winit = egui_winit::State::new(el);

        Self {
            egui_winit,
            egui_skia: EguiSkia::new(),
        }
    }

    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn on_event(&mut self, event: &egui_winit::winit::event::WindowEvent<'_>) -> bool {
        self.egui_winit.on_event(&self.egui_skia.egui_ctx, event)
    }

    /// Returns a duration after witch egui should repaint.
    ///
    /// Call [`Self::paint`] later to paint.
    pub fn run(&mut self, window: &Window, run_ui: impl FnMut(&Context)) -> Duration {
        let raw_input = self.egui_winit.take_egui_input(window);

        let (repaint_after, platform_output) = self.egui_skia.run(raw_input, run_ui);

        self.egui_winit
            .handle_platform_output(window, &self.egui_skia.egui_ctx, platform_output);
        repaint_after
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self, surface: &mut Surface) {
        self.egui_skia.paint(surface);
    }
}
