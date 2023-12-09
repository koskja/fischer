mod bitmap;
use std::{
    ffi::CString,
    ops::Deref,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread::spawn,
};
use bitflags::bitflags;
use eyre::bail;
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{GetWindowDC, HDC},
        UI::{
            Input::KeyboardAndMouse::{
                SendInput, SetActiveWindow, INPUT, INPUT_0, INPUT_TYPE, KEYBDINPUT,
                KEYBD_EVENT_FLAGS, MOUSEINPUT, VIRTUAL_KEY,
            },
            WindowsAndMessaging::{
                FindWindowA, GetDesktopWindow, GetWindowRect, SetForegroundWindow,
            },
        },
    },
};
use crate::control::{ToBrain, ToController, Controller, Eyes};
use self::bitmap::Bitmap;

#[derive(Debug, Clone, Copy)]
pub struct SIZE {
    width: i32,
    height: i32,
}
impl SIZE {
    pub fn of(r: RECT) -> Self {
        Self {
            width: r.right - r.left,
            height: r.bottom - r.top,
        }
    }
}

pub fn convert_kb(ki: KEYBDINPUT) -> INPUT {
    INPUT {
        r#type: INPUT_TYPE(1),
        Anonymous: INPUT_0 { ki },
    }
}
pub fn convert_mouse(mi: MOUSEINPUT) -> INPUT {
    INPUT {
        r#type: INPUT_TYPE(0),
        Anonymous: INPUT_0 { mi },
    }
}
pub fn create_mouse_input(x: i32, y: i32, m: MouseEventFlags) -> INPUT {
    convert_mouse(MOUSEINPUT {
        dx: x,
        dy: y,
        mouseData: 0,
        dwFlags: unsafe { core::mem::transmute(m.bits()) },
        time: 0,
        dwExtraInfo: 0,
    })
}
pub fn kb_input(vk: u16, flags: KeyEventFlags) -> INPUT {
    convert_kb(KEYBDINPUT {
        wVk: VIRTUAL_KEY(vk),
        wScan: 0,
        dwFlags: KEYBD_EVENT_FLAGS(flags.bits()),
        time: 0,
        dwExtraInfo: 0,
    })
}

pub fn mouse_input_list(x: i32, y: i32, rect: RECT, m: &[MouseEventFlags]) -> Vec<INPUT> {
    m.into_iter().map(|m| mouse_input(x, y, rect, *m)).collect()
}

pub fn mouse_input(x: i32, y: i32, rect: RECT, m: MouseEventFlags) -> INPUT {
    let screen_rect = window_rect(unsafe { GetDesktopWindow() });
    let screen_size = SIZE::of(screen_rect);
    let x = rect.left + x;
    let y = rect.top + y;
    create_mouse_input(
        x * 65535 / screen_size.width,
        y * 65535 / screen_size.height,
        m | MouseEventFlags::Absolute,
    )
}

pub fn window_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.unwrap();
    rect
}

pub fn send_input(inputs: impl Deref<Target = [INPUT]>) -> u32 {
    unsafe { SendInput(inputs.deref(), core::mem::size_of::<INPUT>() as i32) }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KeyEventFlags: u32 {
        /// If specified, the wScan scan code consists of a sequence of two bytes,
        /// where the first byte has a value of 0xE0.
        const ExtendedKey = 0x0001;

        /// If specified, the key is being released. If not specified, the key is being pressed.
        const KeyUp = 0x0002;

        /// If specified, wScan identifies the key and wVk is ignored.
        const ScanCode = 0x0008;

        /// If specified, the system synthesizes a VK_PACKET keystroke.
        /// The wVk parameter must be zero. This flag can only be combined with the KEYEVENTF_KEYUP flag.
        const Unicode = 0x0004;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MouseEventFlags: u32 {
        /// Movement occurred.
        const Move = 0x0001;
        /// The left button was pressed.
        const LeftDown = 0x0002;
        /// The left button was released.
        const LeftUp = 0x0004;
        /// The right button was pressed.
        const RightDown = 0x0008;
        /// The right button was released.
        const RightUp = 0x0010;
        /// The middle button was pressed.
        const MiddleDown = 0x0020;
        /// The middle button was released.
        const MiddleUp = 0x0040;
        /// An X button was pressed.
        const XDown = 0x0080;
        /// An X button was released.
        const XUp = 0x0100;
        /// The wheel was moved, if the mouse has a wheel.
        const Wheel = 0x0800;
        /// The wheel was moved horizontally, if the mouse has a wheel.
        const HWheel = 0x1000;
        /// The WM_MOUSEMOVE messages will not be coalesced.
        const MoveNoCoalesce = 0x2000;
        /// Maps coordinates to the entire desktop.
        const VirtualDesk = 0x4000;
        /// The dx and dy members contain normalized absolute coordinates.
        const Absolute = 0x8000;
    }
}

pub mod keycodes {
    pub const VK_LBUTTON: u16 = 0x01;
    pub const VK_RBUTTON: u16 = 0x02;
    pub const VK_CANCEL: u16 = 0x03;
    pub const VK_MBUTTON: u16 = 0x04;
    pub const VK_XBUTTON1: u16 = 0x05;
    pub const VK_XBUTTON2: u16 = 0x06;
    pub const VK_BACK: u16 = 0x08;
    pub const VK_TAB: u16 = 0x09;
    pub const VK_CLEAR: u16 = 0x0C;
    pub const VK_RETURN: u16 = 0x0D;
    pub const VK_SHIFT: u16 = 0x10;
    pub const VK_CONTROL: u16 = 0x11;
    pub const VK_MENU: u16 = 0x12;
    pub const VK_PAUSE: u16 = 0x13;
    pub const VK_CAPITAL: u16 = 0x14;
    pub const VK_KANA: u16 = 0x15;
    pub const VK_HANGUL: u16 = 0x15;
    pub const VK_IME_ON: u16 = 0x16;
    pub const VK_JUNJA: u16 = 0x17;
    pub const VK_FINAL: u16 = 0x18;
    pub const VK_HANJA: u16 = 0x19;
    pub const VK_KANJI: u16 = 0x19;
    pub const VK_IME_OFF: u16 = 0x1A;
    pub const VK_ESCAPE: u16 = 0x1B;
    pub const VK_CONVERT: u16 = 0x1C;
    pub const VK_NONCONVERT: u16 = 0x1D;
    pub const VK_ACCEPT: u16 = 0x1E;
    pub const VK_MODECHANGE: u16 = 0x1F;
    pub const VK_SPACE: u16 = 0x20;
    pub const VK_PRIOR: u16 = 0x21;
    pub const VK_NEXT: u16 = 0x22;
    pub const VK_END: u16 = 0x23;
    pub const VK_HOME: u16 = 0x24;
    pub const VK_LEFT: u16 = 0x25;
    pub const VK_UP: u16 = 0x26;
    pub const VK_RIGHT: u16 = 0x27;
    pub const VK_DOWN: u16 = 0x28;
    pub const VK_SELECT: u16 = 0x29;
    pub const VK_PRINT: u16 = 0x2A;
    pub const VK_EXECUTE: u16 = 0x2B;
    pub const VK_SNAPSHOT: u16 = 0x2C;
    pub const VK_INSERT: u16 = 0x2D;
    pub const VK_DELETE: u16 = 0x2E;
    pub const VK_HELP: u16 = 0x2F;
    pub const VK_LWIN: u16 = 0x5B;
    pub const VK_RWIN: u16 = 0x5C;
    pub const VK_APPS: u16 = 0x5D;
    pub const VK_SLEEP: u16 = 0x5F;
    pub const VK_NUMPAD0: u16 = 0x60;
    pub const VK_NUMPAD1: u16 = 0x61;
    pub const VK_NUMPAD2: u16 = 0x62;
    pub const VK_NUMPAD3: u16 = 0x63;
    pub const VK_NUMPAD4: u16 = 0x64;
    pub const VK_NUMPAD5: u16 = 0x65;
    pub const VK_NUMPAD6: u16 = 0x66;
    pub const VK_NUMPAD7: u16 = 0x67;
    pub const VK_NUMPAD8: u16 = 0x68;
    pub const VK_NUMPAD9: u16 = 0x69;
    pub const VK_MULTIPLY: u16 = 0x6A;
    pub const VK_ADD: u16 = 0x6B;
    pub const VK_SEPARATOR: u16 = 0x6C;
    pub const VK_SUBTRACT: u16 = 0x6D;
    pub const VK_DECIMAL: u16 = 0x6E;
    pub const VK_DIVIDE: u16 = 0x6F;
    pub const VK_F1: u16 = 0x70;
    pub const VK_F2: u16 = 0x71;
    pub const VK_F3: u16 = 0x72;
    pub const VK_F4: u16 = 0x73;
    pub const VK_F5: u16 = 0x74;
    pub const VK_F6: u16 = 0x75;
    pub const VK_F7: u16 = 0x76;
    pub const VK_F8: u16 = 0x77;
    pub const VK_F9: u16 = 0x78;
    pub const VK_F10: u16 = 0x79;
    pub const VK_F11: u16 = 0x7A;
    pub const VK_F12: u16 = 0x7B;
    pub const VK_F13: u16 = 0x7C;
    pub const VK_F14: u16 = 0x7D;
    pub const VK_F15: u16 = 0x7E;
    pub const VK_F16: u16 = 0x7F;
    pub const VK_F17: u16 = 0x80;
    pub const VK_F18: u16 = 0x81;
    pub const VK_F19: u16 = 0x82;
    pub const VK_F20: u16 = 0x83;
    pub const VK_F21: u16 = 0x84;
    pub const VK_F22: u16 = 0x85;
    pub const VK_F23: u16 = 0x86;
    pub const VK_F24: u16 = 0x87;
    pub const VK_NUMLOCK: u16 = 0x90;
    pub const VK_SCROLL: u16 = 0x91;
    pub const VK_LSHIFT: u16 = 0xA0;
    pub const VK_RSHIFT: u16 = 0xA1;
    pub const VK_LCONTROL: u16 = 0xA2;
    pub const VK_RCONTROL: u16 = 0xA3;
    pub const VK_LMENU: u16 = 0xA4;
    pub const VK_RMENU: u16 = 0xA5;
    pub const VK_BROWSER_BACK: u16 = 0xA6;
    pub const VK_BROWSER_FORWARD: u16 = 0xA7;
    pub const VK_BROWSER_REFRESH: u16 = 0xA8;
    pub const VK_BROWSER_STOP: u16 = 0xA9;
    pub const VK_BROWSER_SEARCH: u16 = 0xAA;
    pub const VK_BROWSER_FAVORITES: u16 = 0xAB;
    pub const VK_BROWSER_HOME: u16 = 0xAC;
    pub const VK_VOLUME_MUTE: u16 = 0xAD;
    pub const VK_VOLUME_DOWN: u16 = 0xAE;
    pub const VK_VOLUME_UP: u16 = 0xAF;
    pub const VK_MEDIA_NEXT_TRACK: u16 = 0xB0;
    pub const VK_MEDIA_PREV_TRACK: u16 = 0xB1;
    pub const VK_MEDIA_STOP: u16 = 0xB2;
    pub const VK_MEDIA_PLAY_PAUSE: u16 = 0xB3;
    pub const VK_LAUNCH_MAIL: u16 = 0xB4;
    pub const VK_LAUNCH_MEDIA_SELECT: u16 = 0xB5;
    pub const VK_LAUNCH_APP1: u16 = 0xB6;
    pub const VK_LAUNCH_APP2: u16 = 0xB7;
    pub const VK_OEM_1: u16 = 0xBA;
    pub const VK_OEM_PLUS: u16 = 0xBB;
    pub const VK_OEM_COMMA: u16 = 0xBC;
    pub const VK_OEM_MINUS: u16 = 0xBD;
    pub const VK_OEM_PERIOD: u16 = 0xBE;
    pub const VK_OEM_2: u16 = 0xBF;
    pub const VK_OEM_3: u16 = 0xC0;
    pub const VK_OEM_4: u16 = 0xDB;
    pub const VK_OEM_5: u16 = 0xDC;
    pub const VK_OEM_6: u16 = 0xDD;
    pub const VK_OEM_7: u16 = 0xDE;
    pub const VK_OEM_8: u16 = 0xDF;
    pub const VK_OEM_102: u16 = 0xE2;
    pub const VK_PROCESSKEY: u16 = 0xE5;
    pub const VK_PACKET: u16 = 0xE7;
    pub const VK_ATTN: u16 = 0xF6;
    pub const VK_CRSEL: u16 = 0xF7;
    pub const VK_EXSEL: u16 = 0xF8;
    pub const VK_EREOF: u16 = 0xF9;
    pub const VK_PLAY: u16 = 0xFA;
    pub const VK_ZOOM: u16 = 0xFB;
    pub const VK_NONAME: u16 = 0xFC;
    pub const VK_PA1: u16 = 0xFD;
    pub const VK_OEM_CLEAR: u16 = 0xFE;
}

pub struct Win32Eyes {
    hwnd: HWND,
}
impl Win32Eyes {
    pub fn new(window_name: &str) -> eyre::Result<Self> {
        let cstr = CString::new(window_name).unwrap();
        let window =
            unsafe { FindWindowA(PCSTR::null(), PCSTR::from_raw(cstr.as_ptr() as *const _)) };
        if window.0 == 0 {
            bail!("Failed to find window {window_name}")
        }
        Ok(Self { hwnd: window })
    }
    pub fn make_hands(&self) -> Win32Controller {
        Win32Controller { hwnd: self.hwnd }
    }
    pub fn trect(&self) -> RECT {
        window_rect(self.hwnd)
    }
    pub fn thdc(&self) -> HDC {
        unsafe { GetWindowDC(self.hwnd) }
    }
    fn helper(
        recv_in: Receiver<Bitmap>,
        send_in: SyncSender<Bitmap>,
        send_out: SyncSender<ToBrain>,
    ) -> eyre::Result<()> {
        loop {
            let next = recv_in.recv()?;
            let res = next.to_image();
            send_out.send(ToBrain::NextFrame(res))?;
            send_in.send(next)?;
        }
    }
    fn _run(self, send: SyncSender<ToBrain>) -> eyre::Result<()> {
        let (s1, r1) = sync_channel(2);
        let (s2, r2) = sync_channel(2);
        for _ in 0..2 {
            s2.send(Bitmap::for_window(self.hwnd)?).unwrap();
        }
        let handle = spawn(move || Self::helper(r1, s2, send));
        loop {
            let mut bmp = r2.recv()?;
            let r = self.trect();
            if (r.bottom - r.top) != bmp.height() || (r.right - r.left) != bmp.width() {
                bmp = Bitmap::for_window(self.hwnd)?;
            }
            bmp.copy_from(self.trect());
            s1.send(bmp)?;
            if handle.is_finished() {
                handle
                    .join()
                    .expect("bitmap to image conversion thread failed")?;
                panic!("bitmap to image conversion thread exited");
            }
        }
    }
}
pub struct Win32Controller {
    hwnd: HWND,
}
impl Win32Controller {
    pub fn new(name: &str) -> eyre::Result<Self> {
        Ok(Self {
            hwnd: Win32Eyes::new(name)?.hwnd,
        })
    }
    pub fn move_mouse(&self, x: i32, y: i32) {
        send_input(mouse_input_list(
            x,
            y,
            window_rect(self.hwnd),
            &[MouseEventFlags::Move],
        ));
    }
    pub fn click_mouse(&self, x: i32, y: i32) {
        send_input(mouse_input_list(
            x,
            y,
            window_rect(self.hwnd),
            &[MouseEventFlags::LeftDown, MouseEventFlags::LeftUp],
        ));
    }
    pub fn cast(&mut self) {
        unsafe {
            SetForegroundWindow(self.hwnd);
        }
        let prev = unsafe { SetActiveWindow(self.hwnd) };
        send_input(vec![
            kb_input(keycodes::VK_OEM_3, KeyEventFlags::empty()),
            kb_input(keycodes::VK_OEM_3, KeyEventFlags::KeyUp),
        ]);
        unsafe { SetActiveWindow(prev) };
    }
    fn _run(mut self, input: Receiver<ToController>) -> eyre::Result<()> {
        loop {
            match input.recv()? {
                ToController::MoveMouse([x, y]) => self.move_mouse(x, y),
                ToController::PerformClick([x, y]) => self.click_mouse(x, y),
                ToController::CastHook => self.cast(),
            }
        }
    }
}

impl Controller for Win32Controller {
    fn from_window_name(name: &str) -> eyre::Result<Self> {
        Self::new(name)
    }

    fn run(self, recv: Receiver<ToController>) -> eyre::Result<()> {
        Self::_run(self, recv)
    }
}
impl Eyes for Win32Eyes {
    fn from_window_name(name: &str) -> eyre::Result<Self> {
        Self::new(name)
    }

    fn run(self, send: SyncSender<ToBrain>) -> eyre::Result<()> {
        self._run(send)
    }
}