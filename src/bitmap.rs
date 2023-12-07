use image::{Rgb, RgbImage};
use std::{mem::size_of, ptr::null_mut};

use windows::Win32::{
    Foundation::{HANDLE, HWND, RECT},
    Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, CAPTUREBLT,
        DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, SRCCOPY,
    },
    UI::WindowsAndMessaging::{GetDesktopWindow, GetWindowRect},
};

pub struct Bitmap {
    info: BITMAPINFO,
    handle: HBITMAP,
    output_hdc: DeleteHDC,
    desktop_hdc: ReleaseHDC
}

impl Bitmap {
    pub fn for_window(hwnd: HWND) -> eyre::Result<Self> {
        unsafe {
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect)?;
            Self::new(rect)
        }
    }
    pub fn new(rect: RECT) -> eyre::Result<Self> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: rect.right - rect.left,
                biHeight: rect.bottom - rect.top,
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut data_ptr = null_mut();
        let dwin = unsafe { GetDesktopWindow() };
        let desktop_hdc = ReleaseHDC::from_hwnd(dwin);
        println!("{:?}", info);
        let handle = unsafe {
            CreateDIBSection(desktop_hdc.1, &info, DIB_RGB_COLORS, &mut data_ptr, HANDLE(0), 0).unwrap()
        };

        Ok(Self {
            info,
            handle,
            output_hdc: unsafe { DeleteHDC(CreateCompatibleDC(desktop_hdc.1)) },
            desktop_hdc
        })
    }
    pub fn select_swap(&self, hdc: HDC) -> HGDIOBJ {
        unsafe { SelectObject(hdc, self.handle) }
    }
    pub fn select_into(&self, hdc: HDC) -> SelectionLock {
        SelectionLock {
            old: self.select_swap(hdc),
            hdc,
        }
    }
    pub fn width(&self) -> i32 {
        self.info.bmiHeader.biWidth
    }
    pub fn height(&self) -> i32 {
        self.info.bmiHeader.biHeight
    }
    pub fn copy_from(&self, source_rect: RECT) {
        unsafe {
            let _selection = self.select_into(self.output_hdc.0);
            BitBlt(
                self.output_hdc.0,
                0,
                0,
                self.width(),
                self.height(),
                self.desktop_hdc.1,
                source_rect.left,
                source_rect.top,
                SRCCOPY | CAPTUREBLT,
            )
            .unwrap();
            GdiFlush();
        }
    }
    pub fn to_image(&self) -> RgbImage {
        let w = self.width() as u32;
        let h = self.height() as u32;
        let _sel = self.select_into(self.output_hdc.0);
        let mut buf = vec![0u8; w as usize * h as usize * 4];
        unsafe {
            GetDIBits(
                self.output_hdc.0,
                self.handle,
                0,
                h,
                Some(buf.as_mut_ptr() as *mut _),
                &self.info as *const _ as *mut _,
                DIB_RGB_COLORS,
            )
        };
        RgbImage::from_fn(w, h, |x, y| {
            let offset = (x + (h - 1 - y) * w as u32) as usize * 4;
            let r = buf[offset + 2];
            let g = buf[offset + 1];
            let b = buf[offset + 0];
            Rgb([r, g, b])
        })
    }
}
impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            assert!(DeleteObject(self.handle).as_bool());
        }
    }
}

pub struct SelectionLock {
    old: HGDIOBJ,
    hdc: HDC,
}
impl Drop for SelectionLock {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.hdc, self.old);
        }
    }
}
pub struct ReleaseHDC(HWND, HDC);
impl ReleaseHDC {
    pub fn from_hwnd(hwnd: HWND) -> Self {
        Self(hwnd, unsafe { GetDC(hwnd) })
    }
}
impl Drop for ReleaseHDC {
    fn drop(&mut self) {
        unsafe {
            assert_eq!(ReleaseDC(self.0, self.1), 1);
        }
    }
}
pub struct DeleteHDC(HDC);
impl Drop for DeleteHDC {
    fn drop(&mut self) {
        unsafe {
            assert!(DeleteDC(self.0).as_bool());
        }
    }
}
impl From<DeleteHDC> for HDC {
    fn from(value: DeleteHDC) -> Self {
        value.0
    }
}
