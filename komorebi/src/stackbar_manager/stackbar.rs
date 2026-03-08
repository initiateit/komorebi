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
use crate::windows_api;
use crossbeam_utils::atomic::AtomicConsume;
use std::sync::mpsc;
use std::time::Duration;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::Graphics::Gdi::CreatePen;
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
use windows::Win32::UI::Controls::{
    TTTOOLINFOW, TTF_SUBCLASS, TTF_TRANSPARENT, TTS_ALWAYSTIP, TTS_NOPREFIX,
    TTM_ADDTOOLW, TTM_DELTOOLW, TTM_SETMAXTIPWIDTH,
};
use windows::Win32::UI::WindowsAndMessaging::SendMessageW;
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
use windows::Win32::UI::WindowsAndMessaging::WM_SETCURSOR;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_POPUP;
use windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE;
use windows::Win32::UI::WindowsAndMessaging::DrawIconEx;
use windows::Win32::UI::WindowsAndMessaging::DI_NORMAL;
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use windows::Win32::UI::WindowsAndMessaging::SetWindowPos;
use windows::Win32::UI::WindowsAndMessaging::HWND_TOPMOST;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE;
use windows::Win32::Graphics::GdiPlus::{
    FillModeAlternate, GdipAddPathArcI, GdipClosePathFigure, GdipCreateFromHDC,
    GdipCreatePath, GdipCreateSolidFill, GdipDeleteBrush, GdipDeleteGraphics,
    GdipDeletePath, GdipFillPath, GdipSetSmoothingMode, GdiplusStartup,
    GdiplusStartupInput, GpBrush, SmoothingModeAntiAlias,
};
use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE};
use windows_core::BOOL;
use windows::core::PCWSTR;

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

                // Create an always-on-top tooltip window owned by the stackbar.
                let tooltip_class: Vec<u16> = "tooltips_class32\0".encode_utf16().collect();
                let tooltip = CreateWindowExW(
                    WS_EX_TOPMOST,
                    PCWSTR(tooltip_class.as_ptr()),
                    PCWSTR::null(),
                    WINDOW_STYLE(WS_POPUP.0 | TTS_ALWAYSTIP | TTS_NOPREFIX),
                    CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
                    Some(hwnd),
                    None,
                    None,
                    None,
                ).unwrap_or_default();
                let _ = SetWindowPos(
                    tooltip,
                    Some(HWND_TOPMOST),
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                // Allow multi-line tooltips up to 300px wide
                SendMessageW(tooltip, TTM_SETMAXTIPWIDTH, Some(WPARAM(0)), Some(LPARAM(300)));

                hwnd_sender.send((hwnd.0 as isize, tooltip.0 as isize))?;

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

            // We cannot select a global background brush here because we need it per tab.
            // But we can clear the background to transparent or to a default color if needed.

            // Clear all existing tooltip tools so stale rects don't linger.
            // We delete a generous upper bound of tool IDs — deleteing a non-existent ID is harmless.
            let tooltip_hwnd = HWND(windows_api::as_ptr!(self.tooltip_hwnd));
            for j in 0usize..64 {
                let mut ti = TTTOOLINFOW {
                    cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
                    hwnd: self.hwnd(),
                    uId: j,
                    ..Default::default()
                };
                SendMessageW(tooltip_hwnd, TTM_DELTOOLW, Some(WPARAM(0)), Some(LPARAM(&mut ti as *mut _ as isize)));
            }

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

                // Register this tab rect as a tooltip tool with the window label.
                let label = match STACKBAR_LABEL.load() {
                    StackbarLabel::Process => {
                        let exe = window.exe()?;
                        exe.trim_end_matches(".exe").to_string()
                    }
                    StackbarLabel::Title => window.title()?,
                };
                let mut tip_text: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                let mut ti = TTTOOLINFOW {
                    cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
                    uFlags: TTF_SUBCLASS | TTF_TRANSPARENT,
                    hwnd: self.hwnd(),
                    uId: i as usize,
                    rect: windows::Win32::Foundation::RECT {
                        left: rect.left,
                        top: rect.top,
                        right: rect.right,
                        bottom: rect.bottom,
                    },
                    lpszText: windows::core::PWSTR(tip_text.as_mut_ptr()),
                    ..Default::default()
                };
                SendMessageW(tooltip_hwnd, TTM_ADDTOOLW, Some(WPARAM(0)), Some(LPARAM(&mut ti as *mut _ as isize)));
            }

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

    unsafe extern "system" fn callback(
        hwnd: HWND,
        msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        unsafe {
            match msg {
                WM_SETCURSOR => match LoadCursorW(None, IDC_ARROW) {
                    Ok(cursor) => {
                        SetCursor(Some(cursor));
                        LRESULT(0)
                    }
                    Err(error) => {
                        tracing::error!("{error}");
                        LRESULT(1)
                    }
                },
                WM_LBUTTONDOWN => {
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
