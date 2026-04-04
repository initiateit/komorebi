#![deny(clippy::unwrap_used, clippy::expect_used)]

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows_core::PCWSTR;
use komorebi_themes::colour::Colour;

pub static MONOCLE_BACKDROP_BLUR: AtomicBool = AtomicBool::new(false);
pub static MONOCLE_BACKDROP_COLOR: Mutex<Colour> = Mutex::new(Colour::Rgb(komorebi_themes::colour::Rgb::new(0, 0, 0)));
pub static MONOCLE_BACKDROP_ALPHA: Mutex<u8> = Mutex::new(180);

static CHANNEL: OnceLock<(Sender<Notification>, Receiver<Notification>)> = OnceLock::new();
static BLUR_HWNDS: OnceLock<Mutex<Vec<(isize, usize)>>> = OnceLock::new(); // (hwnd, monitor_index)

pub struct Notification;

pub fn channel() -> &'static (Sender<Notification>, Receiver<Notification>) {
    CHANNEL.get_or_init(|| crossbeam_channel::bounded(20))
}

fn event_tx() -> Sender<Notification> {
    channel().0.clone()
}

fn event_rx() -> Receiver<Notification> {
    channel().1.clone()
}

pub fn send_notification() {
    let _ = event_tx().try_send(Notification);
}

pub fn listen_for_notifications(wm: Arc<Mutex<crate::WindowManager>>) {
    std::thread::spawn(move || {
        loop {
            match handle_notifications(wm.clone()) {
                Ok(()) => {
                    tracing::warn!("restarting finished thread");
                }
                Err(error) => {
                    tracing::warn!("restarting failed thread: {}", error);
                }
            }
        }
    });
}

unsafe extern "system" fn blur_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn create_blur_window(x: i32, y: i32, width: i32, height: i32) -> Option<isize> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    unsafe {
        let class_name: Vec<u16> =
            OsStr::new("KomorebiBlurOverlay").encode_wide().chain(Some(0)).collect();

        let instance = match windows::Win32::System::LibraryLoader::GetModuleHandleW(PCWSTR::null()) {
            Ok(h) => h,
            Err(_) => return None,
        };

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(blur_wnd_proc),
            hInstance: windows::Win32::Foundation::HINSTANCE(instance.0),
            hCursor: match LoadCursorW(None, IDC_ARROW) {
                Ok(h) => h,
                Err(_) => Default::default(),
            },
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        let _ = RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            x,
            y,
            width,
            height,
            None,
            None,
            Some(wc.hInstance),
            None,
        );

        let hwnd = hwnd.ok()?;

        // Get configuration values
        let alpha = *MONOCLE_BACKDROP_ALPHA.lock();
        let color = MONOCLE_BACKDROP_COLOR.lock().clone();

        // Convert Colour to RGB value
        let rgb = match &color {
            Colour::Rgb(rgb) => ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32),
            Colour::Hex(_) => 0,
        };

        // Create a solid brush with the configured color
        let brush = CreateSolidBrush(COLORREF(rgb));

        // Set the window background to the solid color
        let _ = SetClassLongPtrW(hwnd, GCL_HBRBACKGROUND, brush.0 as isize);

        // Make the window transparent with the configured alpha
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(rgb), alpha, LWA_ALPHA);

        // Position at bottom of Z-order and show
        let _ = SetWindowPos(hwnd, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);

        Some(hwnd.0 as isize)
    }
}

pub fn handle_notifications(wm: Arc<Mutex<crate::WindowManager>>) -> color_eyre::Result<()> {
    tracing::info!("blur overlay listening");

    let receiver = event_rx();
    let _ = event_tx().send(Notification);

    for _ in receiver {
        tracing::debug!("Received blur overlay notification");

        if !MONOCLE_BACKDROP_BLUR.load(Ordering::Relaxed) {
            tracing::debug!("Backdrop blur disabled, cleaning up windows");
            let mut hwnds = BLUR_HWNDS.get_or_init(|| Mutex::new(Vec::new())).lock();
            for (hwnd, _) in hwnds.drain(..) {
                unsafe {
                    let h = HWND(hwnd as *mut _);
                    let _ = ShowWindow(h, SW_HIDE);
                    let _ = DestroyWindow(h);
                }
            }
            continue;
        }

        let state = wm.lock();

        // Collect monitors with active monocle workspaces
        let monitors_with_monocle: Vec<_> = state
            .monitors
            .elements()
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                m.focused_workspace()
                    .and_then(|ws| ws.monocle_container.as_ref())
                    .is_some()
            })
            .map(|(i, m)| (i, m.size))
            .collect();

        drop(state);

        tracing::debug!("Monocle active on {} monitors: {:?}", monitors_with_monocle.len(), monitors_with_monocle.iter().map(|(i, _)| i).collect::<Vec<_>>());

        let mut hwnds = BLUR_HWNDS.get_or_init(|| Mutex::new(Vec::new())).lock();

        // Remove windows for monitors that no longer have monocle
        hwnds.retain(|window_info| {
            let (hwnd, monitor_index) = *window_info;
            if monitors_with_monocle.iter().any(|(i, _)| *i == monitor_index) {
                true // Keep this window
            } else {
                tracing::info!("Destroying blur window for monitor {}", monitor_index);
                unsafe {
                    let h = HWND(hwnd as *mut _);
                    let _ = ShowWindow(h, SW_HIDE);
                    let _ = DestroyWindow(h);
                }
                false // Remove this window
            }
        });

        // Create windows for monitors that have monocle but no window
        for (monitor_index, rect) in &monitors_with_monocle {
            if !hwnds.iter().any(|(_, i)| *i == *monitor_index) {
                let x = rect.left;
                let y = rect.top;
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;
                tracing::info!("Creating blur window for monitor {}: {}x{} at ({},{})", monitor_index, width, height, x, y);
                let hwnd = create_blur_window(x, y, width, height);
                if let Some(h) = hwnd {
                    tracing::info!("Created blur window for monitor {}: {}", monitor_index, h);
                    hwnds.push((h, *monitor_index));
                } else {
                    tracing::warn!("Failed to create blur window for monitor {}", monitor_index);
                }
            } else {
                // Show existing window for this monitor
                if let Some((hwnd, _)) = hwnds.iter().find(|(_, i)| *i == *monitor_index) {
                    unsafe {
                        let h = HWND(*hwnd as *mut _);
                        let _ = ShowWindow(h, SW_SHOWNA);
                        let _ = SetWindowPos(
                            h,
                            Some(HWND_BOTTOM),
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
