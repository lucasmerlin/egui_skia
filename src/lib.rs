extern crate core;

mod egui_skia;
mod painter;

#[cfg(feature = "winit")]
mod egui_skia_winit;
#[cfg(feature = "winit")]
pub use egui_skia_winit::EguiSkiaWinit;
#[cfg(feature = "sdl2")]
mod egui_skia_sdl2;
#[cfg(feature = "sdl2")]
pub use egui_skia_sdl2::GetDpi;

pub use egui_skia::*;
pub use painter::EguiSkiaPaintCallback;
