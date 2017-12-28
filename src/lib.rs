extern crate winapi;
extern crate image;
extern crate imageproc;
extern crate conv;

use winapi::um::winuser::{GetDesktopWindow, GetClientRect, GetDC, ReleaseDC};
use winapi::um::wingdi::{CreateCompatibleDC, CreateCompatibleBitmap, DeleteObject, DeleteDC, SelectObject, BitBlt, GetDIBits, SRCCOPY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, RGBQUAD};
use winapi::shared::windef::{HBITMAP, HDC, HGDIOBJ, HWND, RECT};
use winapi::_core::ptr::null_mut;
use winapi::ctypes::c_void;

use std::mem::zeroed;
use std::mem::size_of;

use image::{imageops, GenericImage, Pixel, Primitive, ImageBuffer};
use conv::ValueInto;

struct Screenshot {
    data: Vec<RGBQUAD>,
    w: u32,
    h: u32
}

impl From<Screenshot> for ImageBuffer<image::Rgb<u8>, std::vec::Vec<u8>> {
    fn from(screenshot: Screenshot) -> Self {
        ImageBuffer::from_fn(screenshot.w, screenshot.h, |x, y| {
            let i = y * screenshot.w + x;

            Pixel::from_channels(
                screenshot.data[i as usize].rgbRed,
                screenshot.data[i as usize].rgbGreen,
                screenshot.data[i as usize].rgbBlue,
                255u8
            )
        })        
    }
}

#[derive(Debug)]
pub struct FindColorResult {
    x: u32,
    y: u32,
    err: String
}

#[derive(Debug)]
pub struct TemplateMatchResult {
    x: u32,
    y: u32,
    rms: f64,
    err: String
}

fn get_screenshot() -> Result<Screenshot, &'static str> {
    unsafe {
        let hwnd_desktop: HWND = GetDesktopWindow();
        
        let mut rect = zeroed::<RECT>();

        let result = GetClientRect(hwnd_desktop, &mut rect);
        if result == 0 {
            // TODO GetLastError
            
            return Err("GetClientRect failed.")
        }

        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;

        let h_screen_dc: HDC = GetDC(hwnd_desktop);

        let h_capture_dc = CreateCompatibleDC(h_screen_dc);

        let h_capture_bitmap: HBITMAP = CreateCompatibleBitmap(h_screen_dc, w, h);

        let h_old_gdiobj: HGDIOBJ = SelectObject(h_capture_dc, h_capture_bitmap as *mut c_void);

        let result = BitBlt(h_capture_dc, 0, 0, w, h, h_screen_dc, rect.left, rect.top, SRCCOPY);
        if result == 0 {
            // TODO GetLastError
            
            SelectObject(h_capture_dc, h_old_gdiobj);
            DeleteDC(h_capture_dc);
            ReleaseDC(null_mut(), h_screen_dc);
            DeleteObject(h_capture_bitmap as *mut c_void);

            return Err("BitBlt failed.")
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
                rgbReserved: 0
            })
        }

        let result = GetDIBits(h_capture_dc, h_capture_bitmap, 0, h as u32, quads.as_mut_ptr() as *mut c_void, bmi, DIB_RGB_COLORS);
        if result < 1 {
            // TODO GetLastError?
            
            return Err("GetDIBits failed.")
        }

        // cleanup

        SelectObject(h_capture_dc, h_old_gdiobj);
        DeleteDC(h_capture_dc);
        ReleaseDC(null_mut(), h_screen_dc);
        DeleteObject(h_capture_bitmap as *mut c_void);

        Ok(Screenshot {
            data: quads,
            w: w as u32,
            h: h as u32
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
    let mut min = std::f64::MAX;
    let mut minx = 0;
    let mut miny = 0;

    let hw = (*haystack).width();
    let hh = (*haystack).height();
    let nw = (*needle).width();
    let nh = (*needle).height();

    for x in 0..(hw - nw) {
        for y in 0..(hh - nh) {
            let subimg = imageops::crop(haystack, x, y, nw, nh);

            let rms = imageproc::stats::root_mean_squared_error(&subimg, needle);

            if rms < min {
                min = rms;
                minx = x;
                miny = y;
            }
        }
    }

    (minx, miny, min)
}

#[no_mangle]
pub extern "stdcall" fn template_match(filename: &'static str) -> TemplateMatchResult {
    let s = get_screenshot();

    let s = match s {
        Ok(s) => s,
        Err(e) => {
            return TemplateMatchResult {
                x: 0,
                y: 0,
                rms: 0.0f64,
                err: format!("Failed to get screenshot: {}", e).into()
            }            
        }
    };

    let mut haystack: ImageBuffer<image::Rgb<u8>, std::vec::Vec<u8>> = s.into();

    let needle = image::open(filename);

    let needle = match needle {
        Ok(needle) => needle,
        _ => {
            return TemplateMatchResult {
                x: 0,
                y: 0,
                rms: 0.0f64,
                err: "Failed to open needle.".into()
            }               
        }
    };

    let needle = needle.to_rgb();

    let res = template_match_images(&mut haystack, &needle);

    TemplateMatchResult {
        x: res.0,
        y: res.1,
        rms: res.2,
        err: "".into()
    }    
}

#[no_mangle]
pub extern "stdcall" fn find_color(r: u8, b: u8, g: u8, _t: u8) -> FindColorResult {
    let s = get_screenshot();

    let s = match s {
        Ok(s) => s,
        Err(e) => {
            return FindColorResult {
                x: 0,
                y: 0,
                err: format!("Failed to get screenshot: {}", e).into()
            }            
        }
    };

    for i in 0..(s.w * s.h) {
        if s.data[i as usize].rgbRed == r && s.data[i as usize].rgbBlue == b && s.data[i as usize].rgbGreen == g {
            let x = i % s.w;
            let y = (i - x) / s.w;

            return FindColorResult { x: x as u32, y: y as u32, err: "".into()};
        }
    }

    FindColorResult {
        x: 0,
        y: 0,
        err: "Not found.".into()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_find_color() {
        use super::*;

        let c = find_color(0, 0, 0, 0);

        println!("{:?}", c);
    }

    #[test]
    fn test_template_match() {
        use super::*;

        let c = template_match("wayoff.png");

        println!("{:?}", c);
    }
}
