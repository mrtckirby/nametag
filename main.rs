#![windows_subsystem = "windows"] // Prevents a console window from popping up

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows_sys::core::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetCapture;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// Global state for horizontal dragging
static mut DRAG_OFFSET_X: i32 = 0;
static mut IS_DRAGGING: bool = false;
static mut WINDOW_WIDTH: i32 = 0;
static mut DARK_MODE: bool = true;
static mut DISPLAY_NAME: Vec<u16> = Vec::new();

fn main() {
    unsafe {
        // 1. Fetch Windows User Name
        let name_str = get_windows_user_name();
        DISPLAY_NAME = name_str.encode_utf16().chain(std::iter::once(0)).collect();

        // 2. Detect Windows Dark/Light Theme
        DARK_MODE = is_windows_dark_theme();

        let instance = GetModuleHandleW(PCWSTR::null());
        let class_name = w!("NameBannerClass");

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            hCursor: LoadCursorW(0 as _, IDC_ARROW), // Standard arrow cursor
            hbrBackground: 0 as _,
            lpszClassName: class_name,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hIcon: 0 as _,
        };

        RegisterClassW(&wnd_class);

        // Calculate text width to auto-size window
        let hdc = CreateDCW(w!("DISPLAY"), PCWSTR::null(), PCWSTR::null(), std::ptr::null());
        let font = CreateFontW(
            -15, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
            DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32, DEFAULT_QUALITY as u32,
            DEFAULT_PITCH | FF_DONTCARE, w!("Segoe UI")
        );
        let old_font = SelectObject(hdc, font as _);

        let mut text_size = SIZE { cx: 0, cy: 0 };
        GetTextExtentPoint32W(hdc, DISPLAY_NAME.as_ptr(), (DISPLAY_NAME.len() - 1) as i32, &mut text_size);
        
        SelectObject(hdc, old_font);
        DeleteDC(hdc);

        let padding_x = 24;
        let padding_y = 6;
        WINDOW_WIDTH = text_size.cx + padding_x;
        let window_height = text_size.cy + padding_y;

        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let initial_x = (screen_width / 2) - (WINDOW_WIDTH / 2);

        // WS_POPUP removes title bar, WS_EX_TOPMOST pins it above all windows
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!(""),
            WS_POPUP | WS_VISIBLE,
            initial_x, 0, WINDOW_WIDTH, window_height,
            0 as _, 0 as _, instance, std::ptr::null()
        );

        // Apply rounded bottom corners (radius: 12)
        let rgn = CreateRoundRectRgn(0, -12, WINDOW_WIDTH + 1, window_height + 1, 12, 12);
        SetWindowRgn(hwnd, rgn, 1);

        // Message Loop
        let mut msg = MSG {
            hwnd: 0 as _,
            message: 0,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: POINT { x: 0, y: 0 },
        };
        while GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT {
                hdc: 0 as _,
                fErase: 0,
                rcPaint: RECT { left: 0, top: 0, right: 0, bottom: 0 },
                fRestore: 0,
                fIncUpdate: 0,
                rgbReserved: [0; 32],
            };
            let hdc = BeginPaint(hwnd, &mut ps);

            let (bg_color, fg_color, border_color) = if DARK_MODE {
                (0x001F1F1F, 0x00FFFFFF, 0x00888888) // Dark theme (BGR format)
            } else {
                (0x00F3F3F3, 0x00000000, 0x00555555) // Light theme (BGR format)
            };

            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rect);

            // 1. Draw Contrasting Border Frame
            let border_brush = CreateSolidBrush(border_color);
            FillRect(hdc, &rect, border_brush);
            DeleteObject(border_brush as _);

            // 2. Inner Content Background
            let inner_rect = RECT {
                left: 1,
                top: 1,
                right: rect.right - 1,
                bottom: rect.bottom - 1,
            };
            let bg_brush = CreateSolidBrush(bg_color);
            FillRect(hdc, &inner_rect, bg_brush);
            DeleteObject(bg_brush as _);

            // 3. Draw Name Text
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, fg_color);

            let font = CreateFontW(
                -15, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
                DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32, DEFAULT_QUALITY as u32,
                DEFAULT_PITCH | FF_DONTCARE, w!("Segoe UI")
            );
            let old_font = SelectObject(hdc, font as _);

            DrawTextW(
                hdc,
                DISPLAY_NAME.as_ptr(),
                (DISPLAY_NAME.len() - 1) as i32,
                &rect as *const _ as *mut _,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            SelectObject(hdc, old_font);
            DeleteObject(font as _);

            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONDOWN => {
            IS_DRAGGING = true;
            SetCapture(hwnd);
            let x = (lparam & 0xFFFF) as i16 as i32;
            DRAG_OFFSET_X = x;
            0
        }
        WM_MOUSEMOVE => {
            if IS_DRAGGING {
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);

                let mut new_x = pt.x - DRAG_OFFSET_X;
                let screen_width = GetSystemMetrics(SM_CXSCREEN);
                let max_x = screen_width - WINDOW_WIDTH;

                // Clamp within screen horizontal bounds
                if new_x < 0 { new_x = 0; }
                if new_x > max_x { new_x = max_x; }

                // Lock to top edge y = 0
                SetWindowPos(hwnd, 0 as _, new_x, 0, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
            }
            0
        }
        WM_LBUTTONUP => {
            IS_DRAGGING = false;
            windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn is_windows_dark_theme() -> bool {
    let mut key = 0 as _;
    let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, &mut key) == 0 {
        let mut data: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let value_name = w!("AppsUseLightTheme");
        let status = RegQueryValueExW(key, value_name, std::ptr::null_mut(), std::ptr::null_mut(), &mut data as *mut _ as *mut _, &mut size);
        RegCloseKey(key);
        if status == 0 {
            return data == 0;
        }
    }
    true
}

unsafe fn get_windows_user_name() -> String {
    let mut len: u32 = 257;
    let mut buf = vec![0u16; len as usize];
    
    // GetUserNameW (advapi32)
    if windows_sys::Win32::System::WindowsProgramming::GetUserNameW(buf.as_mut_ptr(), &mut len) != 0 {
        return OsString::from_wide(&buf[..(len as usize - 1)]).to_string_lossy().into_owned();
    }
    "User".to_string()
}