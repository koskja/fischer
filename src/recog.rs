use std::{
    random,
    sync::{
        mpsc::{Receiver, SyncSender},
        OnceLock,
    },
    time::{Duration, Instant},
};

use eyre::Context;
use image::{Pixel as _, Rgb, RgbImage};

use crate::control::{ToBrain, ToController, ToEyes};

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

fn ms_since_start() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(|| Instant::now()).elapsed().as_millis() as u64
}

/// This function finds the center of mass of pixels that fall within a specific
/// HSV color range, tuned to find the red part of the fishing bobber.
pub fn find_bobber(img: &RgbImage, background: &RgbImage) -> Option<(i32, i32)> {
    let mut total_x = 0.0;
    let mut total_y = 0.0;
    let mut count = 0;

    assert_eq!(img.dimensions(), background.dimensions());
    let mut output = img.clone();
    let mut output_hsv = img.clone();
    let mut output_cmyk = img.clone();
    let mut output_detected_pixels = img.clone();

    let w = img.width();
    let h = img.height();
    let left = w / 3;
    let right = 2 * w / 3;
    let top = h / 3;
    let bottom = 2 * h / 3;

    for y in top..bottom {
        for x in left..right {
            let foreground_pixel = img.get_pixel(x, y);
            let background_pixel = background.get_pixel(x, y);
            let pixel =
                foreground_pixel.map2(background_pixel, |a, b| (a as i32 - b as i32).abs() as u8);
            output.put_pixel(x, y, pixel);
            let (cyan, m, yellow, _) = rgb_to_cmyk(pixel);
            let (h, saturation, v) = rgb_to_hsv(pixel);
            output_cmyk.put_pixel(
                x,
                y,
                Rgb([
                    (cyan * 255.0) as u8,
                    (m * 255.0) as u8,
                    (yellow * 255.0) as u8,
                ]),
            );
            output_hsv.put_pixel(
                x,
                y,
                Rgb([
                    (h / 360.0 * 255.0) as u8,
                    (saturation * 255.0) as u8,
                    (v * 255.0) as u8,
                ]),
            );

            // HSV thresholding for red color
            let is_red = (h < 20.0 || h > 340.0) && saturation > 0.5 && v > 0.3;

            if is_red {
                total_x += x as f64;
                total_y += y as f64;
                count += 1;
                output_detected_pixels.put_pixel(x, y, Rgb([255, 0, 0]));
            }
        }
    }

    if false && random::random::<u8>() % 100 == 0 {
        output
            .save(format!("output_{}.png", ms_since_start()))
            .unwrap();
        output_cmyk
            .save(format!("output_cmyk_{}.png", ms_since_start()))
            .unwrap();
        output_hsv
            .save(format!("output_hsv_{}.png", ms_since_start()))
            .unwrap();
        output_detected_pixels
            .save(format!("output_detected_pixels_{}.png", ms_since_start()))
            .unwrap();
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
                (bx - nx).pow(2) + (by - ny).pow(2) > 10i32.pow(2)
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
    reference_image: Option<RgbImage>,
    stopped_casting_at: Option<Instant>,
}
impl Brain {
    pub fn new() -> Self {
        Self {
            ongoing: None,
            reference_image: None,
            stopped_casting_at: None,
        }
    }
    pub fn cast(&mut self) -> eyre::Result<ToController> {
        self.ongoing = Some(HookCast::new());
        Ok(ToController::CastHook)
    }
    pub fn run(
        mut self,
        input: Receiver<ToBrain>,
        output: SyncSender<ToController>,
        eyes_feedback: SyncSender<ToEyes>,
    ) -> eyre::Result<()> {
        let log_frames = false;
        let timestamp = Instant::now();
        loop {
            if self.reference_image.is_some() {
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
            }
            let frame = input.recv().wrap_err("Failed to receive next input")?;
            'anchor: loop {
                match frame {
                    ToBrain::NextFrame(mut frame) => {
                        if self.reference_image.is_none() {
                            if self.stopped_casting_at.is_none()
                                || Instant::now() - self.stopped_casting_at.unwrap()
                                    > Duration::from_secs(3)
                            {
                                self.reference_image = Some(frame);
                            }
                            break 'anchor;
                        }
                        if let Some((x, y)) =
                            find_bobber(&frame, self.reference_image.as_ref().unwrap())
                        {
                            if log_frames {
                                for i in -10..=10 {
                                    for j in -10..=10 {
                                        frame.put_pixel(
                                            (x + i) as u32,
                                            (y + j) as u32,
                                            Rgb([255, 0, 0]),
                                        );
                                    }
                                }
                                frame
                                    .save(format!("frame_{}.png", timestamp.elapsed().as_millis()))
                                    .unwrap();
                            }
                            output.send(ToController::MoveMouse([x, y]))?;
                            if self.ongoing.as_mut().unwrap().register_pos([x, y]) {
                                output.send(ToController::PerformClick([x, y]))?;
                                self.ongoing = None;
                                self.reference_image = None;
                                self.stopped_casting_at = Some(Instant::now());
                                println!("Bere!");
                            }
                        }
                    }
                };
                break;
            }
            eyes_feedback.send(ToEyes::FrameProcessed)?;
        }
    }
}
impl Default for Brain {
    fn default() -> Self {
        Self::new()
    }
}
