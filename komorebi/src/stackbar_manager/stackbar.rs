use crate::DEFAULT_CONTAINER_PADDING;
use crate::WINDOWS_11;
use crate::WindowsApi;
use crate::border_manager::BORDER_OFFSET;
use crate::border_manager::BORDER_WIDTH;
use crate::border_manager::STYLE;
use crate::container::Container;
use crate::core::BorderStyle;
use crate::core::Rect;
use crate::core::StackbarLabel;
use crate::core::StackbarPosition;
use crate::stackbar_manager::STACKBAR_FOCUSED_TEXT_COLOUR;
use crate::stackbar_manager::STACKBAR_FONT_FAMILY;
use crate::stackbar_manager::STACKBAR_FONT_SIZE;
use crate::stackbar_manager::STACKBAR_LABEL;
use crate::stackbar_manager::STACKBAR_TAB_BACKGROUND_COLOUR;
use crate::stackbar_manager::STACKBAR_TAB_HEIGHT;
use crate::stackbar_manager::STACKBAR_TAB_WIDTH;
use crate::stackbar_manager::STACKBAR_VERTICAL_WIDTH;
use crate::stackbar_manager::STACKBAR_POSITION;
use crate::stackbar_manager::STACKBAR_UNFOCUSED_TEXT_COLOUR;
use crate::stackbar_manager::STACKBARS_CONTAINERS;
use crate::windows_api;
use crossbeam_utils::atomic::AtomicConsume;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::Graphics::Gdi::CreateFontIndirectW;
use windows::Win32::Graphics::Gdi::CreatePen;
use windows::Win32::Graphics::Gdi::CreateSolidBrush;
use windows::Win32::Graphics::Gdi::DT_CENTER;
use windows::Win32::Graphics::Gdi::DT_END_ELLIPSIS;
use windows::Win32::Graphics::Gdi::DT_SINGLELINE;
use windows::Win32::Graphics::Gdi::DT_VCENTER;
use windows::Win32::Graphics::Gdi::DeleteObject;
use windows::Win32::Graphics::Gdi::DrawTextW;
use windows::Win32::Graphics::Gdi::FONT_QUALITY;
use windows::Win32::Graphics::Gdi::FW_BOLD;
use windows::Win32::Graphics::Gdi::GetDC;
use windows::Win32::Graphics::Gdi::GetDeviceCaps;
use windows::Win32::Graphics::Gdi::LOGFONTW;
use windows::Win32::Graphics::Gdi::LOGPIXELSY;
use windows::Win32::Graphics::Gdi::PROOF_QUALITY;
use windows::Win32::Graphics::Gdi::PS_SOLID;
use windows::Win32::Graphics::Gdi::Rectangle;
use windows::Win32::Graphics::Gdi::ReleaseDC;
use windows::Win32::Graphics::Gdi::RoundRect;
use windows::Win32::Graphics::Gdi::SelectObject;
use windows::Win32::Graphics::Gdi::SetBkColor;
use windows::Win32::Graphics::Gdi::SetTextColor;
use windows::Win32::Graphics::Gdi::CreateCompatibleBitmap;
use windows::Win32::Graphics::Gdi::CreateCompatibleDC;
use windows::Win32::System::WindowsProgramming::MulDiv;
use windows::Win32::UI::WindowsAndMessaging::CS_HREDRAW;
use windows::Win32::UI::WindowsAndMessaging::CS_VREDRAW;
use windows::Win32::UI::WindowsAndMessaging::CreateWindowExW;
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows::Win32::UI::WindowsAndMessaging::DispatchMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetMessageW;
use windows::Win32::UI::WindowsAndMessaging::IDC_ARROW;
use windows::Win32::UI::WindowsAndMessaging::LWA_COLORKEY;
use windows::Win32::UI::WindowsAndMessaging::LoadCursorW;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use windows::Win32::UI::WindowsAndMessaging::SetCursor;
use windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes;
use windows::Win32::UI::WindowsAndMessaging::TranslateMessage;
use windows::Win32::UI::WindowsAndMessaging::WM_DESTROY;
use windows::Win32::UI::WindowsAndMessaging::WM_ERASEBKGND;
use windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONDOWN;
use windows::Win32::UI::WindowsAndMessaging::WM_SETCURSOR;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_POPUP;
use windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE;
use windows::Win32::UI::WindowsAndMessaging::DrawIconEx;
use windows::Win32::UI::WindowsAndMessaging::DI_NORMAL;
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
}

impl From<isize> for Stackbar {
    fn from(value: isize) -> Self {
        Self { hwnd: value }
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

                SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_COLORKEY)?;
                let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
                hwnd_sender.send(hwnd.0 as isize)?;

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

        Ok(Self {
            hwnd: hwnd_receiver.recv()?,
        })
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
        let background = STACKBAR_TAB_BACKGROUND_COLOUR.load_consume();
        let focused_text_colour = STACKBAR_FOCUSED_TEXT_COLOUR.load_consume();
        let unfocused_text_colour = STACKBAR_UNFOCUSED_TEXT_COLOUR.load_consume();

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
        let p_width = layout.right - layout.left;
        let p_height = layout.bottom - layout.top;
        if current_rect.left != layout.left
            || current_rect.top != layout.top
            || (current_rect.right - current_rect.left) != p_width
            || (current_rect.bottom - current_rect.top) != p_height
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

            let hdc = GetDC(Option::from(self.hwnd()));
            let hdc_screen = GetDC(None);

            // Double Buffering: Create an offscreen device context and a bitmap to match the window size.
            let window_rect = WindowsApi::window_rect(self.hwnd).unwrap_or_default();
            let win_width = window_rect.right - window_rect.left;
            let win_height = window_rect.bottom - window_rect.top;
            
            let hdc_mem = CreateCompatibleDC(Option::from(hdc));
            let hbm_mem = CreateCompatibleBitmap(hdc, win_width, win_height);
            let hbm_old = SelectObject(hdc_mem, hbm_mem.into());

            // By default, make the entire memory bitmap transparent (COLORKEY)
            let hbrush_clear = CreateSolidBrush(COLORREF(0));
            let mut mem_rect = windows::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: win_width,
                bottom: win_height,
            };
            windows::Win32::Graphics::Gdi::FillRect(hdc_mem, &mut mem_rect, hbrush_clear.into());
            let _ = DeleteObject(hbrush_clear.into());

            let hpen = CreatePen(PS_SOLID, 0, COLORREF(background));
            let hbrush = CreateSolidBrush(COLORREF(background));

            SelectObject(hdc_mem, hpen.into());
            SelectObject(hdc_mem, hbrush.into());
            SetBkColor(hdc_mem, COLORREF(background));

            let mut logfont = LOGFONTW {
                lfWeight: FW_BOLD.0 as i32,
                lfQuality: FONT_QUALITY(PROOF_QUALITY.0),
                lfFaceName: [0; 32],
                ..Default::default()
            };

            if let Some(font_name) = &*STACKBAR_FONT_FAMILY.lock() {
                let font = wide_string(font_name);
                for (i, &c) in font.iter().enumerate() {
                    logfont.lfFaceName[i] = c;
                }
            }

            let logical_height = -MulDiv(
                STACKBAR_FONT_SIZE.load(Ordering::SeqCst),
                72,
                GetDeviceCaps(Option::from(hdc), LOGPIXELSY),
            );

            logfont.lfHeight = logical_height;

            let hfont = CreateFontIndirectW(&logfont);

            SelectObject(hdc_mem, hfont.into());

            for (i, window) in container.windows().iter().enumerate() {
                if window.hwnd == container.focused_window().copied().unwrap_or_default().hwnd {
                    SetTextColor(hdc_mem, COLORREF(focused_text_colour));
                } else {
                    SetTextColor(hdc_mem, COLORREF(unfocused_text_colour));
                }

                let is_top = matches!(STACKBAR_POSITION.load(), StackbarPosition::Top);
                let tab_width = if is_top { width } else { STACKBAR_VERTICAL_WIDTH.load_consume() };

                let mut rect = if is_top {
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

                // For double buffering, we capture the screen underneath the window into our
                // memory DC so the GDI+ anti-aliasing can blend against it smoothly.
                let _ = windows::Win32::Graphics::Gdi::BitBlt(
                    hdc_mem,
                    rect.left, rect.top,
                    rect.right - rect.left, rect.bottom - rect.top,
                    Option::from(hdc_screen),
                    window_rect.left + rect.left, window_rect.top + rect.top,
                    windows::Win32::Graphics::Gdi::SRCCOPY,
                );

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
                GdipDeleteGraphics(graphics);

                let label = match STACKBAR_LABEL.load() {
                    StackbarLabel::Process => {
                        let exe = window.exe()?;
                        exe.trim_end_matches(".exe").to_string()
                    }
                    StackbarLabel::Title => window.title()?,
                };

                let mut tab_title: Vec<u16> = label.encode_utf16().collect();

                if let Ok(icon) = window.icon() {
                    // Draw the 16x16 icon on the left side, centered vertically
                    let icon_size = 16;
                    let icon_x = rect.left + 10;
                    let icon_y = rect.top + (height - icon_size) / 2;
                    
                    let _ = DrawIconEx(hdc_mem, icon_x, icon_y, icon, icon_size, icon_size, 0, None, DI_NORMAL);
                    
                    rect.left_padding(icon_size + 15);
                } else {
                    rect.left_padding(10);
                }
                
                rect.right_padding(10);

                DrawTextW(
                    hdc_mem,
                    &mut tab_title,
                    &mut rect.into(),
                    DT_SINGLELINE | DT_CENTER | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }

            // Copy the fully constructed offscreen bitmap to the physical screen DC
            let _ = windows::Win32::Graphics::Gdi::BitBlt(
                hdc,
                0, 0,
                win_width, win_height,
                Option::from(hdc_mem),
                0, 0,
                windows::Win32::Graphics::Gdi::SRCCOPY,
            );

            // Cleanup Double Buffering resources
            SelectObject(hdc_mem, hbm_old);
            let _ = DeleteObject(hbm_mem.into());
            let _ = windows::Win32::Graphics::Gdi::DeleteDC(hdc_mem);

            ReleaseDC(None, hdc_screen);
            ReleaseDC(Option::from(self.hwnd()), hdc);
            let _ = DeleteObject(hpen.into());
            // TODO: error handling
            let _ = DeleteObject(hbrush.into());
            // TODO: error handling
            let _ = DeleteObject(hfont.into());
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

fn wide_string(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
