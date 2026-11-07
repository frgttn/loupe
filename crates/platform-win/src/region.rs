//! Picking a part of the screen: a still of every display, dimmed, over
//! everything, where the user drags out a rectangle. Under it a bar asks
//! what to do with it (Copy Text, Scan QR, Google Lens), unless that was decided
//! before. Plain Win32 on a thread of its own, so the launcher's window and
//! UI stay as they are: GDI copies the still, Direct2D and DirectWrite draw
//! the selection's frame, its size and the bar (smooth corners and text, in
//! the launcher's dark theme).

use std::{ffi::c_void, mem::size_of, ptr, thread};

use image::RgbaImage;
use loupe_core::item::RegionAction;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::{
            Direct2D::{
                Common::{D2D_RECT_F, D2D1_ALPHA_MODE_IGNORE, D2D1_COLOR_F, D2D1_PIXEL_FORMAT},
                D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
                D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
                D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
                D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory, ID2D1DCRenderTarget,
                ID2D1Factory, ID2D1SolidColorBrush,
            },
            DirectWrite::{
                DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_MEDIUM, DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_TEXT_METRICS, DWRITE_WORD_WRAPPING_NO_WRAP,
                DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection, IDWriteTextFormat,
                IDWriteTextLayout,
            },
            Dwm::{DWMWA_TRANSITIONS_FORCEDISABLED, DwmSetWindowAttribute},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::{
                AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, BeginPaint,
                BitBlt, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC,
                DeleteObject, EndPaint, GdiAlphaBlend, GdiFlush, GetDC, GetMonitorInfoW, HBITMAP,
                HDC, HGDIOBJ, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO,
                MonitorFromPoint, PAINTSTRUCT, ReleaseDC, SRCCOPY, SelectObject,
            },
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
                SetThreadDpiAwarenessContext,
            },
            Input::KeyboardAndMouse::{
                GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_ESCAPE, VK_RETURN,
            },
            WindowsAndMessaging::{
                CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW,
                DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetMessageW, GetSystemMetrics,
                GetWindowLongPtrW, HTCLIENT, IDC_CROSS, IDC_HAND, LoadCursorW, MSG,
                RegisterClassExW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
                SM_YVIRTUALSCREEN, SW_SHOW, SetCursor, SetWindowLongPtrW, ShowWindow,
                TranslateMessage, WA_INACTIVE, WM_ACTIVATE, WM_CLOSE, WM_DPICHANGED, WM_ERASEBKGND,
                WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCCREATE, WM_PAINT,
                WM_RBUTTONDOWN, WM_SETCURSOR, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
                WS_POPUP,
            },
        },
    },
    core::{BOOL, HSTRING, PCWSTR, w},
};
use windows_numerics::{Matrix3x2, Vector2};

/// The part of the screen picked, as captured, and what to do with it.
pub struct Picked {
    pub image: RgbaImage,
    pub action: RegionAction,
}

/// Takes a still of every display as they are now, shows it and lets the
/// user pick a part of it. With `preset`, releasing the mouse is the end of
/// it; without, a bar asks. `None` arrives when they cancel (Esc, right
/// click, switching away).
pub fn pick(preset: Option<RegionAction>) -> async_channel::Receiver<Option<Picked>> {
    let (tx, rx) = async_channel::bounded(1);
    thread::spawn(move || {
        let picked = unsafe { run(preset) };
        if let Err(err) = &picked {
            eprintln!("[region] {err}");
        }
        let _ = tx.send_blocking(picked.ok().flatten());
    });
    rx
}

/// How the picker looks over `screen` with `selection` made and the bar's
/// button `hover` under the pointer, without a window: for checking the
/// design. Measured as on the display at the screen's top left corner.
pub fn preview(
    screen: RgbaImage,
    selection: RECT,
    hover: Option<usize>,
) -> Result<RgbaImage, String> {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let mut state = State::new(Some(&screen), None)?;
        state.selection = Some(selection);
        state.hover = hover;
        let (width, height) = (state.width, state.height);
        compose(
            &mut state,
            RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            },
        );
        let _ = GdiFlush();
        let mut rgba =
            std::slice::from_raw_parts(state.back.bits, (width * height * 4) as usize).to_vec();
        for px in rgba.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
            px[3] = 255;
        }
        RgbaImage::from_raw(width as u32, height as u32, rgba).ok_or("bad preview size".into())
    }
}

const CLASS: PCWSTR = w!("LoupeRegionPicker");
/// Smaller than this (physical pixels) on both sides is a click, not a region.
const MIN_SIDE: i32 = 4;
/// How much the black laid over the still outside the selection hides of
/// it, out of 255.
const DIM: u8 = 140;

/// The launcher's dark theme (`theme.rs`, WinUI's Fluent tokens), `0xRRGGBBAA`.
mod color {
    /// AccentFillColorDefault.
    pub const ACCENT: u32 = 0x60cdffff;
    /// SolidBackgroundFillColorQuarternary: a flyout.
    pub const PANEL: u32 = 0x2c2c2cff;
    /// The flyout's outline.
    pub const PANEL_BORDER: u32 = 0xffffff1c;
    pub const HOVER: u32 = 0xffffff14;
    pub const PRESSED: u32 = 0xffffff0c;
    pub const DIVIDER: u32 = 0xffffff15;
    pub const TEXT: u32 = 0xffffffff;
    pub const TEXT_MUTED: u32 = 0xffffffc5;
    pub const KEYCAP: u32 = 0xffffff0f;
    pub const KEYCAP_BORDER: u32 = 0xffffff18;
    pub const KEYCAP_TEXT: u32 = 0xffffff9e;
    pub const LABEL: u32 = 0x2c2c2ce8;
}

struct Button {
    glyph: char,
    label: &'static str,
    key: &'static str,
    action: RegionAction,
}

/// The bar's buttons, in order.
const BUTTONS: [Button; 3] = [
    Button {
        // Copy.
        glyph: '\u{E8C8}',
        label: "Copy Text",
        key: "T",
        action: RegionAction::CopyText,
    },
    Button {
        // QR code.
        glyph: '\u{ED14}',
        label: "Scan QR",
        key: "Q",
        action: RegionAction::ScanQrCode,
    },
    Button {
        // Search.
        glyph: '\u{E721}',
        label: "Google Lens",
        key: "L",
        action: RegionAction::GoogleLens,
    },
];

/// Sizes in pixels at 100% scale.
mod size {
    pub const LABEL_FONT: f32 = 13.;
    pub const KEY_FONT: f32 = 11.;
    pub const ICON_FONT: f32 = 14.;
    pub const SIZE_FONT: f32 = 11.5;
    pub const BAR_PAD: f32 = 4.;
    pub const BAR_RADIUS: f32 = 8.;
    pub const BAR_GAP: f32 = 8.;
    pub const BUTTON_HEIGHT: f32 = 34.;
    pub const BUTTON_RADIUS: f32 = 5.;
    pub const BUTTON_PAD_LEFT: f32 = 10.;
    pub const BUTTON_PAD_RIGHT: f32 = 7.;
    pub const ICON_GAP: f32 = 8.;
    pub const KEY_GAP: f32 = 12.;
    pub const KEYCAP: f32 = 20.;
    pub const KEYCAP_RADIUS: f32 = 4.;
    pub const DIVIDER_GAP: f32 = 4.;
    pub const HANDLE: f32 = 14.;
    pub const HANDLE_WIDTH: f32 = 3.;
    pub const SIZE_PAD_X: f32 = 7.;
    pub const SIZE_HEIGHT: f32 = 20.;
    pub const SIZE_GAP: f32 = 8.;
}

/// A screen-sized bitmap selected into a memory DC.
struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *const u8,
}

impl Surface {
    /// From BGRA pixels, top row first; blank (black) without.
    unsafe fn new(
        screen: HDC,
        width: i32,
        height: i32,
        bgra: Option<&[u8]>,
    ) -> Result<Self, String> {
        unsafe {
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut c_void = ptr::null_mut();
            let bitmap = CreateDIBSection(Some(screen), &info, DIB_RGB_COLORS, &mut bits, None, 0)
                .map_err(|err| format!("Couldn't make a bitmap: {err}"))?;
            if bits.is_null() {
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
                return Err("Couldn't make a bitmap".into());
            }
            if let Some(bgra) = bgra {
                ptr::copy_nonoverlapping(bgra.as_ptr(), bits.cast::<u8>(), bgra.len());
            }
            let dc = CreateCompatibleDC(Some(screen));
            let old = SelectObject(dc, HGDIOBJ(bitmap.0));
            Ok(Self {
                dc,
                bitmap,
                old,
                bits: bits.cast(),
            })
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
            let _ = DeleteDC(self.dc);
        }
    }
}

/// Direct2D drawing onto the back buffer, and DirectWrite's text.
struct Graphics {
    d2d: ID2D1Factory,
    target: ID2D1DCRenderTarget,
    write: IDWriteFactory,
    ui_font: HSTRING,
    icon_font: HSTRING,
    /// The text formats at a scale (a display's DPI), made again when it changes.
    formats: Option<(u32, Formats)>,
}

struct Formats {
    label: IDWriteTextFormat,
    key: IDWriteTextFormat,
    icon: IDWriteTextFormat,
    size: IDWriteTextFormat,
}

impl Graphics {
    unsafe fn new() -> Result<Self, String> {
        unsafe {
            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
                .map_err(|err| format!("Couldn't start Direct2D: {err}"))?;
            let target = Self::target(&d2d)?;
            let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
                .map_err(|err| format!("Couldn't start DirectWrite: {err}"))?;
            let mut fonts: Option<IDWriteFontCollection> = None;
            let _ = write.GetSystemFontCollection(&mut fonts, false);
            let installed = |name: &str| {
                let (mut index, mut exists) = (0, BOOL(0));
                fonts.as_ref().is_some_and(|fonts| {
                    fonts
                        .FindFamilyName(&HSTRING::from(name), &mut index, &mut exists)
                        .is_ok()
                        && exists.as_bool()
                })
            };
            // Windows 11's UI and icon fonts, or Windows 10's.
            let ui_font = if installed("Segoe UI Variable Text") {
                "Segoe UI Variable Text"
            } else {
                "Segoe UI"
            };
            let icon_font = if installed("Segoe Fluent Icons") {
                "Segoe Fluent Icons"
            } else {
                "Segoe MDL2 Assets"
            };
            Ok(Self {
                d2d,
                target,
                write,
                ui_font: ui_font.into(),
                icon_font: icon_font.into(),
                formats: None,
            })
        }
    }

    unsafe fn target(d2d: &ID2D1Factory) -> Result<ID2D1DCRenderTarget, String> {
        unsafe {
            let target = d2d
                .CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                    r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_IGNORE,
                    },
                    // Units are physical pixels, as everything here is.
                    dpiX: 96.,
                    dpiY: 96.,
                    usage: D2D1_RENDER_TARGET_USAGE_NONE,
                    minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
                })
                .map_err(|err| format!("Couldn't start Direct2D: {err}"))?;
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            Ok(target)
        }
    }

    unsafe fn formats(&mut self, scale: f32) -> &Formats {
        unsafe {
            let dpi = (scale * 96.).round() as u32;
            if self.formats.as_ref().is_none_or(|(at, _)| *at != dpi) {
                let make = |family: &HSTRING, weight: DWRITE_FONT_WEIGHT, px: f32| {
                    let format = self
                        .write
                        .CreateTextFormat(
                            family,
                            None,
                            weight,
                            DWRITE_FONT_STYLE_NORMAL,
                            DWRITE_FONT_STRETCH_NORMAL,
                            px * scale,
                            w!("en-us"),
                        )
                        .expect("a system font");
                    let _ = format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                    format
                };
                let formats = Formats {
                    label: make(&self.ui_font, DWRITE_FONT_WEIGHT_MEDIUM, size::LABEL_FONT),
                    key: make(&self.ui_font, DWRITE_FONT_WEIGHT_SEMI_BOLD, size::KEY_FONT),
                    icon: make(&self.icon_font, DWRITE_FONT_WEIGHT_NORMAL, size::ICON_FONT),
                    size: make(&self.ui_font, DWRITE_FONT_WEIGHT_NORMAL, size::SIZE_FONT),
                };
                self.formats = Some((dpi, formats));
            }
            &self.formats.as_ref().expect("just made").1
        }
    }

    /// `text` laid out on one line, and its width and height.
    unsafe fn text(&self, text: &str, format: &IDWriteTextFormat) -> Option<Text> {
        unsafe {
            let wide: Vec<u16> = text.encode_utf16().collect();
            let layout = self
                .write
                .CreateTextLayout(&wide, format, 10_000., 10_000.)
                .ok()?;
            let mut metrics = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics).ok()?;
            Some(Text {
                layout,
                width: metrics.widthIncludingTrailingWhitespace,
                height: metrics.height,
            })
        }
    }
}

struct Text {
    layout: IDWriteTextLayout,
    width: f32,
    height: f32,
}

struct State {
    /// The virtual screen's top left corner, and its size.
    origin: (i32, i32),
    width: i32,
    height: i32,
    preset: Option<RegionAction>,
    bright: Surface,
    dim: Surface,
    back: Surface,
    graphics: Graphics,
    /// Where the drag started, while it goes on.
    anchor: Option<POINT>,
    selection: Option<RECT>,
    hover: Option<usize>,
    /// What was last drawn over the dimmed still, to repaint when it moves.
    drawn: RECT,
    done: bool,
    action: Option<RegionAction>,
}

impl State {
    /// Over `image`, or without one over a still of the screen taken now.
    /// The still goes straight into a bitmap and GDI dims it, so there's no
    /// pass over its pixels here (which took most of a second in a debug
    /// build).
    unsafe fn new(image: Option<&RgbaImage>, preset: Option<RegionAction>) -> Result<Self, String> {
        unsafe {
            let (origin, width, height) = match image {
                Some(image) => ((0, 0), image.width() as i32, image.height() as i32),
                None => (
                    (
                        GetSystemMetrics(SM_XVIRTUALSCREEN),
                        GetSystemMetrics(SM_YVIRTUALSCREEN),
                    ),
                    GetSystemMetrics(SM_CXVIRTUALSCREEN),
                    GetSystemMetrics(SM_CYVIRTUALSCREEN),
                ),
            };
            if width <= 0 || height <= 0 {
                return Err("No display to capture".into());
            }
            let screen = GetDC(None);
            if screen.is_invalid() {
                return Err("Couldn't reach the screen".into());
            }
            let surfaces = Self::surfaces(screen, image, origin, width, height);
            ReleaseDC(None, screen);
            let (bright, dim, back) = surfaces?;
            Ok(Self {
                origin,
                width,
                height,
                preset,
                bright,
                dim,
                back,
                graphics: Graphics::new()?,
                anchor: None,
                selection: None,
                hover: None,
                drawn: RECT::default(),
                done: false,
                action: None,
            })
        }
    }

    /// The still, the still dimmed, and the back buffer.
    unsafe fn surfaces(
        screen: HDC,
        image: Option<&RgbaImage>,
        origin: (i32, i32),
        width: i32,
        height: i32,
    ) -> Result<(Surface, Surface, Surface), String> {
        unsafe {
            let bright = match image {
                Some(image) => {
                    let mut bgra = image.as_raw().clone();
                    for px in bgra.as_chunks_mut::<4>().0 {
                        px.swap(0, 2);
                    }
                    Surface::new(screen, width, height, Some(&bgra))?
                }
                None => {
                    let bright = Surface::new(screen, width, height, None)?;
                    // CAPTUREBLT: layered windows too (most of what's on screen).
                    BitBlt(
                        bright.dc,
                        0,
                        0,
                        width,
                        height,
                        Some(screen),
                        origin.0,
                        origin.1,
                        SRCCOPY | CAPTUREBLT,
                    )
                    .map_err(|err| format!("Couldn't capture the screen: {err}"))?;
                    bright
                }
            };
            let dim = Surface::new(screen, width, height, None)?;
            let _ = BitBlt(dim.dc, 0, 0, width, height, Some(bright.dc), 0, 0, SRCCOPY);
            // Black over it, stretched from one pixel (a new bitmap is black).
            let black = Surface::new(screen, 1, 1, None)?;
            let _ = GdiAlphaBlend(
                dim.dc,
                0,
                0,
                width,
                height,
                black.dc,
                0,
                0,
                1,
                1,
                BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: DIM,
                    AlphaFormat: 0,
                },
            );
            Ok((bright, dim, Surface::new(screen, width, height, None)?))
        }
    }

    /// The pixels of `rect` of the still.
    unsafe fn crop(&self, rect: RECT) -> RgbaImage {
        unsafe {
            let _ = GdiFlush();
            let (w, h) = (
                (rect.right - rect.left) as u32,
                (rect.bottom - rect.top) as u32,
            );
            let stride = self.width as usize * 4;
            let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
            for y in rect.top..rect.bottom {
                let row = self
                    .bright
                    .bits
                    .add(y as usize * stride + rect.left as usize * 4);
                for px in std::slice::from_raw_parts(row, w as usize * 4)
                    .as_chunks::<4>()
                    .0
                {
                    rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
                }
            }
            RgbaImage::from_raw(w, h, rgba).expect("w by h pixels")
        }
    }
}

unsafe fn run(preset: Option<RegionAction>) -> Result<Option<Picked>, String> {
    unsafe {
        // Physical pixels, whatever the process's own DPI awareness, so a
        // scaled display isn't captured blurred or cut off.
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let state = Box::into_raw(Box::new(State::new(None, preset)?));
        let (origin, width, height) = ((*state).origin, (*state).width, (*state).height);

        let instance = GetModuleHandleW(None).map_err(|err| err.to_string())?;
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: CLASS,
            ..Default::default()
        };
        // Already registered by an earlier pick is fine.
        RegisterClassExW(&class);

        let window = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            CLASS,
            w!("Select a Region"),
            WS_POPUP,
            origin.0,
            origin.1,
            width,
            height,
            None,
            None,
            Some(instance.into()),
            Some(state as *const c_void),
        );
        let window = match window {
            Ok(window) => window,
            Err(err) => {
                drop(Box::from_raw(state));
                return Err(format!("Couldn't open the region picker: {err}"));
            }
        };
        // Appear at once, without the fade in.
        let off = BOOL(1);
        let _ = DwmSetWindowAttribute(
            window,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            (&raw const off).cast(),
            size_of::<BOOL>() as u32,
        );
        let _ = ShowWindow(window, SW_SHOW);
        crate::win::take_foreground(window.0 as isize);

        let mut msg = MSG::default();
        while !(*state).done && GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        SetWindowLongPtrW(window, GWLP_USERDATA, 0);
        let _ = DestroyWindow(window);
        let state = Box::from_raw(state);
        Ok(match (state.action, state.selection) {
            (Some(action), Some(rect)) => Some(Picked {
                image: state.crop(rect),
                action,
            }),
            _ => None,
        })
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
            return DefWindowProcW(window, message, wparam, lparam);
        }
        let state = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut State;
        if state.is_null() {
            return DefWindowProcW(window, message, wparam, lparam);
        }
        let state = &mut *state;
        let point = || POINT {
            x: (lparam.0 & 0xffff) as i16 as i32,
            y: ((lparam.0 >> 16) & 0xffff) as i16 as i32,
        };
        match message {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(window, &mut ps);
                let area = ps.rcPaint;
                compose(state, area);
                let _ = BitBlt(
                    dc,
                    area.left,
                    area.top,
                    area.right - area.left,
                    area.bottom - area.top,
                    Some(state.back.dc),
                    area.left,
                    area.top,
                    SRCCOPY,
                );
                let _ = EndPaint(window, &ps);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            // The still is in physical pixels: never rescale the window.
            WM_DPICHANGED => LRESULT(0),
            WM_SETCURSOR if (lparam.0 & 0xffff) as u32 == HTCLIENT => {
                let cursor = if state.hover.is_some() {
                    IDC_HAND
                } else {
                    IDC_CROSS
                };
                if let Ok(cursor) = LoadCursorW(None, cursor) {
                    SetCursor(Some(cursor));
                }
                LRESULT(1)
            }
            WM_LBUTTONDOWN => {
                let at = point();
                if let Some(button) = button_at(state, at) {
                    finish(state, Some(BUTTONS[button].action));
                    return LRESULT(0);
                }
                state.anchor = Some(at);
                state.selection = None;
                state.hover = None;
                SetCapture(window);
                refresh(window, state);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                let at = point();
                if let Some(anchor) = state.anchor {
                    state.selection = Some(RECT {
                        left: anchor.x.min(at.x).max(0),
                        top: anchor.y.min(at.y).max(0),
                        right: (anchor.x.max(at.x) + 1).min(state.width),
                        bottom: (anchor.y.max(at.y) + 1).min(state.height),
                    });
                    refresh(window, state);
                } else {
                    let hover = button_at(state, at);
                    if hover != state.hover {
                        state.hover = hover;
                        refresh(window, state);
                    }
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                if state.anchor.take().is_some() {
                    let _ = ReleaseCapture();
                    let small = state.selection.is_none_or(|rect| {
                        rect.right - rect.left < MIN_SIDE && rect.bottom - rect.top < MIN_SIDE
                    });
                    if small {
                        state.selection = None;
                    } else if state.preset.is_some() {
                        finish(state, state.preset);
                        return LRESULT(0);
                    }
                    state.hover = button_at(state, point());
                    refresh(window, state);
                }
                LRESULT(0)
            }
            WM_RBUTTONDOWN => {
                finish(state, None);
                LRESULT(0)
            }
            WM_KEYDOWN => {
                let key = wparam.0 as u16;
                let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
                if key == VK_ESCAPE.0 {
                    finish(state, None);
                } else if state.selection.is_some() && state.anchor.is_none() {
                    let action = match key {
                        k if k == VK_RETURN.0 => {
                            Some(state.preset.unwrap_or(RegionAction::CopyText))
                        }
                        k if k == b'T' as u16 => Some(RegionAction::CopyText),
                        k if k == b'Q' as u16 => Some(RegionAction::ScanQrCode),
                        k if k == b'C' as u16 && ctrl => Some(RegionAction::CopyText),
                        k if k == b'L' as u16 || k == b'G' as u16 => Some(RegionAction::GoogleLens),
                        _ => None,
                    };
                    if action.is_some() {
                        finish(state, action);
                    }
                }
                LRESULT(0)
            }
            WM_ACTIVATE if (wparam.0 & 0xffff) as u32 == WA_INACTIVE => {
                // Switched away (Alt+Tab, Win): don't stay over everything.
                finish(state, None);
                LRESULT(0)
            }
            WM_CLOSE => {
                finish(state, None);
                LRESULT(0)
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

fn finish(state: &mut State, action: Option<RegionAction>) {
    state.action = action;
    state.done = true;
}

fn bar_shown(state: &State) -> bool {
    state.preset.is_none() && state.anchor.is_none() && state.selection.is_some()
}

/// Repaints what changed: what was drawn last time and what is drawn now.
unsafe fn refresh(window: HWND, state: &mut State) {
    unsafe {
        let now = extent(state);
        let dirty = union(state.drawn, now);
        state.drawn = now;
        let _ = InvalidateRect(Some(window), Some(&dirty), false);
    }
}

fn union(a: RECT, b: RECT) -> RECT {
    if a.right <= a.left {
        return b;
    }
    if b.right <= b.left {
        return a;
    }
    RECT {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}

fn inflate(rect: RECT, by: i32) -> RECT {
    RECT {
        left: rect.left - by,
        top: rect.top - by,
        right: rect.right + by,
        bottom: rect.bottom + by,
    }
}

fn outer(rect: D2D_RECT_F) -> RECT {
    RECT {
        left: rect.left.floor() as i32,
        top: rect.top.floor() as i32,
        right: rect.right.ceil() as i32,
        bottom: rect.bottom.ceil() as i32,
    }
}

/// Everything drawn over the dimmed still: the selection with its frame
/// and size label, and the bar.
unsafe fn extent(state: &mut State) -> RECT {
    unsafe {
        let Some(selection) = state.selection else {
            return RECT::default();
        };
        let (_, scale) = monitor_at(state, corner(selection));
        let mut all = inflate(selection, (size::HANDLE_WIDTH * scale).ceil() as i32 + 2);
        if let Some(label) = size_label(state, selection) {
            all = union(all, inflate(outer(label.rect), 2));
        }
        if let Some(bar) = bar_layout(state, selection) {
            all = union(all, inflate(outer(bar.rect), 2));
        }
        all
    }
}

fn corner(selection: RECT) -> POINT {
    POINT {
        x: selection.left,
        y: selection.top,
    }
}

/// The monitor under a point of the window, in window coordinates, and its scale.
unsafe fn monitor_at(state: &State, at: POINT) -> (RECT, f32) {
    unsafe {
        let screen = POINT {
            x: at.x + state.origin.0,
            y: at.y + state.origin.1,
        };
        let monitor = MonitorFromPoint(screen, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let (mut dpi_x, mut dpi_y) = (96, 96);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let rect = info.rcMonitor;
        (
            RECT {
                left: rect.left - state.origin.0,
                top: rect.top - state.origin.1,
                right: rect.right - state.origin.0,
                bottom: rect.bottom - state.origin.1,
            },
            dpi_x as f32 / 96.,
        )
    }
}

struct Label {
    rect: D2D_RECT_F,
    text: Text,
    scale: f32,
}

/// The "W × H" chip: above the selection's top left corner, or inside it
/// at the top of a display.
unsafe fn size_label(state: &mut State, selection: RECT) -> Option<Label> {
    unsafe {
        let (monitor, scale) = monitor_at(state, corner(selection));
        let format = state.graphics.formats(scale).size.clone();
        let text = format!(
            "{} × {}",
            selection.right - selection.left,
            selection.bottom - selection.top
        );
        let text = state.graphics.text(&text, &format)?;
        let w = (text.width + 2. * size::SIZE_PAD_X * scale).round();
        let h = (size::SIZE_HEIGHT * scale).round();
        let gap = size::SIZE_GAP * scale;
        let mut top = selection.top as f32 - gap - h;
        if top < monitor.top as f32 {
            top = selection.top as f32 + gap;
        }
        let left = (selection.left as f32)
            .min(monitor.right as f32 - w)
            .max(monitor.left as f32);
        Some(Label {
            rect: D2D_RECT_F {
                left,
                top,
                right: left + w,
                bottom: top + h,
            },
            text,
            scale,
        })
    }
}

struct Bar {
    rect: D2D_RECT_F,
    buttons: Vec<Placed>,
    scale: f32,
}

struct Placed {
    rect: D2D_RECT_F,
    icon: Text,
    label: Text,
    key: Text,
}

/// The bar and its buttons: under the selection's right end, or above it,
/// or inside it, whichever fits on its display.
unsafe fn bar_layout(state: &mut State, selection: RECT) -> Option<Bar> {
    unsafe {
        if !bar_shown(state) {
            return None;
        }
        let at = POINT {
            x: selection.right - 1,
            y: selection.bottom - 1,
        };
        let (monitor, scale) = monitor_at(state, at);
        let s = |v: f32| (v * scale).round();
        let formats = state.graphics.formats(scale);
        let (label_format, key_format, icon_format) = (
            formats.label.clone(),
            formats.key.clone(),
            formats.icon.clone(),
        );
        let mut parts = Vec::new();
        for button in &BUTTONS {
            let icon = state
                .graphics
                .text(&button.glyph.to_string(), &icon_format)?;
            let label = state.graphics.text(button.label, &label_format)?;
            let key = state.graphics.text(button.key, &key_format)?;
            let width = s(size::BUTTON_PAD_LEFT)
                + icon.width.ceil()
                + s(size::ICON_GAP)
                + label.width.ceil()
                + s(size::KEY_GAP)
                + s(size::KEYCAP).max(key.width.ceil() + s(8.))
                + s(size::BUTTON_PAD_RIGHT);
            parts.push((width, icon, label, key));
        }
        let pad = s(size::BAR_PAD);
        let divider = s(size::DIVIDER_GAP) * 2. + 1.;
        let width = pad * 2.
            + parts.iter().map(|(w, ..)| w).sum::<f32>()
            + divider * (parts.len() - 1) as f32;
        let height = s(size::BUTTON_HEIGHT) + pad * 2.;
        let gap = s(size::BAR_GAP);
        let mut top = selection.bottom as f32 + gap;
        if top + height > monitor.bottom as f32 - gap {
            top = selection.top as f32 - gap - height;
            if top < monitor.top as f32 + gap {
                top = selection.bottom as f32 - gap - height;
            }
        }
        let left = (selection.right as f32 - width)
            .min(monitor.right as f32 - width - gap)
            .max(monitor.left as f32 + gap);
        let mut x = left + pad;
        let mut buttons = Vec::new();
        for (w, icon, label, key) in parts {
            buttons.push(Placed {
                rect: D2D_RECT_F {
                    left: x,
                    top: top + pad,
                    right: x + w,
                    bottom: top + height - pad,
                },
                icon,
                label,
                key,
            });
            x += w + divider;
        }
        Some(Bar {
            rect: D2D_RECT_F {
                left,
                top,
                right: left + width,
                bottom: top + height,
            },
            buttons,
            scale,
        })
    }
}

unsafe fn button_at(state: &mut State, at: POINT) -> Option<usize> {
    unsafe {
        let bar = bar_layout(state, state.selection?)?;
        let (x, y) = (at.x as f32, at.y as f32);
        bar.buttons.iter().position(|b| {
            x >= b.rect.left && x < b.rect.right && y >= b.rect.top && y < b.rect.bottom
        })
    }
}

/// Draws `area` of the picker into the back buffer.
unsafe fn compose(state: &mut State, area: RECT) {
    unsafe {
        let back = state.back.dc;
        let (w, h) = (area.right - area.left, area.bottom - area.top);
        let _ = BitBlt(
            back,
            area.left,
            area.top,
            w,
            h,
            Some(state.dim.dc),
            area.left,
            area.top,
            SRCCOPY,
        );
        let Some(selection) = state.selection else {
            return;
        };
        let _ = BitBlt(
            back,
            selection.left,
            selection.top,
            selection.right - selection.left,
            selection.bottom - selection.top,
            Some(state.bright.dc),
            selection.left,
            selection.top,
            SRCCOPY,
        );
        let label = size_label(state, selection);
        let bar = bar_layout(state, selection);
        let (_, scale) = monitor_at(state, corner(selection));
        let target = state.graphics.target.clone();
        if target.BindDC(back, &area).is_err() {
            return;
        }
        target.BeginDraw();
        // The bound area's top left is the target's origin.
        target.SetTransform(&Matrix3x2 {
            M11: 1.,
            M12: 0.,
            M21: 0.,
            M22: 1.,
            M31: -area.left as f32,
            M32: -area.top as f32,
        });
        let paint = Paint { target: &target };
        paint.frame(selection, scale);
        if let Some(label) = &label {
            paint.size_label(label);
        }
        if let Some(bar) = &bar {
            paint.bar(bar, state.hover);
        }
        if target.EndDraw(None, None).is_err() {
            // The device went away (a driver update): start over next time.
            if let Ok(fresh) = Graphics::target(&state.graphics.d2d) {
                state.graphics.target = fresh;
            }
        }
    }
}

struct Paint<'a> {
    target: &'a ID2D1DCRenderTarget,
}

impl Paint<'_> {
    unsafe fn brush(&self, rgba: u32) -> Option<ID2D1SolidColorBrush> {
        unsafe {
            let c = |shift: u32| ((rgba >> shift) & 0xff) as f32 / 255.;
            self.target
                .CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: c(24),
                        g: c(16),
                        b: c(8),
                        a: c(0),
                    },
                    None,
                )
                .ok()
        }
    }

    unsafe fn fill_rect(&self, rect: D2D_RECT_F, rgba: u32) {
        unsafe {
            if let Some(brush) = self.brush(rgba) {
                self.target.FillRectangle(&rect, &brush);
            }
        }
    }

    unsafe fn fill_rounded(&self, rect: D2D_RECT_F, radius: f32, rgba: u32) {
        unsafe {
            if let Some(brush) = self.brush(rgba) {
                self.target.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect,
                        radiusX: radius,
                        radiusY: radius,
                    },
                    &brush,
                );
            }
        }
    }

    /// A 1 px outline just inside `rect`.
    unsafe fn stroke_rounded(&self, rect: D2D_RECT_F, radius: f32, rgba: u32) {
        unsafe {
            if let Some(brush) = self.brush(rgba) {
                self.target.DrawRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: inset(rect, 0.5),
                        radiusX: radius - 0.5,
                        radiusY: radius - 0.5,
                    },
                    &brush,
                    1.,
                    None,
                );
            }
        }
    }

    unsafe fn text(&self, text: &Text, x: f32, y: f32, rgba: u32) {
        unsafe {
            if let Some(brush) = self.brush(rgba) {
                self.target.DrawTextLayout(
                    Vector2 {
                        X: x.round(),
                        Y: y.round(),
                    },
                    &text.layout,
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                );
            }
        }
    }

    /// `text` centred in `rect`.
    unsafe fn text_centered(&self, text: &Text, rect: D2D_RECT_F, rgba: u32) {
        unsafe {
            let x = rect.left + (rect.right - rect.left - text.width) / 2.;
            let y = rect.top + (rect.bottom - rect.top - text.height) / 2.;
            self.text(text, x, y, rgba);
        }
    }

    /// The selection's outline, with brackets on its corners.
    unsafe fn frame(&self, selection: RECT, scale: f32) {
        unsafe {
            let (l, t, r, b) = (
                selection.left as f32,
                selection.top as f32,
                selection.right as f32,
                selection.bottom as f32,
            );
            let line = scale.round().max(1.);
            // Just outside the picked pixels, so they show as they are.
            self.fill_rect(rect(l - line, t - line, r + line, t), color::ACCENT);
            self.fill_rect(rect(l - line, b, r + line, b + line), color::ACCENT);
            self.fill_rect(rect(l - line, t, l, b), color::ACCENT);
            self.fill_rect(rect(r, t, r + line, b), color::ACCENT);

            let thick = (size::HANDLE_WIDTH * scale).round();
            let long = (size::HANDLE * scale)
                .round()
                .min((r - l) / 2. + thick)
                .min((b - t) / 2. + thick);
            for (x, y, dx, dy) in [
                (l, t, 1., 1.),
                (r, t, -1., 1.),
                (l, b, 1., -1.),
                (r, b, -1., -1.),
            ] {
                // The bracket's outer corner, outside the selection.
                let (ox, oy) = (x - dx * thick, y - dy * thick);
                let horizontal = rect(ox, oy, ox + dx * long, oy + dy * thick);
                let vertical = rect(ox, oy, ox + dx * thick, oy + dy * long);
                self.fill_rect(normal(horizontal), color::ACCENT);
                self.fill_rect(normal(vertical), color::ACCENT);
            }
        }
    }

    unsafe fn size_label(&self, label: &Label) {
        unsafe {
            let radius = 4. * label.scale;
            self.fill_rounded(label.rect, radius, color::LABEL);
            self.stroke_rounded(label.rect, radius, color::PANEL_BORDER);
            self.text_centered(&label.text, label.rect, color::TEXT_MUTED);
        }
    }

    unsafe fn bar(&self, bar: &Bar, hover: Option<usize>) {
        unsafe {
            let scale = bar.scale;
            let radius = size::BAR_RADIUS * scale;
            self.fill_rounded(bar.rect, radius, color::PANEL);
            self.stroke_rounded(bar.rect, radius, color::PANEL_BORDER);

            for (i, button) in bar.buttons.iter().enumerate() {
                let r = button.rect;
                if i > 0 {
                    let x = (r.left - (size::DIVIDER_GAP * scale).round() - 1.).round();
                    let inset_y = 8. * scale;
                    self.fill_rect(
                        rect(x, r.top + inset_y, x + 1., r.bottom - inset_y),
                        color::DIVIDER,
                    );
                }
                if hover == Some(i) {
                    self.fill_rounded(r, size::BUTTON_RADIUS * scale, color::HOVER);
                } else if hover.is_none() && i == 0 {
                    // Enter does this one.
                    self.fill_rounded(r, size::BUTTON_RADIUS * scale, color::PRESSED);
                }
                let middle = (r.top + r.bottom) / 2.;
                let mut x = r.left + (size::BUTTON_PAD_LEFT * scale).round();
                self.text(
                    &button.icon,
                    x,
                    middle - button.icon.height / 2.,
                    color::ACCENT,
                );
                x += button.icon.width.ceil() + (size::ICON_GAP * scale).round();
                self.text(
                    &button.label,
                    x,
                    middle - button.label.height / 2.,
                    color::TEXT,
                );

                let cap_w = (size::KEYCAP * scale)
                    .round()
                    .max(button.key.width.ceil() + (8. * scale).round());
                let cap_h = (size::KEYCAP * scale).round();
                let right = r.right - (size::BUTTON_PAD_RIGHT * scale).round();
                let cap = rect(
                    right - cap_w,
                    middle - cap_h / 2.,
                    right,
                    middle + cap_h / 2.,
                );
                let cap = D2D_RECT_F {
                    top: cap.top.round(),
                    bottom: cap.top.round() + cap_h,
                    ..cap
                };
                let cap_radius = size::KEYCAP_RADIUS * scale;
                self.fill_rounded(cap, cap_radius, color::KEYCAP);
                self.stroke_rounded(cap, cap_radius, color::KEYCAP_BORDER);
                self.text_centered(&button.key, cap, color::KEYCAP_TEXT);
            }
        }
    }
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

/// With left ≤ right and top ≤ bottom.
fn normal(r: D2D_RECT_F) -> D2D_RECT_F {
    rect(
        r.left.min(r.right),
        r.top.min(r.bottom),
        r.left.max(r.right),
        r.top.max(r.bottom),
    )
}

fn inset(r: D2D_RECT_F, by: f32) -> D2D_RECT_F {
    rect(r.left + by, r.top + by, r.right - by, r.bottom - by)
}
