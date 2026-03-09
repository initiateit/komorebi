use crate::DEFAULT_CONTAINER_PADDING;
use crate::WindowsApi;
use crate::border_manager::BORDER_OFFSET;
use crate::border_manager::BORDER_WIDTH;
use crate::container::Container;
use crate::core::Rect;
use crate::core::StackbarLabel;
use crate::core::StackbarPosition;
use crate::stackbar_manager::STACKBAR_LABEL;
use crate::stackbar_manager::STACKBAR_TAB_FOCUSED_BACKGROUND_COLOUR;
use crate::stackbar_manager::STACKBAR_TAB_UNFOCUSED_BACKGROUND_COLOUR;
use crate::stackbar_manager::STACKBAR_TAB_HEIGHT;
use crate::stackbar_manager::STACKBAR_TAB_WIDTH;
use crate::stackbar_manager::STACKBAR_VERTICAL_WIDTH;
use crate::stackbar_manager::STACKBAR_POSITION;
use crate::stackbar_manager::STACKBARS_CONTAINERS;
use crate::stackbar_manager::STACKBAR_TOOLTIP_LABELS;
use super::STACKBAR_TOOLTIP_ALPHA;
use super::STACKBAR_TOOLTIP_BACKGROUND_COLOUR;
use super::STACKBAR_TOOLTIP_FONT_FAMILY;
use super::STACKBAR_TOOLTIP_FONT_WEIGHT;
use super::STACKBAR_TOOLTIP_FONT_SIZE;
use super::STACKBAR_TOOLTIP_TEXT_COLOUR;
use crate::windows_api;
use crossbeam_utils::atomic::AtomicConsume;
use std::sync::mpsc;
use std::time::Duration;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::TRUE;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::Graphics::Dwm::DWM_BB_BLURREGION;
use windows::Win32::Graphics::Dwm::DWM_BB_ENABLE;
use windows::Win32::Graphics::Dwm::DWM_BLURBEHIND;
use windows::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE;
use windows::Win32::Graphics::Dwm::DWMWA_SYSTEMBACKDROP_TYPE;
use windows::Win32::Graphics::Dwm::DWMWCP_ROUND;
use windows::Win32::Graphics::Dwm::DWMSBT_TRANSIENTWINDOW;
use windows::Win32::Graphics::Dwm::DwmEnableBlurBehindWindow;
use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
use windows::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows::Win32::UI::Controls::MARGINS;
const WM_MOUSELEAVE_MSG: u32 = 0x02A3;
use windows::Win32::Graphics::Gdi::CreatePen;
use windows::Win32::Graphics::Gdi::CreateRectRgn;
use windows::Win32::Graphics::Gdi::CreateSolidBrush;
use windows::Win32::Graphics::Gdi::DeleteObject;
use windows::Win32::Graphics::Gdi::PS_SOLID;
use windows::Win32::Graphics::Gdi::SelectObject;
use windows::Win32::Graphics::Gdi::SetBkColor;
use windows::Win32::Graphics::Gdi::CreateCompatibleDC;
use windows::Win32::Graphics::Gdi::BITMAPINFO;
use windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
use windows::Win32::Graphics::Gdi::BI_RGB;
use windows::Win32::Graphics::Gdi::DIB_RGB_COLORS;
use windows::Win32::Graphics::Gdi::CreateDIBSection;
use windows::Win32::Graphics::Gdi::BLENDFUNCTION;
use windows::Win32::UI::WindowsAndMessaging::CS_HREDRAW;
use windows::Win32::UI::WindowsAndMessaging::CS_VREDRAW;
use windows::Win32::UI::WindowsAndMessaging::CreateWindowExW;
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows::Win32::UI::WindowsAndMessaging::DispatchMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetMessageW;
use windows::Win32::UI::WindowsAndMessaging::IDC_ARROW;
use windows::Win32::UI::WindowsAndMessaging::LoadCursorW;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use windows::Win32::UI::WindowsAndMessaging::SetCursor;
use windows::Win32::UI::WindowsAndMessaging::TranslateMessage;
use windows::Win32::UI::WindowsAndMessaging::UpdateLayeredWindow;
use windows::Win32::UI::WindowsAndMessaging::ULW_ALPHA;
use windows::Win32::UI::WindowsAndMessaging::WM_DESTROY;
use windows::Win32::UI::WindowsAndMessaging::WM_ERASEBKGND;
use windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONDOWN;
use windows::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE;
use windows::Win32::UI::WindowsAndMessaging::WM_SETCURSOR;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_NOACTIVATE;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
use windows::Win32::UI::WindowsAndMessaging::WS_POPUP;
use windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE;
use windows::Win32::UI::WindowsAndMessaging::DrawIconEx;
use windows::Win32::UI::WindowsAndMessaging::DI_NORMAL;
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use windows::Win32::UI::WindowsAndMessaging::SetWindowPos;
use windows::Win32::UI::WindowsAndMessaging::ShowWindow;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE;
use windows::Win32::UI::WindowsAndMessaging::HWND_TOPMOST;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE;
use windows::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT;
use windows::Win32::UI::Input::KeyboardAndMouse::TME_LEAVE;
use windows::Win32::UI::Input::KeyboardAndMouse::TrackMouseEvent;
use windows::Win32::Graphics::GdiPlus::{
    FillModeAlternate, GdipAddPathArcI, GdipClosePathFigure, GdipCreateFromHDC,
    GdipCreatePath, GdipCreateSolidFill, GdipDeleteBrush, GdipDeleteGraphics,
    GdipDeletePath, GdipFillPath, GdipSetSmoothingMode, GdiplusStartup,
    GdiplusStartupInput, GpBrush, SmoothingModeAntiAlias,
    GdipCreateFont, GdipCreateStringFormat, GdipDeleteFont,
    GdipDeleteFontFamily, GdipDeleteStringFormat,
    GdipCreateFontFamilyFromName, GdipMeasureString, GdipDrawString,
    FontStyleRegular, FontStyleBold, FontStyleItalic, StringFormatFlagsNoWrap, Unit,
    GdipSetTextRenderingHint, TextRenderingHintAntiAliasGridFit, UnitPoint,
};
use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE};
use windows_core::BOOL;
use windows::core::PCWSTR;

/// Maps a stackbar hwnd to its DWM tooltip hwnd.
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::collections::HashMap;

lazy_static! {
    /// Maps stackbar hwnd -> tooltip hwnd
    static ref TOOLTIP_HWNDS: Mutex<HashMap<isize, isize>> = Mutex::new(HashMap::new());
    /// Tracks whether the mouse is currently inside the stackbar (per stackbar hwnd).
    static ref TRACKING_MOUSE: Mutex<HashMap<isize, bool>> = Mutex::new(HashMap::new());
    /// Tracks which tab index is currently hovered (per stackbar hwnd), -1 = none.
    static ref HOVERED_TAB: Mutex<HashMap<isize, i32>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub struct Stackbar {
    pub hwnd: isize,
    pub tooltip_hwnd: isize,
}

impl From<isize> for Stackbar {
    fn from(value: isize) -> Self {
        Self { hwnd: value, tooltip_hwnd: 0 }
    }
}

impl Stackbar {
    pub const fn hwnd(&self) -> HWND {
        HWND(windows_api::as_ptr!(self.hwnd))
    }

    /// Create the DWM-styled tooltip popup window (initially hidden).
    unsafe fn create_dwm_tooltip(parent: HWND, h_module: HINSTANCE) -> isize {
        let tooltip_class_name: Vec<u16> = "komotooltip\0".encode_utf16().collect();
        let class_name = PCWSTR(tooltip_class_name.as_ptr());

        unsafe extern "system" fn def_window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }

        // Register a minimal window class for the tooltip (idempotent).
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(def_window_proc),
            hInstance: h_module,
            lpszClassName: class_name,
            hbrBackground: WindowsApi::create_solid_brush(0),
            ..Default::default()
        };
        let _ = WindowsApi::register_class_w(&wc);

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name,
                PCWSTR::null(),
                WS_POPUP, // NOT visible initially
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1,
                1,
                Some(parent),
                None,
                Some(h_module),
                None,
            )
        };

        let hwnd = match hwnd {
            Ok(h) => h,
            Err(_) => return 0,
        };

        unsafe {
            // Enable DWM blur behind
            let bb = DWM_BLURBEHIND {
                dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
                fEnable: TRUE,
                hRgnBlur: CreateRectRgn(0, 0, -1, -1),
                fTransitionOnMaximized: FALSE,
            };
            let _ = DwmEnableBlurBehindWindow(hwnd, &bb);

            // Extend glass into entire client area
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

            // Rounded corners (Windows 11+)
            let corner = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                std::ptr::addr_of!(corner).cast(),
                std::mem::size_of_val(&corner) as u32,
            );

            // Acrylic/transient backdrop (Windows 11 22H2+)
            let backdrop = DWMSBT_TRANSIENTWINDOW;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                std::ptr::addr_of!(backdrop).cast(),
                std::mem::size_of_val(&backdrop) as u32,
            );

            let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
        }

        hwnd.0 as isize
    }

    /// Render tooltip text to a layered window surface and show it at (x, y).
    unsafe fn show_tooltip(tooltip_hwnd: isize, text: &str, x: i32, y: i32) {
        if tooltip_hwnd == 0 || text.is_empty() {
            return;
        }

        let hwnd = HWND(windows_api::as_ptr!(tooltip_hwnd));

        unsafe {
            // Measure text with GDI+ to determine tooltip size
            let wide_text: Vec<u16> = text.encode_utf16().collect();
            let text_len = wide_text.len() as i32;

            let font_family_lock = STACKBAR_TOOLTIP_FONT_FAMILY.lock();
            let font_name = font_family_lock.as_deref().unwrap_or("Segoe UI");
            let mut font_family_name: Vec<u16> = font_name.encode_utf16().collect();
            font_family_name.push(0); 

            let mut font_family = std::ptr::null_mut();
            GdipCreateFontFamilyFromName(
                PCWSTR(font_family_name.as_ptr()),
                std::ptr::null_mut(),
                &mut font_family,
            );

            let font_size = STACKBAR_TOOLTIP_FONT_SIZE.load_consume() as f32;
            let weight_lock = STACKBAR_TOOLTIP_FONT_WEIGHT.lock();
            let weight_str = weight_lock.as_deref().unwrap_or("Regular").to_lowercase();
            let style = match weight_str.as_str() {
                "bold" | "700" => FontStyleBold.0 as i32,
                "semibold" | "600" => FontStyleBold.0 as i32, // GDI+ maps SemiBold to Bold style if no specific SemiBold weight handled
                "italic" => FontStyleItalic.0 as i32,
                _ => FontStyleRegular.0 as i32,
            };
            
            let mut font: *mut windows::Win32::Graphics::GdiPlus::GpFont = std::ptr::null_mut();
            let status = GdipCreateFont(font_family, font_size, style, Unit(UnitPoint.0), &mut font);
            
            // Fallback if font creation failed (e.g. invalid family)
            if status.0 != 0 || font.is_null() {
                let mut generic_family = std::ptr::null_mut();
                let _ = windows::Win32::Graphics::GdiPlus::GdipGetGenericFontFamilySansSerif(&mut generic_family);
                let _ = GdipCreateFont(generic_family, font_size, style, Unit(UnitPoint.0), &mut font);
                let _ = GdipDeleteFontFamily(generic_family);
            }

            let mut string_format = std::ptr::null_mut();
            GdipCreateStringFormat(StringFormatFlagsNoWrap.0 as i32, 0, &mut string_format);

            // Create a temporary DC/graphics just for measuring
            let hdc_measure = CreateCompatibleDC(None);
            let mut graphics_measure = std::ptr::null_mut();
            GdipCreateFromHDC(hdc_measure, &mut graphics_measure);

            let layout_rect = windows::Win32::Graphics::GdiPlus::RectF {
                X: 0.0,
                Y: 0.0,
                Width: 2000.0,
                Height: 100.0,
            };
            let mut bounding_box = windows::Win32::Graphics::GdiPlus::RectF {
                X: 0.0,
                Y: 0.0,
                Width: 0.0,
                Height: 0.0,
            };

            GdipMeasureString(
                graphics_measure,
                PCWSTR(wide_text.as_ptr()),
                text_len,
                font,
                &layout_rect,
                string_format,
                &mut bounding_box,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            GdipDeleteGraphics(graphics_measure);
            let _ = windows::Win32::Graphics::Gdi::DeleteDC(hdc_measure);

            let pad_x = 12;
            let pad_y = 6;
            let tip_width = bounding_box.Width.ceil() as i32 + pad_x * 2;
            let tip_height = bounding_box.Height.ceil() as i32 + pad_y * 2;

            // Create a 32-bit DIB for per-pixel alpha blending
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: tip_width,
                    biHeight: -tip_height, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pvbits: *mut std::ffi::c_void = std::ptr::null_mut();
            let hbm = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut pvbits, None, 0)
                .unwrap_or_default();

            if !pvbits.is_null() {
                // Fill with semi-transparent dark background (BGRA premultiplied)
                let pixels = std::slice::from_raw_parts_mut(
                    pvbits as *mut u8,
                    (tip_width * tip_height * 4) as usize,
                );
                let alpha = STACKBAR_TOOLTIP_ALPHA.load_consume() as u8;
                let bg_colour = STACKBAR_TOOLTIP_BACKGROUND_COLOUR.load_consume();
                let bg_r = (bg_colour & 0xFF) as u8;
                let bg_g = ((bg_colour >> 8) & 0xFF) as u8;
                let bg_b = ((bg_colour >> 16) & 0xFF) as u8;

                // Premultiply
                let pr = (bg_r as u16 * alpha as u16 / 255) as u8;
                let pg = (bg_g as u16 * alpha as u16 / 255) as u8;
                let pb = (bg_b as u16 * alpha as u16 / 255) as u8;
                for chunk in pixels.chunks_exact_mut(4) {
                    chunk[0] = pb; // B
                    chunk[1] = pg; // G
                    chunk[2] = pr; // R
                    chunk[3] = alpha; // A
                }
            }

            let hdc_mem = CreateCompatibleDC(None);
            let hbm_old = SelectObject(hdc_mem, hbm.into());

            // Draw text with GDI+
            let mut graphics = std::ptr::null_mut();
            GdipCreateFromHDC(hdc_mem, &mut graphics);
            GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
            GdipSetTextRenderingHint(graphics, TextRenderingHintAntiAliasGridFit);

            // User-defined text colour (0xBBGGRR) -> GDI+ ARGB (0xAARRGGBB)
            let text_colour = STACKBAR_TOOLTIP_TEXT_COLOUR.load_consume();
            let tr = text_colour & 0xFF;
            let tg = (text_colour >> 8) & 0xFF;
            let tb = (text_colour >> 16) & 0xFF;
            let text_argb: u32 = 0xFF_00_00_00 | (tr << 16) | (tg << 8) | tb;

            let mut text_brush = std::ptr::null_mut();
            GdipCreateSolidFill(text_argb, &mut text_brush);

            let draw_rect = windows::Win32::Graphics::GdiPlus::RectF {
                X: pad_x as f32,
                Y: pad_y as f32,
                Width: (tip_width - pad_x * 2) as f32,
                Height: (tip_height - pad_y * 2) as f32,
            };

            GdipDrawString(
                graphics,
                PCWSTR(wide_text.as_ptr()),
                text_len,
                font,
                &draw_rect,
                string_format,
                text_brush as *mut GpBrush,
            );

            GdipDeleteBrush(text_brush as *mut GpBrush);
            GdipDeleteGraphics(graphics);
            GdipDeleteFont(font);
            GdipDeleteFontFamily(font_family);
            GdipDeleteStringFormat(string_format);

            // Blit to the layered window with UpdateLayeredWindow
            let blend = BLENDFUNCTION {
                BlendOp: 0,   // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1, // AC_SRC_ALPHA
            };
            let src_pt = windows::Win32::Foundation::POINT { x: 0, y: 0 };
            let dst_pt = windows::Win32::Foundation::POINT { x, y };
            let size = windows::Win32::Foundation::SIZE {
                cx: tip_width,
                cy: tip_height,
            };
            let _ = UpdateLayeredWindow(
                hwnd,
                None,
                Some(&dst_pt),
                Some(&size),
                Some(hdc_mem),
                Some(&src_pt),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            SelectObject(hdc_mem, hbm_old);
            let _ = DeleteObject(hbm.into());
            let _ = windows::Win32::Graphics::Gdi::DeleteDC(hdc_mem);

            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    unsafe fn hide_tooltip(tooltip_hwnd: isize) {
        if tooltip_hwnd != 0 {
            unsafe {
                let _ = ShowWindow(
                    HWND(windows_api::as_ptr!(tooltip_hwnd)),
                    SW_HIDE,
                );
            }
        }
    }

    pub fn create(id: &str) -> color_eyre::Result<Self> {
        let name: Vec<u16> = format!("komostackbar-{id}\0").encode_utf16().collect();
        let class_name = PCWSTR(name.as_ptr());

        let h_module = WindowsApi::module_handle_w()?;

        let window_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(Self::callback),
            hInstance: h_module.into(),
            lpszClassName: class_name,
            hbrBackground: WindowsApi::create_solid_brush(0),
            ..Default::default()
        };

        let _ = WindowsApi::register_class_w(&window_class);

        let (hwnd_sender, hwnd_receiver) = mpsc::channel();

        let name_cl = name.clone();
        let instance = h_module.0 as isize;
        std::thread::spawn(move || -> color_eyre::Result<()> {
            unsafe {
                let hwnd = CreateWindowExW(
                    WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                    PCWSTR(name_cl.as_ptr()),
                    PCWSTR(name_cl.as_ptr()),
                    WS_POPUP | WS_VISIBLE,
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    Option::from(HINSTANCE(windows_api::as_ptr!(instance))),
                    None,
                )?;

                let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);

                // Create DWM-styled tooltip window
                let tooltip_hwnd = Self::create_dwm_tooltip(
                    hwnd,
                    HINSTANCE(windows_api::as_ptr!(instance)),
                );

                // Store tooltip hwnd in the shared map
                TOOLTIP_HWNDS.lock().insert(hwnd.0 as isize, tooltip_hwnd);

                hwnd_sender.send((hwnd.0 as isize, tooltip_hwnd))?;

                let mut msg: MSG = MSG::default();

                loop {
                    if !GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        tracing::debug!("stackbar window event processing thread shutdown");
                        break;
                    };
                    // TODO: error handling
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);

                    std::thread::sleep(Duration::from_millis(10))
                }
            }

            Ok(())
        });

        let (hwnd, tooltip_hwnd) = hwnd_receiver.recv()?;
        Ok(Self { hwnd, tooltip_hwnd })
    }

    pub fn destroy(&self) -> color_eyre::Result<()> {
        // Clean up shared maps
        TOOLTIP_HWNDS.lock().remove(&self.hwnd);
        TRACKING_MOUSE.lock().remove(&self.hwnd);
        HOVERED_TAB.lock().remove(&self.hwnd);
        STACKBAR_TOOLTIP_LABELS.lock().remove(&self.hwnd);

        // Destroy tooltip window
        if self.tooltip_hwnd != 0 {
            let _ = WindowsApi::close_window(self.tooltip_hwnd);
        }

        WindowsApi::close_window(self.hwnd)
    }

    pub fn update(
        &self,
        container_padding: i32,
        container: &mut Container,
        layout: &Rect,
    ) -> color_eyre::Result<()> {
        let width = STACKBAR_TAB_WIDTH.load_consume();
        let height = STACKBAR_TAB_HEIGHT.load_consume();
        let gap = DEFAULT_CONTAINER_PADDING.load_consume();

        let mut stackbars_containers = STACKBARS_CONTAINERS.lock();
        stackbars_containers.insert(self.hwnd, container.clone());

        let mut layout = *layout;
        let workspace_specific_offset =
            BORDER_WIDTH.load_consume() + BORDER_OFFSET.load_consume() + container_padding;

        let is_top = matches!(STACKBAR_POSITION.load(), StackbarPosition::Top);
        if is_top {
            layout.top -= workspace_specific_offset + STACKBAR_TAB_HEIGHT.load_consume();
            layout.left -= workspace_specific_offset;
        } else {
            layout.top -= workspace_specific_offset;
            layout.left -= workspace_specific_offset + STACKBAR_VERTICAL_WIDTH.load_consume();
        }

        // Async causes the stackbar to disappear or flicker because we modify it right after,
        // so we have to do a synchronous call.
        // To avoid unneeded repaints and DWM flickering, only position if changed.
        let current_rect = WindowsApi::window_rect(self.hwnd).unwrap_or_default();
        if current_rect.left != layout.left
            || current_rect.top != layout.top
            || current_rect.right != layout.right
            || current_rect.bottom != layout.bottom
        {
            WindowsApi::position_window(self.hwnd, &layout, false, false)?;
        }

        unsafe {
            static INIT: std::sync::Once = std::sync::Once::new();
            INIT.call_once(|| {
                let mut token = 0;
                let input = GdiplusStartupInput {
                    GdiplusVersion: 1,
                    DebugEventCallback: 0,
                    SuppressBackgroundThread: BOOL(0),
                    SuppressExternalCodecs: BOOL(0),
                };
                let _ = GdiplusStartup(&mut token, &input, std::ptr::null_mut());
            });

            let window_rect = WindowsApi::window_rect(self.hwnd).unwrap_or_default();
            let win_width = window_rect.right;
            let win_height = window_rect.bottom;

            if win_width <= 0 || win_height <= 0 {
                return Ok(());
            }

            // Create a 32-bit top-down DIB so GDI+ writes proper per-pixel alpha.
            // Anti-aliased edge pixels blend against alpha=0 (fully transparent),
            // not against the colorkey black — this gives correct smooth corners.
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: win_width,
                    biHeight: -win_height, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut pvbits: *mut std::ffi::c_void = std::ptr::null_mut();
            let hbm_dib = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut pvbits, None, 0)
                .unwrap_or_default();
            // Zero-init: all pixels start as alpha=0 (fully transparent).
            if !pvbits.is_null() {
                std::ptr::write_bytes(pvbits as *mut u8, 0u8, (win_width * win_height * 4) as usize);
            }

            let hdc_mem = CreateCompatibleDC(None);
            let hbm_old = SelectObject(hdc_mem, hbm_dib.into());

            // Collect tooltip labels for this stackbar
            let mut labels: Vec<String> = Vec::new();

            for (i, window) in container.windows().iter().enumerate() {
                let is_top = matches!(STACKBAR_POSITION.load(), StackbarPosition::Top);
                let tab_width = if is_top { width } else { STACKBAR_VERTICAL_WIDTH.load_consume() };

                let rect = if is_top {
                    let left = gap + (i as i32 * (tab_width + gap));
                    Rect {
                        top: 0,
                        left,
                        right: left + tab_width,
                        bottom: height,
                    }
                } else {
                    let top = gap + (i as i32 * (height + gap));
                    Rect {
                        top,
                        left: 0,
                        right: tab_width,
                        bottom: top + height,
                    }
                };

                let background = if i == container.focused_window_idx() {
                    STACKBAR_TAB_FOCUSED_BACKGROUND_COLOUR.load_consume()
                } else {
                    STACKBAR_TAB_UNFOCUSED_BACKGROUND_COLOUR.load_consume()
                };

                let hpen = CreatePen(PS_SOLID, 0, COLORREF(background));
                let hbrush = CreateSolidBrush(COLORREF(background));
                SelectObject(hdc_mem, hpen.into());
                SelectObject(hdc_mem, hbrush.into());
                SetBkColor(hdc_mem, COLORREF(background));

                let mut graphics = std::ptr::null_mut();
                GdipCreateFromHDC(hdc_mem, &mut graphics);
                GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);

                let mut path = std::ptr::null_mut();
                GdipCreatePath(FillModeAlternate, &mut path);

                let dia = 20;
                GdipAddPathArcI(path, rect.left, rect.top, dia, dia, 180.0, 90.0);
                GdipAddPathArcI(path, rect.right - dia - 1, rect.top, dia, dia, 270.0, 90.0);
                GdipAddPathArcI(path, rect.right - dia - 1, rect.bottom - dia - 1, dia, dia, 0.0, 90.0);
                GdipAddPathArcI(path, rect.left, rect.bottom - dia - 1, dia, dia, 90.0, 90.0);
                GdipClosePathFigure(path);

                let r = (background & 0xFF) as u8;
                let g = ((background >> 8) & 0xFF) as u8;
                let b = ((background >> 16) & 0xFF) as u8;
                let argb = 0xFF_00_00_00 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

                let mut solid_brush = std::ptr::null_mut();
                GdipCreateSolidFill(argb, &mut solid_brush);
                let brush = solid_brush as *mut GpBrush;

                GdipFillPath(graphics, brush, path);

                GdipDeleteBrush(brush);
                GdipDeletePath(path);

                // Draw horizontal indicator line at the bottom of the active tab
                if i == container.focused_window_idx() {
                    let indicator_height = 2; // 2px high line
                    let indicator_width = 16;
                    let indicator_color = crate::border_manager::FOCUSED.load(std::sync::atomic::Ordering::SeqCst);
                    
                    let ir = (indicator_color & 0xFF) as u8;
                    let ig = ((indicator_color >> 8) & 0xFF) as u8;
                    let ib = ((indicator_color >> 16) & 0xFF) as u8;
                    let iargb = 0xFF_00_00_00 | ((ir as u32) << 16) | ((ig as u32) << 8) | (ib as u32);
                    
                    let mut i_brush = std::ptr::null_mut();
                    GdipCreateSolidFill(iargb, &mut i_brush);
                    let brush = i_brush as *mut GpBrush;

                    let mut i_path = std::ptr::null_mut();
                    GdipCreatePath(FillModeAlternate, &mut i_path);

                    let icon_x = rect.left + (tab_width - indicator_width) / 2;
                    let icon_y = rect.top + (height - 16) / 2;
                    let i_top = icon_y + 16 + 4; // Exactly 4 pixels below the icon

                    let i_rect = Rect {
                        top: i_top,
                        left: icon_x,
                        right: icon_x + indicator_width,
                        bottom: i_top + indicator_height,
                    };
                    
                    // Since we're in GDI+, let's just add a rectangle to the path and fill it
                    GdipAddPathArcI(i_path, i_rect.left, i_rect.top, 1, 1, 180.0, 90.0);
                    GdipAddPathArcI(i_path, i_rect.right - 1, i_rect.top, 1, 1, 270.0, 90.0);
                    GdipAddPathArcI(i_path, i_rect.right - 1, i_rect.bottom - 1, 1, 1, 0.0, 90.0);
                    GdipAddPathArcI(i_path, i_rect.left, i_rect.bottom - 1, 1, 1, 90.0, 90.0);
                    GdipClosePathFigure(i_path);
                    
                    GdipFillPath(graphics, brush, i_path);
                    
                    GdipDeletePath(i_path);
                    GdipDeleteBrush(brush);
                }

                GdipDeleteGraphics(graphics);

                let _ = DeleteObject(hpen.into());
                let _ = DeleteObject(hbrush.into());

                // Draw icon centered in the tab (no text).
                if let Ok(icon) = window.icon() {
                    let icon_size = 16;
                    let icon_x = rect.left + (tab_width - icon_size) / 2;
                    let icon_y = rect.top + (height - icon_size) / 2;
                    let _ = DrawIconEx(hdc_mem, icon_x, icon_y, icon, icon_size, icon_size, 0, None, DI_NORMAL);
                }

                // Build the tooltip label text for this tab.
                let label = match STACKBAR_LABEL.load() {
                    StackbarLabel::Process => {
                        let exe = window.exe().unwrap_or_default();
                        exe.trim_end_matches(".exe").to_string()
                    }
                    StackbarLabel::Title => window.title().unwrap_or_default(),
                };
                labels.push(label);
            }

            // Store labels in the shared map for the mouse-tracking callback
            STACKBAR_TOOLTIP_LABELS.lock().insert(self.hwnd, labels);

            // Push the DIB to the layered window using per-pixel alpha blending.
            // Must supply explicit pptDst + psize on every call (required on first call
            // and harmless on subsequent calls).
            let blend = BLENDFUNCTION {
                BlendOp: 0,   // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1, // AC_SRC_ALPHA
            };
            let src_pt = windows::Win32::Foundation::POINT { x: 0, y: 0 };
            let dst_pt = windows::Win32::Foundation::POINT { x: window_rect.left, y: window_rect.top };
            let size = windows::Win32::Foundation::SIZE { cx: win_width, cy: win_height };
            let _ = UpdateLayeredWindow(
                self.hwnd(),
                None,
                Some(&dst_pt),
                Some(&size),
                Some(hdc_mem),
                Some(&src_pt),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            // Cleanup
            SelectObject(hdc_mem, hbm_old);
            let _ = DeleteObject(hbm_dib.into());
            let _ = windows::Win32::Graphics::Gdi::DeleteDC(hdc_mem);
        }

        Ok(())
    }

    pub fn get_position_from_container_layout(&self, layout: &Rect) -> Rect {
        Rect {
            bottom: STACKBAR_TAB_HEIGHT.load_consume(),
            ..*layout
        }
    }

    /// Determine which tab index (0-based) the cursor at (x, y) is over, or -1 if none.
    fn hit_test_tab(x: i32, y: i32, window_count: usize) -> i32 {
        let width = STACKBAR_TAB_WIDTH.load_consume();
        let height = STACKBAR_TAB_HEIGHT.load_consume();
        let gap = DEFAULT_CONTAINER_PADDING.load_consume();
        let is_top = matches!(STACKBAR_POSITION.load(), StackbarPosition::Top);
        let tab_width = if is_top { width } else { STACKBAR_VERTICAL_WIDTH.load_consume() };

        for i in 0..window_count {
            let (left, right, top, bottom) = if is_top {
                let l = gap + (i as i32 * (tab_width + gap));
                (l, l + tab_width, 0, height)
            } else {
                let t = gap + (i as i32 * (height + gap));
                (0, tab_width, t, t + height)
            };

            if x >= left && x <= right && y >= top && y <= bottom {
                return i as i32;
            }
        }

        -1
    }

    unsafe extern "system" fn callback(
        hwnd: HWND,
        msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        unsafe {
            match msg {
                WM_SETCURSOR => {
                    let cursor = LoadCursorW(None, IDC_ARROW);
                    if let Ok(cursor) = cursor {
                        SetCursor(Some(cursor));
                    }
                    LRESULT(0)
                },
                WM_MOUSEMOVE => {
                    let hwnd_isize = hwnd.0 as isize;
                    let x = l_param.0 as i32 & 0xFFFF;
                    let y = (l_param.0 as i32 >> 16) & 0xFFFF;

                    // Register for WM_MOUSELEAVE if not already tracking
                    {
                        let mut tracking = TRACKING_MOUSE.lock();
                        if !tracking.get(&hwnd_isize).copied().unwrap_or(false) {
                            let mut tme = TRACKMOUSEEVENT {
                                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                                dwFlags: TME_LEAVE,
                                hwndTrack: hwnd,
                                dwHoverTime: 0,
                            };
                            let _ = TrackMouseEvent(&mut tme);
                            tracking.insert(hwnd_isize, true);
                        }
                    }

                    // Determine which tab is hovered
                    let window_count = {
                        let labels = STACKBAR_TOOLTIP_LABELS.lock();
                        labels.get(&hwnd_isize).map_or(0, |v| v.len())
                    };

                    let tab_idx = Self::hit_test_tab(x, y, window_count);
                    let prev_idx = HOVERED_TAB.lock().get(&hwnd_isize).copied().unwrap_or(-1);

                    if tab_idx != prev_idx {
                        HOVERED_TAB.lock().insert(hwnd_isize, tab_idx);

                        let tooltip_hwnd = TOOLTIP_HWNDS.lock().get(&hwnd_isize).copied().unwrap_or(0);

                        if tab_idx >= 0 {
                            let label = {
                                let labels = STACKBAR_TOOLTIP_LABELS.lock();
                                labels.get(&hwnd_isize)
                                    .and_then(|v| v.get(tab_idx as usize))
                                    .cloned()
                                    .unwrap_or_default()
                            };

                            if !label.is_empty() {
                                // Get the screen position of this stackbar window
                                let stackbar_rect = WindowsApi::window_rect(hwnd_isize).unwrap_or_default();
                                let is_top = matches!(STACKBAR_POSITION.load(), StackbarPosition::Top);

                                // Position tooltip below/beside the tab
                                let (tip_x, tip_y) = if is_top {
                                    let tab_width = STACKBAR_TAB_WIDTH.load_consume();
                                    let gap = DEFAULT_CONTAINER_PADDING.load_consume();
                                    let tab_left = gap + (tab_idx * (tab_width + gap));
                                    (
                                        stackbar_rect.left + tab_left,
                                        stackbar_rect.top + STACKBAR_TAB_HEIGHT.load_consume() + 4,
                                    )
                                } else {
                                    let height = STACKBAR_TAB_HEIGHT.load_consume();
                                    let gap = DEFAULT_CONTAINER_PADDING.load_consume();
                                    let tab_top = gap + (tab_idx * (height + gap));
                                    (
                                        stackbar_rect.left + STACKBAR_VERTICAL_WIDTH.load_consume() + 4,
                                        stackbar_rect.top + tab_top,
                                    )
                                };

                                Self::show_tooltip(tooltip_hwnd, &label, tip_x, tip_y);
                            }
                        } else {
                            let tooltip_hwnd = TOOLTIP_HWNDS.lock().get(&hwnd_isize).copied().unwrap_or(0);
                            Self::hide_tooltip(tooltip_hwnd);
                        }
                    }

                    LRESULT(0)
                }
                msg if msg == WM_MOUSELEAVE_MSG => {
                    let hwnd_isize = hwnd.0 as isize;
                    TRACKING_MOUSE.lock().insert(hwnd_isize, false);
                    HOVERED_TAB.lock().insert(hwnd_isize, -1);

                    let tooltip_hwnd = TOOLTIP_HWNDS.lock().get(&hwnd_isize).copied().unwrap_or(0);
                    Self::hide_tooltip(tooltip_hwnd);

                    LRESULT(0)
                }
                WM_LBUTTONDOWN => {
                    // Hide tooltip on click
                    let hwnd_isize = hwnd.0 as isize;
                    let tooltip_hwnd = TOOLTIP_HWNDS.lock().get(&hwnd_isize).copied().unwrap_or(0);
                    Self::hide_tooltip(tooltip_hwnd);
                    HOVERED_TAB.lock().insert(hwnd_isize, -1);

                    let stackbars_containers = STACKBARS_CONTAINERS.lock();
                    if let Some(container) = stackbars_containers.get(&(hwnd.0 as isize)) {
                        let x = l_param.0 as i32 & 0xFFFF;
                        let y = (l_param.0 as i32 >> 16) & 0xFFFF;

                        let width = STACKBAR_TAB_WIDTH.load_consume();
                        let height = STACKBAR_TAB_HEIGHT.load_consume();
                        let gap = DEFAULT_CONTAINER_PADDING.load_consume();

                        let focused_window_idx = container.focused_window_idx();
                        let focused_window_rect = WindowsApi::window_rect(
                            container.focused_window().cloned().unwrap_or_default().hwnd,
                        )
                        .unwrap_or_default();

                        let is_top = matches!(STACKBAR_POSITION.load(), StackbarPosition::Top);
                        let tab_width = if is_top { width } else { STACKBAR_VERTICAL_WIDTH.load_consume() };

                        for (index, window) in container.windows().iter().enumerate() {
                            let (left, right, top, bottom) = if is_top {
                                let l = gap + (index as i32 * (tab_width + gap));
                                (l, l + tab_width, 0, height)
                            } else {
                                let t = gap + (index as i32 * (height + gap));
                                (0, tab_width, t, t + height)
                            };

                            if x >= left && x <= right && y >= top && y <= bottom {
                                // If we are focusing a window that isn't currently focused in the
                                // stackbar, make sure we update its location so that it doesn't render
                                // on top of other tiles before eventually ending up in the correct
                                // tile
                                if index != focused_window_idx
                                    && let Err(err) =
                                        window.set_position(&focused_window_rect, false)
                                {
                                    tracing::error!(
                                        "stackbar WM_LBUTTONDOWN repositioning error: hwnd {} ({})",
                                        *window,
                                        err
                                    );
                                }

                                // Restore the window corresponding to the tab we have clicked
                                window.restore_with_border(false);
                                if let Err(err) = window.focus(false) {
                                    tracing::error!(
                                        "stackbar WMLBUTTONDOWN focus error: hwnd {} ({})",
                                        *window,
                                        err
                                    );
                                }
                            } else {
                                // Hide any windows in the stack that don't correspond to the window
                                // we have clicked
                                window.hide_with_border(false);
                            }
                        }
                    }

                    LRESULT(0)
                }
                WM_ERASEBKGND => {
                    // Suppress system background erasure to eliminate black flashes before update
                    LRESULT(1)
                }
                WM_DESTROY => {
                    PostQuitMessage(0);
                    LRESULT(0)
                }
                _ => DefWindowProcW(hwnd, msg, w_param, l_param),
            }
        }
    }
}
