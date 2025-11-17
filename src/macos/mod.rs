use std::sync::mpsc::{Receiver, SyncSender};

use crate::control::{Controller, Eyes, FrameBudget, GuiContext, ToBrain, ToController, ToEyes};
use core_foundation::{
    base::{CFType, TCFType},
    boolean::CFBoolean,
    dictionary::{CFDictionary, CFDictionaryRef},
    number::CFNumber,
    string::CFString,
};
use core_graphics::{
    display::{self, CGDisplay},
    event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton},
    event_source::{CGEventSource, CGEventSourceStateID},
    geometry::{CGPoint, CGRect, CGSize},
    image::CGImage,
};
use eyre::{bail, ensure, eyre, Context};
use image::{ImageBuffer, RgbImage};

const KVK_ANSI_GRAVE: u16 = 50;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

pub fn request_permissions() -> eyre::Result<()> {
    let accessibility_granted = request_accessibility_permission();
    if !accessibility_granted {
        eyre::bail!(
            "macOS accessibility permission not granted. Please enable accessibility access \
             for this application in System Settings › Privacy & Security › Accessibility."
        );
    }

    let screen_granted = request_screen_capture_permission();
    if !screen_granted {
        eyre::bail!(
            "macOS screen recording permission not granted. Please enable screen recording \
             for this application in System Settings › Privacy & Security › Screen Recording."
        );
    }

    Ok(())
}

fn request_accessibility_permission() -> bool {
    let prompt_key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
    let prompt_value = CFBoolean::true_value();
    let options =
        CFDictionary::from_CFType_pairs(&[(prompt_key.as_CFType(), prompt_value.as_CFType())]);

    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
}

fn request_screen_capture_permission() -> bool {
    unsafe {
        if CGPreflightScreenCaptureAccess() {
            true
        } else {
            CGRequestScreenCaptureAccess()
        }
    }
}

#[derive(Debug, Clone)]
struct WindowInfo {
    id: u32,
    bounds: CGRect,
}

pub struct MacosContext {
    window: WindowInfo,
}

pub struct MacosController {
    window: WindowInfo,
    scale_factor: f64,
}

pub struct MacosEyes {
    window: WindowInfo,
}

impl GuiContext for MacosContext {
    type Controller = MacosController;
    type Eyes = MacosEyes;

    fn from_window_name(name: &str) -> eyre::Result<Self> {
        let name = name.trim();
        ensure!(!name.is_empty(), "window name must not be empty");
        let window = find_window_by_title(name)
            .with_context(|| format!("failed to locate window named '{name}'"))?;
        Ok(Self { window })
    }

    fn controller(&self) -> eyre::Result<Self::Controller> {
        MacosController::new(self.window.clone())
    }

    fn eyes(&self) -> eyre::Result<Self::Eyes> {
        Ok(MacosEyes {
            window: self.window.clone(),
        })
    }
}

impl MacosController {
    fn new(window: WindowInfo) -> eyre::Result<Self> {
        let display = CGDisplay::main();
        let bounds = display.bounds();
        let scale_factor = if bounds.size.width > 0.0 {
            display.pixels_wide() as f64 / bounds.size.width
        } else {
            1.0
        };
        Ok(Self {
            window,
            scale_factor,
        })
    }

    fn move_mouse(&self, coords: [i32; 2]) -> eyre::Result<()> {
        self.post_mouse_event(CGEventType::MouseMoved, coords)
    }

    fn click_mouse(&self, coords: [i32; 2]) -> eyre::Result<()> {
        let source = create_event_source()?;
        let point = self.global_point(coords);
        let down = CGEvent::new_mouse_event(
            source.clone(),
            CGEventType::LeftMouseDown,
            point,
            CGMouseButton::Left,
        )
        .map_err(|_| eyre!("failed to create mouse down event"))?;
        down.post(CGEventTapLocation::HID);

        let up =
            CGEvent::new_mouse_event(source, CGEventType::LeftMouseUp, point, CGMouseButton::Left)
                .map_err(|_| eyre!("failed to create mouse up event"))?;
        up.post(CGEventTapLocation::HID);
        Ok(())
    }

    fn cast(&self) -> eyre::Result<()> {
        let source = create_event_source()?;
        let down = CGEvent::new_keyboard_event(source.clone(), KVK_ANSI_GRAVE, true)
            .map_err(|_| eyre!("failed to create key down event"))?;
        down.post(CGEventTapLocation::HID);

        let up = CGEvent::new_keyboard_event(source, KVK_ANSI_GRAVE, false)
            .map_err(|_| eyre!("failed to create key up event"))?;
        up.post(CGEventTapLocation::HID);
        Ok(())
    }

    fn post_mouse_event(&self, event_type: CGEventType, coords: [i32; 2]) -> eyre::Result<()> {
        let source = create_event_source()?;
        let point = self.global_point(coords);
        let event = CGEvent::new_mouse_event(source, event_type, point, CGMouseButton::Left)
            .map_err(|_| eyre!("failed to create mouse event"))?;
        event.post(CGEventTapLocation::HID);
        Ok(())
    }

    fn global_point(&self, coords: [i32; 2]) -> CGPoint {
        let x = self.window.bounds.origin.x + coords[0] as f64 / self.scale_factor / 2.0;
        let y = self.window.bounds.origin.y + coords[1] as f64 / self.scale_factor / 2.0;
        CGPoint::new(x, y)
    }
}

impl Controller for MacosController {
    fn run(self, recv: Receiver<ToController>) -> eyre::Result<()> {
        loop {
            match recv.recv()? {
                ToController::MoveMouse(coords) => self.move_mouse(coords)?,
                ToController::PerformClick(coords) => {
                    self.move_mouse(coords)?;
                    self.click_mouse(coords)?;
                }
                ToController::CastHook => self.cast()?,
            }
        }
    }
}

impl MacosEyes {
    fn capture(&self) -> eyre::Result<RgbImage> {
        let image = CGDisplay::screenshot(
            self.window.bounds,
            display::kCGWindowListOptionIncludingWindow,
            self.window.id,
            display::kCGWindowImageDefault,
        )
        .ok_or_else(|| eyre!("failed to capture window image"))?;

        cgimage_to_rgb(&image)
    }
}

impl Eyes for MacosEyes {
    fn run(self, send: SyncSender<ToBrain>, recv: Receiver<ToEyes>) -> eyre::Result<()> {
        let mut budget = FrameBudget::new(recv);
        loop {
            budget.wait_for_slot()?;
            let frame = self.capture()?;
            send.send(ToBrain::NextFrame(frame))?;
            budget.frame_sent()?;
        }
    }
}

fn create_event_source() -> eyre::Result<CGEventSource> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| eyre!("failed to create CGEvent source"))
}

fn find_window_by_title(name: &str) -> eyre::Result<WindowInfo> {
    let options =
        display::kCGWindowListOptionOnScreenOnly | display::kCGWindowListExcludeDesktopElements;
    let list = CGDisplay::window_list_info(options, None)
        .ok_or_else(|| eyre!("failed to query macOS window list"))?;

    let key_number = CFString::from_static_string("kCGWindowNumber");
    let key_bounds = CFString::from_static_string("kCGWindowBounds");
    let key_name = CFString::from_static_string("kCGWindowName");
    let key_owner = CFString::from_static_string("kCGWindowOwnerName");

    let bounds_x = CFString::from_static_string("X");
    let bounds_y = CFString::from_static_string("Y");
    let bounds_w = CFString::from_static_string("Width");
    let bounds_h = CFString::from_static_string("Height");

    for entry in list.iter() {
        let dict_ref = *entry as CFDictionaryRef;
        let dict = unsafe { CFDictionary::<CFString, CFType>::wrap_under_get_rule(dict_ref) };

        let window_title = dict
            .find(&key_name)
            .and_then(|value| value.downcast::<CFString>())
            .map(|s| s.to_string());
        let owner_title = dict
            .find(&key_owner)
            .and_then(|value| value.downcast::<CFString>())
            .map(|s| s.to_string());

        if !matches_search(name, window_title.as_deref(), owner_title.as_deref()) {
            continue;
        }

        let window_id = dict
            .find(&key_number)
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|num| num.to_i64())
            .ok_or_else(|| eyre!("missing window identifier"))? as u32;

        let bounds_dict = dict
            .find(&key_bounds)
            .and_then(|value| value.downcast::<CFDictionary>())
            .map(|dict| unsafe {
                CFDictionary::<CFString, CFType>::wrap_under_get_rule(dict.as_concrete_TypeRef())
            })
            .ok_or_else(|| eyre!("missing window bounds"))?;

        let x = bounds_dict
            .find(&bounds_x)
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|num| num.to_f64())
            .ok_or_else(|| eyre!("missing window bounds x"))?;
        let y = bounds_dict
            .find(&bounds_y)
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|num| num.to_f64())
            .ok_or_else(|| eyre!("missing window bounds y"))?;
        let width = bounds_dict
            .find(&bounds_w)
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|num| num.to_f64())
            .ok_or_else(|| eyre!("missing window bounds width"))?;
        let height = bounds_dict
            .find(&bounds_h)
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|num| num.to_f64())
            .ok_or_else(|| eyre!("missing window bounds height"))?;

        let bounds = CGRect::new(&CGPoint::new(x, y), &CGSize::new(width, height));

        return Ok(WindowInfo {
            id: window_id,
            bounds,
        });
    }

    bail!("window '{name}' not found");
}

fn cgimage_to_rgb(image: &CGImage) -> eyre::Result<RgbImage> {
    let width = image.width();
    let height = image.height();
    let bytes_per_row = image.bytes_per_row();

    let data = image.data();
    let bytes = data.bytes();

    let mut buffer = vec![0u8; width * height * 3];
    for y in 0..height {
        let row_start = y * bytes_per_row;
        for x in 0..width {
            let src = row_start + x * 4;
            let dst = (y * width + x) * 3;
            let b = bytes[src] as f32;
            let g = bytes[src + 1] as f32;
            let r = bytes[src + 2] as f32;
            let a = bytes[src + 3];

            if a == 0 {
                buffer[dst] = 0;
                buffer[dst + 1] = 0;
                buffer[dst + 2] = 0;
            } else if a == 255 {
                buffer[dst] = r as u8;
                buffer[dst + 1] = g as u8;
                buffer[dst + 2] = b as u8;
            } else {
                let alpha = a as f32 / 255.0;
                buffer[dst] = (r / alpha).clamp(0.0, 255.0) as u8;
                buffer[dst + 1] = (g / alpha).clamp(0.0, 255.0) as u8;
                buffer[dst + 2] = (b / alpha).clamp(0.0, 255.0) as u8;
            }
        }
    }

    ImageBuffer::from_raw(width as u32, height as u32, buffer)
        .ok_or_else(|| eyre!("failed to build RGB image"))
}

fn matches_search(target: &str, window_title: Option<&str>, owner_title: Option<&str>) -> bool {
    if window_title == Some(target) || owner_title == Some(target) {
        return true;
    }

    match (owner_title, window_title) {
        (Some(owner), Some(window)) => {
            format!("{owner} - {window}") == target || format!("{window} - {owner}") == target
        }
        _ => false,
    }
}
