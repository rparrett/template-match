extern crate winapi;
extern crate image;
extern crate imageproc;
extern crate conv;
extern crate libc;

use libc::c_char;

use winapi::um::winuser::{GetDesktopWindow, GetClientRect, GetDC, ReleaseDC};
use winapi::um::wingdi::{CreateCompatibleDC, CreateCompatibleBitmap, DeleteObject, DeleteDC,
                         SelectObject, BitBlt, GetDIBits, SRCCOPY, BITMAPINFO, BITMAPINFOHEADER,
                         BI_RGB, DIB_RGB_COLORS, RGBQUAD};
use winapi::shared::windef::{HBITMAP, HDC, HGDIOBJ, HWND, RECT};
use winapi::_core::ptr::null_mut;
use winapi::ctypes::c_void;

use std::mem::zeroed;
use std::mem::size_of;

use std::ffi::CStr;

use image::{imageops, GenericImage, Pixel, Primitive, ImageBuffer};
use conv::ValueInto;

struct Screenshot {
    data: Vec<RGBQUAD>,
    w: u32,
    h: u32,
}

impl From<Screenshot> for ImageBuffer<image::Rgb<u8>, std::vec::Vec<u8>> {
    fn from(screenshot: Screenshot) -> Self {
        ImageBuffer::from_fn(screenshot.w, screenshot.h, |x, y| {
            let i = y * screenshot.w + x;

            Pixel::from_channels(
                screenshot.data[i as usize].rgbRed,
                screenshot.data[i as usize].rgbGreen,
                screenshot.data[i as usize].rgbBlue,
                255u8,
            )
        })
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct TemplateMatchResult {
    x: u32,
    y: u32,
    rms: f64,
}

fn get_screenshot() -> Result<Screenshot, &'static str> {
    unsafe {
        let hwnd_desktop: HWND = GetDesktopWindow();

        let mut rect = zeroed::<RECT>();

        let result = GetClientRect(hwnd_desktop, &mut rect);
        if result == 0 {
            // TODO GetLastError

            return Err("GetClientRect failed.");
        }

        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;

        let h_screen_dc: HDC = GetDC(hwnd_desktop);

        let h_capture_dc = CreateCompatibleDC(h_screen_dc);

        let h_capture_bitmap: HBITMAP = CreateCompatibleBitmap(h_screen_dc, w, h);

        let h_old_gdiobj: HGDIOBJ = SelectObject(h_capture_dc, h_capture_bitmap as *mut c_void);

        let result = BitBlt(
            h_capture_dc,
            0,
            0,
            w,
            h,
            h_screen_dc,
            rect.left,
            rect.top,
            SRCCOPY,
        );
        if result == 0 {
            // TODO GetLastError?

            SelectObject(h_capture_dc, h_old_gdiobj);
            DeleteDC(h_capture_dc);
            ReleaseDC(null_mut(), h_screen_dc);
            DeleteObject(h_capture_bitmap as *mut c_void);

            return Err("BitBlt failed.");
        }

        let mut buf = vec![0u8; size_of::<BITMAPINFO>()];
        let bmi = &mut *(buf.as_mut_ptr() as *mut BITMAPINFO);

        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -1 * h;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut quads = Vec::<RGBQUAD>::with_capacity((w * h) as usize);
        for _ in 0..(w * h) {
            quads.push(RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            })
        }

        let result = GetDIBits(
            h_capture_dc,
            h_capture_bitmap,
            0,
            h as u32,
            quads.as_mut_ptr() as *mut c_void,
            bmi,
            DIB_RGB_COLORS,
        );
        if result < 1 {
            // TODO GetLastError?

            return Err("GetDIBits failed.");
        }

        // cleanup

        SelectObject(h_capture_dc, h_old_gdiobj);
        DeleteDC(h_capture_dc);
        ReleaseDC(null_mut(), h_screen_dc);
        DeleteObject(h_capture_bitmap as *mut c_void);

        Ok(Screenshot {
            data: quads,
            w: w as u32,
            h: h as u32,
        })
    }
}

fn template_match_images<I: 'static, P, S>(haystack: &mut I, needle: &I) -> (u32, u32, f64)
where
    I: GenericImage<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
    P::Subpixel: ValueInto<f64>,
{
    let mut min_rms = std::f64::MAX;
    let mut min_x = 0;
    let mut min_y = 0;

    let h_w = haystack.width();
    let h_h = haystack.height();
    let n_w = needle.width();
    let n_h = needle.height();

    for x in 0..(h_w - n_w + 1) {
        for y in 0..(h_h - n_h + 1) {
            let subimg = imageops::crop(haystack, x, y, n_w, n_h);

            let rms = imageproc::stats::root_mean_squared_error(&subimg, needle);

            if rms < min_rms {
                min_rms = rms;
                min_x = x;
                min_y = y;
            }
        }
    }

    (min_x, min_y, min_rms)
}

#[no_mangle]
pub extern "stdcall" fn template_match(
    raw_filename: *const c_char,
    raw_result: *mut TemplateMatchResult,
) -> u32 {
    if raw_result.is_null() {
        return 1;
    }

    let result = unsafe { &mut *raw_result };

    if raw_filename.is_null() {
        return 2;
    }

    let c_str = unsafe { CStr::from_ptr(raw_filename) };

    let filename = match c_str.to_str() {
        Ok(f) => f,
        Err(_) => return 3,
    };

    let s = match get_screenshot() {
        Ok(s) => s,
        Err(_) => return 4,
    };

    let mut haystack: ImageBuffer<image::Rgb<u8>, std::vec::Vec<u8>> = s.into();

    let needle = match image::open(filename) {
        Ok(needle) => needle,
        Err(_) => return 5,
    };

    let needle = needle.to_rgb();

    let res = template_match_images(&mut haystack, &needle);

    result.x = res.0;
    result.y = res.1;
    result.rms = res.2;

    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_template_match() {
        use super::*;
        use std::ffi::CString;

        // Will fail if the test pattern is open in an image editor on your
        // screen!

        let mut r = TemplateMatchResult {
            x: 0,
            y: 0,
            rms: 0.0f64,
        };

        let f = CString::new("resources/test-needle.png").unwrap();
        let f_ptr = f.as_ptr();

        let res = template_match(f_ptr, &mut r);

        assert!(res == 0);
        assert!(r.rms > 0.0f64);
    }

    #[test]
    fn test_template_match_images() {
        use super::*;

        let mut haystack = image::open("resources/test-haystack.png").unwrap();
        let needle = image::open("resources/test-needle.png").unwrap();

        let res = template_match_images(&mut haystack, &needle);

        assert!(res == (32, 24, 0.0f64));
    }

    #[test]
    fn test_template_match_images_bottomright() {
        use super::*;

        let mut haystack = image::open("resources/test-haystack-bottomright.png").unwrap();
        let needle = image::open("resources/test-needle.png").unwrap();

        let res = template_match_images(&mut haystack, &needle);

        assert!(res == (54, 41, 0.0f64));
    }

    #[test]
    fn test_template_match_images_fuzzy() {
        use super::*;

        let mut haystack = image::open("resources/test-haystack-fuzzy.png").unwrap();
        let needle = image::open("resources/test-needle.png").unwrap();

        let res = template_match_images(&mut haystack, &needle);

        // Fuzzy test haystack has 4 wrong pixels

        let mut sum_squared_diffs = 0f64;
        sum_squared_diffs += (255f64 * 255f64 * 3f64) + // White - Black
            (255f64 * 255f64 * 2f64) + // White - Red                        
            (255f64 * 255f64 * 2f64) + // White - Green
            (255f64 * 255f64 * 2f64); // White - Blue

        let count = (needle.width() * needle.height() * 4) as f64;
        let rms = (sum_squared_diffs / count).sqrt();

        assert!(res == (32, 24, rms));
    }

    #[test]
    fn test_template_match_images_same() {
        use super::*;

        let mut haystack = image::open("resources/test-needle.png").unwrap();
        let needle = image::open("resources/test-needle.png").unwrap();

        let res = template_match_images(&mut haystack, &needle);

        assert!(res == (0, 0, 0.0f64));
    }
}
