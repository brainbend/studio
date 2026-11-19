//! Where the close, minimize and zoom buttons sit on macOS. The window has no title bar of its own (the app draws one), so the buttons float
//! over the top bar, and are placed in it to line up with the bar's own contents: an inset from the left edge, and centered in the bar's height.
//!
//! AppKit puts the buttons back where it wants them when the window is resized or goes in and out of full screen, so the place is asked for
//! again every frame; a button that is already there is left alone.
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindow, NSWindowButton};
use objc2_foundation::NSPoint;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// How far in from the left edge the first button is.
pub(crate) const INSET: f64 = 14.0;

thread_local! {
    /// Where the second and third buttons sit to the right of the first, as macOS first put them: its own spacing is kept.
    static OFFSETS: std::cell::Cell<Option<[f64; 3]>> = const { std::cell::Cell::new(None) };
}

/// Put the three buttons `inset` from the left edge of the window and centered in a bar `bar_height` tall at the top of it.
pub(crate) fn place_buttons(frame: &eframe::Frame, inset: f64, bar_height: f64) {
    let Ok(handle) = frame.window_handle() else { return };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else { return };
    // SAFETY: the handle is the window's view, which lives as long as the window the frame belongs to; this runs on the main thread.
    let view: &NSView = unsafe { h.ns_view.cast::<NSView>().as_ref() };
    let Some(window): Option<Retained<NSWindow>> = view.window() else { return };
    // Full screen has no bar to line up with.
    if window.styleMask().contains(objc2_app_kit::NSWindowStyleMask::FullScreen) { return; }
    let kinds = [NSWindowButton::CloseButton, NSWindowButton::MiniaturizeButton, NSWindowButton::ZoomButton];
    let offsets = match OFFSETS.get() {
        Some(o) => o,
        None => {
            // The first time, before any button has been moved, the buttons are where the system put them.
            let xs: Vec<f64> = kinds.iter().filter_map(|k| window.standardWindowButton(*k)).map(|b| b.frame().origin.x).collect();
            if xs.len() < 3 { return; }
            let o = [0.0, xs[1] - xs[0], xs[2] - xs[0]];
            OFFSETS.set(Some(o));
            o
        }
    };
    for (i, kind) in kinds.into_iter().enumerate() {
        let Some(button) = window.standardWindowButton(kind) else { continue };
        let Some(holder) = (unsafe { button.superview() }) else { continue };
        let (size, holder_h) = (button.frame().size, holder.frame().size.height);
        // The holder sits at the top of the window; its own origin is at its bottom left.
        let want = NSPoint::new(inset + offsets[i], holder_h - bar_height / 2.0 - size.height / 2.0);
        let now = button.frame().origin;
        if (now.x - want.x).abs() > 0.5 || (now.y - want.y).abs() > 0.5 { unsafe { button.setFrameOrigin(want) }; }
    }
}
