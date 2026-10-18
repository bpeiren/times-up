//! The system colour dialog, for choosing a colour of the bar in the Settings window.

use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::UI::Controls::Dialogs::{CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW, ChooseColorW};

use crate::config::Rgba;

/// Ask for a colour, starting from `current`. The opacity is not part of the dialog and stays as
/// it was. `None` when the teacher cancels.
pub fn pick_color(owner: HWND, current: Rgba) -> Option<Rgba> {
    let mut custom = [COLORREF(0x00ff_ffff); 16];
    let mut dialog = CHOOSECOLORW {
        lStructSize: size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: owner,
        rgbResult: COLORREF(
            u32::from(current.r) | u32::from(current.g) << 8 | u32::from(current.b) << 16,
        ),
        lpCustColors: custom.as_mut_ptr(),
        Flags: CC_RGBINIT | CC_FULLOPEN,
        ..Default::default()
    };
    // SAFETY: `dialog` and the custom colours outlive this modal call.
    if !unsafe { ChooseColorW(&mut dialog) }.as_bool() {
        return None;
    }
    let c = dialog.rgbResult.0;
    Some(Rgba {
        r: c as u8,
        g: (c >> 8) as u8,
        b: (c >> 16) as u8,
        a: current.a,
    })
}
