use crate::control::{Controller, Eyes, GuiContext, ToBrain, ToController};

use eyre::Context;
use image::{ImageBuffer, Rgb};
use std::{
    process::Command,
    ptr,
    sync::mpsc::{Receiver, SyncSender},
};
use x11::xlib::{self, _XDisplay};

#[derive(Debug, Clone, Copy)]
pub struct XContext {
    window: xlib::Window,
}
impl GuiContext for XContext {
    type Controller = XController;
    type Eyes = XEyes;

    fn from_window_name(name: &str) -> eyre::Result<Self> {
        let window = get_window_id_by_title(name)?;
        Ok(Self { window })
    }

    fn controller(&self) -> eyre::Result<Self::Controller> {
        XController::new(self.window)
    }

    fn eyes(&self) -> eyre::Result<Self::Eyes> {
        XEyes::new(self.window)
    }
}

pub struct XController {
    window: xlib::Window,
    display: *mut _XDisplay,
}

pub struct XEyes {
    window: xlib::Window,
    display: *mut _XDisplay,
}

fn get_window_id_by_title(name: &str) -> eyre::Result<xlib::Window> {
    let output = Command::new("xdotool")
        .args(&["search", "--name", &format!("{name}$")])
        .output()?;

    let binding =
        String::from_utf8(output.stdout).expect("Error converting process stdout to utf8");
    let window_id_str = binding.as_str();
    let window_id = window_id_str
        .trim()
        .parse()
        .context(format!("Window {name} not found"))?;
    Ok(window_id)
}

impl XEyes {
    pub fn new(window: xlib::Window) -> eyre::Result<Self> {
        Ok(Self {
            window,
            display: unsafe { xlib::XOpenDisplay(ptr::null()) },
        })
    }

    pub fn get_image(&self) -> eyre::Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
        unsafe {
            // Get the window attributes
            let mut window_attributes: xlib::XWindowAttributes = std::mem::zeroed();
            xlib::XGetWindowAttributes(self.display, self.window, &mut window_attributes);

            // Get the dimensions of the window
            let width = window_attributes.width as u32;
            let height = window_attributes.height as u32;

            // Create an XImage structure to hold the screenshot
            let image = xlib::XGetImage(
                self.display,
                self.window,
                0,
                0,
                width,
                height,
                xlib::XAllPlanes(),
                xlib::ZPixmap,
            );

            // Process the image data
            let mut buffer: Vec<u8> = Vec::with_capacity((width * height * 4) as usize);

            for y in 0..height {
                for x in 0..width {
                    let pixel = xlib::XGetPixel(image, x as i32, y as i32);
                    buffer.push(((pixel >> 16) & 0xFF) as u8); // Red
                    buffer.push(((pixel >> 8) & 0xFF) as u8); // Green
                    buffer.push((pixel & 0xFF) as u8); // Blue
                }
            }

            // Create an ImageBuffer from the pixel data
            let image_buffer = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, buffer).unwrap();

            // Clean up
            xlib::XDestroyImage(image);
            return Ok(image_buffer);
        }
    }
}

impl XController {
    pub fn new(window: xlib::Window) -> eyre::Result<Self> {
        Ok(Self {
            window,
            display: unsafe { xlib::XOpenDisplay(ptr::null()) },
        })
    }

    pub fn move_mouse_to_coordinate(&self, x: i32, y: i32) {
        unsafe {
            // Move the mouse to the specified coordinates
            xlib::XWarpPointer(self.display, 0, self.window, 0, 0, 0, 0, x, y);
        }
    }
    pub fn left_click(&self, x: i32, y: i32) {
        unsafe {
            // Create a button press event
            let button_event: xlib::XButtonEvent = xlib::XButtonEvent {
                type_: xlib::ButtonPress,
                display: self.display,
                window: self.window,
                subwindow: 0,
                time: 0,
                x: x,
                y: y,
                same_screen: xlib::True,
                button: 1,
                ..std::mem::zeroed()
            };

            let mut xevent: xlib::XEvent = xlib::XEvent {
                button: button_event,
            };

            // Send the button press event
            xlib::XSendEvent(
                self.display,
                self.window,
                xlib::True,
                xlib::ButtonPressMask,
                &mut xevent,
            );

            let button_event: xlib::XButtonEvent = xlib::XButtonEvent {
                type_: xlib::ButtonRelease,
                display: self.display,
                window: self.window,
                subwindow: 0,
                time: 0,
                x: x,
                y: y,
                same_screen: xlib::True,
                button: 1,
                ..std::mem::zeroed()
            };

            let mut xevent: xlib::XEvent = xlib::XEvent {
                button: button_event,
            };

            // Send the button press event
            xlib::XSendEvent(
                self.display,
                self.window,
                xlib::True,
                xlib::ButtonReleaseMask,
                &mut xevent,
            );
        }
    }
    pub fn cast_hook(&self) {
        unsafe {
            let key_event: xlib::XKeyEvent = xlib::XKeyEvent {
                type_: xlib::KeyPress,
                display: self.display,
                window: self.window,
                subwindow: 0,
                time: 0,
                x: 0,
                y: 0,
                same_screen: xlib::True,
                keycode: 49,
                ..std::mem::zeroed()
            };

            let mut xevent: xlib::XEvent = xlib::XEvent { key: key_event };

            xlib::XSendEvent(
                self.display,
                self.window,
                xlib::True,
                xlib::KeyPressMask,
                &mut xevent,
            );

            let key_event: xlib::XKeyEvent = xlib::XKeyEvent {
                type_: xlib::KeyRelease,
                display: self.display,
                window: self.window,
                subwindow: 0,
                time: 0,
                x: 0,
                y: 0,
                same_screen: xlib::True,
                keycode: 49,
                ..std::mem::zeroed()
            };

            let mut xevent: xlib::XEvent = xlib::XEvent { key: key_event };

            xlib::XSendEvent(
                self.display,
                self.window,
                xlib::True,
                xlib::KeyReleaseMask,
                &mut xevent,
            );
        }
    }
}
impl Controller for XController {
    fn run(self, input: Receiver<ToController>) -> eyre::Result<()> {
        loop {
            match input.recv()? {
                ToController::MoveMouse([x, y]) => self.move_mouse_to_coordinate(x, y),
                ToController::PerformClick([x, y]) => self.left_click(x, y),
                ToController::CastHook => self.cast_hook(),
            }
            unsafe {
                xlib::XFlush(self.display);
            }
        }
    }
}
impl Eyes for XEyes {
    fn run(self, send: SyncSender<ToBrain>) -> eyre::Result<()> {
        loop {
            send.send(ToBrain::NextFrame(self.get_image()?))?;
        }
    }
}

unsafe impl Send for XController {}
unsafe impl Sync for XController {}

unsafe impl Send for XEyes {}
unsafe impl Sync for XEyes {}
