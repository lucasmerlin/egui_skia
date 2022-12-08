extern crate sdl2;
use sdl2::video::Window;

const DEFAULT_DPI: f32 = 96.0;

pub trait GetDpi {
    fn display_dpi(&self) -> Option<f32>;
    fn infallible_dpi(&self) -> f32 {
        self.display_dpi().unwrap_or(1.0)
    }
}

#[cfg(not(target_os = "macos"))]
impl GetDpi for Window {
    fn display_dpi(&self) -> Option<f32> {
        let display_index = self.display_index().ok()?;
        let system = self.subsystem();
        let (_, dpi, _) = system.display_dpi(display_index).ok()?;
        Some((1.0 / (DEFAULT_DPI / dpi)).into())
    }
}

/// On MacOS, the DPI scaling is automatically handled.
#[cfg(target_os = "macos")]
impl GetDpi for Window {
    fn display_dpi(&self) -> Option<f32> {
        Some(1.0)
    }
}
