use crate::painter::Painter;
use egui::{Context, PaintCallbackInfo, Pos2};
use skia_safe::{Size, Surface};
use std::time::Duration;

pub struct RasterizeOptions {
    pub pixels_per_point: f32,
}

impl Default for RasterizeOptions {
    fn default() -> Self {
        Self {
            pixels_per_point: 1.0,
        }
    }
}

pub fn rasterize(
    size: (i32, i32),
    ui: impl FnMut(&Context),
    options: Option<RasterizeOptions>,
) -> Surface {
    let mut surface = Surface::new_raster_n32_premul(size).expect("Failed to create surface");
    draw_onto_surface(&mut surface, ui, options);
    surface
}

pub fn draw_onto_surface(
    surface: &mut Surface,
    ui: impl FnMut(&Context),
    options: Option<RasterizeOptions>,
) {
    let RasterizeOptions { pixels_per_point } = options.unwrap_or_default();
    let mut backend = EguiSkia::new();

    let input = egui::RawInput {
        screen_rect: Some(
            [
                Pos2::default(),
                Pos2::new(surface.width() as f32, surface.height() as f32),
            ]
            .into(),
        ),
        pixels_per_point: Some(pixels_per_point),
        ..Default::default()
    };

    backend.run(input, ui);

    backend.paint(surface);
}

/// Convenience wrapper for using [`egui`] from a [`skia`] app.
pub struct EguiSkia {
    pub egui_ctx: Context,
    pub painter: Painter,

    shapes: Vec<egui::epaint::ClippedShape>,
    textures_delta: egui::TexturesDelta,
}

impl EguiSkia {
    pub fn new() -> Self {
        let painter = Painter::new();
        Self {
            egui_ctx: Default::default(),
            painter,
            shapes: Default::default(),
            textures_delta: Default::default(),
        }
    }

    /// Returns a duration after witch egui should repaint.
    ///
    /// Call [`Self::paint`] later to paint.
    pub fn run(
        &mut self,
        input: egui::RawInput,
        run_ui: impl FnMut(&Context),
    ) -> (Duration, egui::PlatformOutput) {
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            repaint_after,
        } = self.egui_ctx.run(input, run_ui);

        self.shapes = shapes;
        self.textures_delta.append(textures_delta);

        (repaint_after, platform_output)
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self, surface: &mut Surface) {
        let shapes = std::mem::take(&mut self.shapes);
        let textures_delta = std::mem::take(&mut self.textures_delta);
        let clipped_primitives = self.egui_ctx.tessellate(shapes);
        self.painter.paint_and_update_textures(
            surface,
            self.egui_ctx.pixels_per_point(),
            clipped_primitives,
            textures_delta,
        );
    }
}
