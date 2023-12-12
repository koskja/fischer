use crate::control::{Controller, Eyes, ToBrain, ToController};

use std::{ptr, sync::mpsc::{SyncSender, Receiver}, process::Command, time::Instant};
use x11::xlib::{self, _XDisplay};
use image::{ImageBuffer, Rgb};

pub struct XController {
    window: u64,
}
pub struct XEyes {
    window: u64,
}

fn get_window_id_by_title(name: &str) -> eyre::Result<xlib::Window> {
    let output = Command::new("xdotool")
        .args(&["search", "--name", name])
        .output()?;

    let binding = String::from_utf8(output.stdout).expect("Error getting window id");
    let window_id_str = binding.as_str();
    let window_id = window_id_str.trim().parse()?;
    Ok(window_id)
}

impl XEyes {
    pub fn new(window_name: &str) -> eyre::Result<Self>{
        Ok(Self {window: get_window_id_by_title(window_name)?})
    }

    
    pub fn get_image(&self, display: *mut _XDisplay) -> eyre::Result<ImageBuffer<Rgb<u8>,Vec<u8>>> {
        unsafe {
            // Open the display
        
            // Get the window attributes
            let mut window_attributes: xlib::XWindowAttributes = std::mem::zeroed();
            xlib::XGetWindowAttributes(display,  self.window, &mut window_attributes);
    
            // Get the dimensions of the window
            let width = window_attributes.width as u32;
            let height = window_attributes.height as u32;
            
            // Create an XImage structure to hold the screenshot
            let image = xlib::XGetImage(
                display,
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
                    buffer.push(((pixel >> 8) & 0xFF) as u8);  // Green
                    buffer.push((pixel & 0xFF) as u8);         // Blue
                }
            }
    
            // Create an ImageBuffer from the pixel data
            let image_buffer = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, buffer).unwrap();
            
            // Clean up
            xlib::XDestroyImage(image);
            return Ok(image_buffer);
        }
    }
    fn _run(&self, send: SyncSender<ToBrain>) -> eyre::Result<()> {
        let display =unsafe { xlib::XOpenDisplay(ptr::null())};
        loop {
            send.send(ToBrain::NextFrame(self.get_image(display)?))?;
        }
    }
}

impl XController {
    pub fn new(window_name: &str) -> eyre::Result<Self>{
        Ok(Self {window: get_window_id_by_title(window_name)?})
    }

    
    pub fn move_mouse_to_coordinate(&self, display: *mut _XDisplay, x: i32, y: i32) {
        unsafe {   
            // Move the mouse to the specified coordinates
            xlib::XWarpPointer(display, 0, self.window, 0, 0, 0, 0, x, y);
            xlib::XFlush(display);
        }
    }
    pub fn left_click(&self, display: *mut _XDisplay, x: i32, y: i32) {
        unsafe {
            {
            // Create a button press event
            let mut button_event: xlib::XButtonEvent = std::mem::zeroed();
            button_event.type_ = xlib::ButtonPress;
            button_event.display = display;
            button_event.window = self.window;
            button_event.subwindow = 0;
            button_event.time = 0;
            button_event.x = x;
            button_event.y = y;
            button_event.same_screen = xlib::True;
            button_event.button = 1; // 1 corresponds to the left mouse button

            let mut xevent: xlib::XEvent = std::mem::zeroed();
            xevent.button = button_event;
            

            // Send the button press event
            (xlib::XSendEvent)(display, self.window, xlib::True, xlib::ButtonPressMask, &mut xevent);
            (xlib::XFlush)(display);
            }

            {
            let mut button_event: xlib::XButtonEvent = std::mem::zeroed();
            button_event.type_ = xlib::ButtonRelease;
            button_event.display = display;
            button_event.window = self.window;
            button_event.subwindow = 0;
            button_event.time = 0;
            button_event.x = x;
            button_event.y = y;
            button_event.same_screen = xlib::True;
            button_event.button = 1; // 1 corresponds to the left mouse button

            let mut xevent: xlib::XEvent = std::mem::zeroed();
            xevent.button = button_event;


            // Send the button press event
            (xlib::XSendEvent)(display, self.window, xlib::True, xlib::ButtonReleaseMask, &mut xevent);
            (xlib::XFlush)(display);
            }
        }
    }
    pub fn cast_hook(&self, display: *mut _XDisplay) {
        
            unsafe {
                {
                let mut key_event: xlib::XKeyEvent = std::mem::zeroed();
                key_event.type_ = xlib::KeyPress;
                key_event.display = display;
                key_event.window = self.window;
                key_event.subwindow = 0;
                key_event.time = 0;
                key_event.x = 0;
                key_event.y = 0;
                key_event.same_screen = xlib::True;
                key_event.keycode = 49;
                
                let mut xevent: xlib::XEvent = std::mem::zeroed();
                xevent.key = key_event;
        
                (xlib::XSendEvent)(
                display,
                self.window,
                xlib::True,
                xlib::KeyPressMask,
                &mut xevent,
                );

                // Flush the X server to ensure the changes take effect
                (xlib::XFlush)(display);
                }
                {
                let mut key_event: xlib::XKeyEvent = std::mem::zeroed();
                key_event.type_ = xlib::KeyRelease;
                key_event.display = display;
                key_event.window = self.window;
                key_event.subwindow = 0;
                key_event.time = 0;
                key_event.x = 0;
                key_event.y = 0;
                key_event.same_screen = xlib::True;
                key_event.keycode = 49;
                
                let mut xevent: xlib::XEvent = std::mem::zeroed();
                xevent.key = key_event;
        
                (xlib::XSendEvent)(
                display,
                self.window,
                xlib::True,
                xlib::KeyReleaseMask,
                &mut xevent,
                );

                // Flush the X server to ensure the changes take effect
                (xlib::XFlush)(display);
                }
            }
        }


    fn _run(&self, input: Receiver<ToController>) -> eyre::Result<()> {
        let display =unsafe { xlib::XOpenDisplay(ptr::null())};
        loop {
            match input.recv()? {
                ToController::MoveMouse([x, y]) =>  self.move_mouse_to_coordinate(display, x,y),
                ToController::PerformClick([x, y]) => self.left_click(display, x,y),
                ToController::CastHook => self.cast_hook(display),
            }
        }
    }
}
impl Controller for XController {
    fn from_window_name(name: &str) -> eyre::Result<Self> {
        Self::new(name)
    }

    fn run(self, recv: Receiver<ToController>) -> eyre::Result<()> {
        Self::_run(&self, recv)
    }
}
impl Eyes for XEyes {
    fn from_window_name(name: &str) -> eyre::Result<Self> {
        Self::new(name)
    }

    fn run(self, send: SyncSender<ToBrain>) -> eyre::Result<()> {
        self._run(send)
    }
}
