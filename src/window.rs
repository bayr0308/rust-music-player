pub fn set_always_on_top(ctx: &eframe::egui::Context, enabled: bool) {
    use eframe::egui::{ViewportCommand, ViewportId, WindowLevel};

    ctx.send_viewport_cmd_to(
        ViewportId::ROOT,
        ViewportCommand::WindowLevel(if enabled {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        }),
    );
}

/// Ask Windows to compose real desktop acrylic behind the transparent framebuffer.
/// A solid surface is used if the operating system does not support the material.
#[cfg(windows)]
pub fn acrylic(cc: &eframe::CreationContext<'_>) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{Graphics::Dwm::*, UI::Controls::MARGINS};
    let Ok(handle) = cc.window_handle() else {
        return false;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return false;
    };
    let hwnd = handle.hwnd.get() as _;
    let dark: i32 = 1;
    let corners = DWMWCP_ROUND;
    let backdrop = DWMSBT_TRANSIENTWINDOW;
    let margins = MARGINS {
        cxLeftWidth: -1,
        cxRightWidth: -1,
        cyTopHeight: -1,
        cyBottomHeight: -1,
    };
    // All pointers refer to initialized values of exactly the type/size required
    // by DWM. The HWND is borrowed from eframe on its owning UI thread.
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            (&dark as *const i32).cast(),
            size_of_val(&dark) as u32,
        );
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            (&corners as *const i32).cast(),
            size_of_val(&corners) as u32,
        );
        let extended = DwmExtendFrameIntoClientArea(hwnd, &margins);
        let applied = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE as u32,
            (&backdrop as *const i32).cast(),
            size_of_val(&backdrop) as u32,
        );
        extended >= 0 && applied >= 0
    }
}

#[cfg(not(windows))]
pub fn acrylic(_cc: &eframe::CreationContext<'_>) -> bool {
    false
}
