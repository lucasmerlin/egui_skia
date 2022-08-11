extern crate core;

mod egui_skia;
mod painter;

#[cfg(feature = "winit")]
mod egui_skia_winit;
#[cfg(feature = "winit")]
pub use egui_skia_winit::EguiSkiaWinit;

pub use egui_skia::*;
pub use painter::{EguiSkiaPaintCallback};
