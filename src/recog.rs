use std::{
    sync::mpsc::{Receiver, SyncSender},
    time::{Duration, Instant},
};

use eyre::Context;
use image::{Rgb, RgbImage};

use crate::control::{ToBrain, ToController};

/// Convert` RGB color to CMYK color space.
fn rgb_to_cmyk(rgb: Rgb<u8>) -> (f64, f64, f64, f64) {
    let r = rgb[0] as f64 / 255.0;
    let g = rgb[1] as f64 / 255.0;
    let b = rgb[2] as f64 / 255.0;

    let k = 1.0 - r.max(g).max(b);
    let c = (1.0 - r - k) / (1.0 - k);
    let m = (1.0 - g - k) / (1.0 - k);
    let y = (1.0 - b - k) / (1.0 - k);

    (c, m, y, k)
}

/// Convert RGB color to HSV color space.
fn rgb_to_hsv(rgb: Rgb<u8>) -> (f64, f64, f64) {
    let r = rgb[0] as f64 / 255.0;
    let g = rgb[1] as f64 / 255.0;
    let b = rgb[2] as f64 / 255.0;

    let c_max = r.max(g).max(b);
    let c_min = r.min(g).min(b);
    let delta = c_max - c_min;

    let hue = if delta == 0.0 {
        0.0
    } else if c_max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if c_max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let saturation = if c_max == 0.0 { 0.0 } else { delta / c_max };

    let value = c_max;

    (hue, saturation, value)
}

/// This function finds the center of mass of pixels with a cyan value < 10% and saturation > 40%.
pub fn find_bobber(img: &RgbImage) -> Option<(i32, i32)> {
    let mut total_x = 0.0;
    let mut total_y = 0.0;
    let mut count = 0;

    let w = img.width();
    let h = img.height();
    for (x, y, pixel) in img.enumerate_pixels() {
        if w / 3 > x || 2 * w / 3 < x || h / 3 > y || 2 * h / 3 < y {
            continue;
        }
        let (cyan, _, _, _) = rgb_to_cmyk(*pixel);
        let (_, saturation, _) = rgb_to_hsv(*pixel);

        let cyan_threshold = 0.1;
        let saturation_threshold = 0.4;

        // Check if the pixel meets the criteria
        if cyan < cyan_threshold && saturation > saturation_threshold {
            total_x += x as f64;
            total_y += y as f64;
            count += 1;
        }
    }

    if count > 0 {
        // Calculate the center of mass
        let center_x = (total_x / count as f64) as i32;
        let center_y = (total_y / count as f64) as i32;
        Some((center_x, center_y))
    } else {
        None
    }
}

pub struct HookCast {
    start: Instant,
    bobber_pos: Option<[i32; 2]>,
}
impl HookCast {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            bobber_pos: None,
        }
    }
    /// Returns true if the `pos` is sufficiently different
    pub fn register_pos(&mut self, [nx, ny]: [i32; 2]) -> bool {
        if (Instant::now() - self.start).as_millis() > 3000 {
            if let Some([bx, by]) = self.bobber_pos {
                (bx - nx).pow(2) + (by - ny).pow(2) > 25
            } else {
                self.bobber_pos = Some([nx, ny]);
                false
            }
        } else {
            false
        }
    }
}
pub struct Brain {
    ongoing: Option<HookCast>,
}
impl Brain {
    pub fn new() -> Self {
        Self { ongoing: None }
    }
    pub fn cast(&mut self) -> eyre::Result<ToController> {
        self.ongoing = Some(HookCast::new());
        Ok(ToController::CastHook)
    }
    pub fn run(
        mut self,
        input: Receiver<ToBrain>,
        output: SyncSender<ToController>,
    ) -> eyre::Result<()> {
        loop {
            if self.ongoing.is_none() {
                output.send(self.cast()?)?;
                continue;
            }
            if let Some(cast) = &self.ongoing
                && Instant::now() - cast.start > Duration::from_secs(30)
            {
                output.send(self.cast()?)?;
                continue;
            }
            let frame = input.recv().wrap_err("Failed to receive next input")?;
            match frame {
                ToBrain::NextFrame(frame) => {
                    if let Some((x, y)) = find_bobber(&frame) {
                        output.send(ToController::MoveMouse([x, y]))?;
                        if self.ongoing.as_mut().unwrap().register_pos([x, y]) {
                            output.send(ToController::PerformClick([x, y]))?;
                            self.ongoing = None;
                            println!("Bere!");
                        }
                    }
                }
            };
        }
    }
}

impl Default for Brain {
    fn default() -> Self {
        Self::new()
    }
}
