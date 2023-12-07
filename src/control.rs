use image::RgbImage;

pub enum ToBrain {
    NextFrame(RgbImage),
}
pub enum ToController {
    MoveMouse([i32; 2]),
    PerformClick([i32; 2]),
    CastHook,
}
