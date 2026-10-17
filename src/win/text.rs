//! Text for the software canvas. GDI draws white text on a black DIB; the green channel is then
//! the anti-aliased coverage of each pixel. Using GDI keeps DirectWrite/Direct2D (and their
//! memory) out of the process.

use std::ffi::c_void;

use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLIP_DEFAULT_PRECIS,
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DEFAULT_CHARSET, DIB_RGB_COLORS, DT_CENTER,
    DT_LEFT, DT_NOCLIP, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, FF_DONTCARE,
    FW_SEMIBOLD, HGDIOBJ, OUT_DEFAULT_PRECIS, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
    VARIABLE_PITCH,
};
use windows::core::w;

use crate::canvas::Align;

/// Render `text` centred in a `w` x `h` coverage mask using a Segoe UI font of `font_px` pixels
/// (device pixels, so the caller scales for DPI). Returns `None` if GDI fails.
pub fn render_mask(text: &str, w: u32, h: u32, font_px: i32) -> Option<Vec<u8>> {
    render_mask_aligned(text, w, h, font_px, Align::Center)
}

/// Like [`render_mask`], with the text against the left edge or in the middle.
pub fn render_mask_aligned(
    text: &str,
    w: u32,
    h: u32,
    font_px: i32,
    align: Align,
) -> Option<Vec<u8>> {
    if w == 0 || h == 0 || text.is_empty() {
        return Some(vec![0; (w as usize) * (h as usize)]);
    }
    // SAFETY: plain GDI calls on objects created and released in this function; the DIB pointer
    // is only read while the bitmap is alive and is `w * h` u32s as requested below.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: -(h as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0);
        let font = CreateFontW(
            -font_px,
            0,
            0,
            0,
            FW_SEMIBOLD.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            (VARIABLE_PITCH.0 | FF_DONTCARE.0) as u32,
            w!("Segoe UI"),
        );

        let mut mask = None;
        if let Ok(bitmap) = bitmap
            && !bits.is_null()
        {
            let old_bitmap = SelectObject(dc, HGDIOBJ(bitmap.0));
            let old_font = SelectObject(dc, HGDIOBJ(font.0));
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, COLORREF(0x00ff_ffff));
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: w as i32,
                bottom: h as i32,
            };
            DrawTextW(
                dc,
                &mut wide,
                &mut rect,
                match align {
                    Align::Center => DT_CENTER,
                    Align::Left => DT_LEFT,
                } | DT_VCENTER
                    | DT_SINGLELINE
                    | DT_NOCLIP,
            );

            let pixels =
                std::slice::from_raw_parts(bits as *const u32, (w as usize) * (h as usize));
            mask = Some(pixels.iter().map(|p| ((p >> 8) & 0xff) as u8).collect());

            SelectObject(dc, old_font);
            SelectObject(dc, old_bitmap);
        }
        let _ = DeleteObject(HGDIOBJ(font.0));
        if let Ok(bitmap) = bitmap {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
        }
        let _ = DeleteDC(dc);
        mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_produces_coverage_inside_the_mask() {
        let mask = render_mask("12:34", 120, 40, 24).expect("GDI text");
        assert_eq!(mask.len(), 120 * 40);
        let inked = mask.iter().filter(|&&m| m > 0).count();
        assert!(inked > 100, "only {inked} inked pixels");
        assert!(
            mask.contains(&255) || mask.iter().any(|&m| m > 200),
            "no strong pixels"
        );
        // Anti-aliased: there are partial coverage values too.
        assert!(mask.iter().any(|&m| (1..200).contains(&m)));
    }

    #[test]
    fn text_is_centred() {
        let (w, h) = (200usize, 40usize);
        let mask = render_mask("00:00", w as u32, h as u32, 24).unwrap();
        let columns: Vec<usize> = (0..w)
            .filter(|&x| (0..h).any(|y| mask[y * w + x] > 0))
            .collect();
        let rows: Vec<usize> = (0..h)
            .filter(|&y| (0..w).any(|x| mask[y * w + x] > 0))
            .collect();
        let (left, right) = (columns[0], w - 1 - columns[columns.len() - 1]);
        let (top, bottom) = (rows[0], h - 1 - rows[rows.len() - 1]);
        assert!(
            left.abs_diff(right) <= 3,
            "left margin {left}, right margin {right}"
        );
        assert!(
            top.abs_diff(bottom) <= 8,
            "top margin {top}, bottom margin {bottom}"
        );
    }

    #[test]
    fn larger_font_covers_more() {
        let ink = |px| {
            render_mask("88:88", 200, 60, px)
                .unwrap()
                .iter()
                .filter(|&&m| m > 0)
                .count()
        };
        assert!(ink(32) > ink(16));
    }

    #[test]
    fn empty_text_and_empty_sizes_are_fine() {
        assert_eq!(render_mask("", 10, 10, 12).unwrap(), vec![0; 100]);
        assert_eq!(render_mask("x", 0, 10, 12).unwrap(), Vec::<u8>::new());
    }
}
