#![windows_subsystem = "windows"]

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows_sys::core::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetCapture;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

struct AppState {
    display_name: Vec<u16>,
    dark_mode: bool,
    window_width: i32,
    drag_offset_x: i32,
    is_dragging: bool,
}

fn main() {
    unsafe {
        let name_str = get_windows_user_name();
        let display_name: Vec<u16> = name_str.encode_utf16().chain(std::iter::once(0)).collect();
        let dark_mode = is_windows_dark_theme();

        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = w!("NameBannerClass");

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            hCursor: LoadCursorW(0 as _, IDC_ARROW),
            hbrBackground: 0 as _,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hIcon: 0 as _,
        };

        RegisterClassW(&wnd_class);

        // Calculate text geometry
        let hdc = CreateDCW(w!("DISPLAY"), std::ptr::null(), std::ptr::null(), std::ptr::null());
        let font = CreateFontW(
            -14, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
            DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32, DEFAULT_QUALITY as u32,
            (DEFAULT_PITCH | FF_DONTCARE) as u32, w!("Segoe UI")
        );
        let old_font = SelectObject(hdc, font as _);

        let mut text_size = SIZE { cx: 0, cy: 0 };
        GetTextExtentPoint32W(hdc, display_name.as_ptr(), (display_name.len() - 1) as i32, &mut text_size);
        
        SelectObject(hdc, old_font);
        DeleteDC(hdc);

        let padding_x = 20;
        let padding_y = 6;
        let window_width = text_size.cx + padding_x;
        let window_height = text_size.cy + padding_y;

        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let initial_x = (screen_width / 2) - (window_width / 2);

        let state = Box::new(AppState {
            display_name,
            dark_mode,
            window_width,
            drag_offset_x: 0,
            is_dragging: false,
        });
        let state_ptr = Box::into_raw(state);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!(""),
            WS_POPUP | WS_VISIBLE,
            initial_x, 0, window_width, window_height,
            0 as _, 0 as _, instance,
            state_ptr as *const _ as *const std::ffi::c_void,
        );

        // Apply a smooth region mask: -12 top offset flushes the top edge while rounding bottom corners
        let corner_radius = 12;
        let rgn = CreateRoundRectRgn(0, -corner_radius, window_width + 1, window_height + 1, corner_radius, corner_radius);
        SetWindowRgn(hwnd, rgn, 1);

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
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        let state_ptr = (*cs).lpCreateParams as *mut AppState;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    if state_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let state = &mut *state_ptr;

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

            let (bg_color, fg_color, border_color) = if state.dark_mode {
                (0x001F1F1F, 0x00FFFFFF, 0x00888888)
            } else {
                (0x00F3F3F3, 0x00000000, 0x00555555)
            };

            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rect);

            // Create Pen (1px border) and Brush (background fill)
            let border_pen = CreatePen(PS_SOLID as i32, 1, border_color);
            let bg_brush = CreateSolidBrush(bg_color);

            let old_pen = SelectObject(hdc, border_pen as _);
            let old_brush = SelectObject(hdc, bg_brush as _);

            let radius = 12;

            // Draw a rounded rectangle using native GDI shape rendering
            // Top coordinate extends up to -radius so top corners remain square while bottom corners curve
            RoundRect(
                hdc,
                rect.left,
                rect.top - radius,
                rect.right,
                rect.bottom,
                radius,
                radius,
            );

            // Cleanup GDI objects
            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);
            DeleteObject(border_pen as _);
            DeleteObject(bg_brush as _);

            // Draw Name Text
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, fg_color);

            let font = CreateFontW(
                -14, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
                DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32, DEFAULT_QUALITY as u32,
                (DEFAULT_PITCH | FF_DONTCARE) as u32, w!("Segoe UI")
            );
            let old_font = SelectObject(hdc, font as _);

            DrawTextW(
                hdc,
                state.display_name.as_ptr(),
                (state.display_name.len() - 1) as i32,
                &rect as *const _ as *mut _,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            SelectObject(hdc, old_font);
            DeleteObject(font as _);

            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONDOWN => {
            state.is_dragging = true;
            SetCapture(hwnd);
            let x = (lparam & 0xFFFF) as i16 as i32;
            state.drag_offset_x = x;
            0
        }
        WM_MOUSEMOVE => {
            if state.is_dragging {
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);

                let mut new_x = pt.x - state.drag_offset_x;
                let screen_width = GetSystemMetrics(SM_CXSCREEN);
                let max_x = screen_width - state.window_width;

                if new_x < 0 { new_x = 0; }
                if new_x > max_x { new_x = max_x; }

                SetWindowPos(hwnd, 0 as _, new_x, 0, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
            }
            0
        }
        WM_LBUTTONUP => {
            state.is_dragging = false;
            windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
            0
        }
        WM_NCDESTROY => {
            let _ = Box::from_raw(state_ptr);
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
    
    if windows_sys::Win32::System::WindowsProgramming::GetUserNameW(buf.as_mut_ptr(), &mut len) != 0 {
        return OsString::from_wide(&buf[..(len as usize - 1)]).to_string_lossy().into_owned();
    }
    "User".to_string()
}