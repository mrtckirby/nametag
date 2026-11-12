// Build as a GUI-subsystem executable so no console window appears when launched.
#![windows_subsystem = "windows"]

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows_sys::core::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_MENU};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// Font height in logical units (negative = character height rather than cell height).
const FONT_HEIGHT: i32 = -16;
// Radius used for the rounded bottom corners of the banner.
const CORNER_RADIUS: i32 = 12;

// Per-window state. A heap-allocated instance is attached to the window
// (via GWLP_USERDATA) so the window procedure can reach it on every message.
struct AppState {
    // Logged-on user's name as a NUL-terminated UTF-16 string (what Win32 expects).
    display_name: Vec<u16>,
    // Width of the window, used to keep it on-screen while dragging.
    window_width: i32,
    // X offset of the mouse within the window at the moment the drag started.
    drag_offset_x: i32,
    // True while the left mouse button is held down for dragging.
    is_dragging: bool,
}

// Creates the bold Segoe UI font used both for measuring and for painting.
// The caller owns the returned handle and must DeleteObject it.
unsafe fn create_font() -> HFONT {
    CreateFontW(
        FONT_HEIGHT, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
        DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32, DEFAULT_QUALITY as u32,
        (DEFAULT_PITCH | FF_DONTCARE) as u32, w!("Segoe UI"),
    )
}

fn main() {
    // All Win32 calls are FFI and therefore unsafe.
    unsafe {
        // Look up the current user's name and convert it to a NUL-terminated UTF-16 buffer.
        let name_str = get_windows_user_name();
        let display_name: Vec<u16> = name_str.encode_utf16().chain(std::iter::once(0)).collect();

        // Handle of this executable, needed to register and create the window.
        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = w!("NameBannerClass");

        // Describe the window class: redraw on resize, route messages to wnd_proc,
        // arrow cursor, and no background brush (we paint everything ourselves).
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

        // Measure the text so the window can be sized to fit the name.
        // A device context for the whole display is enough for measuring.
        let hdc = CreateDCW(w!("DISPLAY"), std::ptr::null(), std::ptr::null(), std::ptr::null());
        let font = create_font();
        let old_font = SelectObject(hdc, font as _);

        let mut text_size = SIZE { cx: 0, cy: 0 };
        // Length excludes the trailing NUL.
        GetTextExtentPoint32W(hdc, display_name.as_ptr(), (display_name.len() - 1) as i32, &mut text_size);

        // Restore and release GDI objects.
        SelectObject(hdc, old_font);
        DeleteObject(font as _);
        DeleteDC(hdc);

        // Add padding around the text to get the final window size.
        let padding_x = 20;
        let padding_y = 6;
        let window_width = text_size.cx + padding_x;
        let window_height = text_size.cy + padding_y;

        // Start horizontally centred at the top of the primary screen.
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let initial_x = (screen_width / 2) - (window_width / 2);

        // Build the state and leak it as a raw pointer; it is handed to the window
        // and freed again in WM_NCDESTROY.
        let state = Box::new(AppState {
            display_name,
            window_width,
            drag_offset_x: 0,
            is_dragging: false,
        });
        let state_ptr = Box::into_raw(state);

        // Borderless popup that is always on top and hidden from the taskbar / Alt-Tab.
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!(""),
            WS_POPUP | WS_VISIBLE,
            initial_x, 0, window_width, window_height,
            0 as _, 0 as _, instance,
            // Passed through to WM_NCCREATE as CREATESTRUCTW::lpCreateParams.
            state_ptr as *const _ as *const std::ffi::c_void,
        );

        // Clip the window to a rounded shape. The -CORNER_RADIUS top offset pushes the
        // top rounded corners off-screen, flush with the screen edge, leaving only the
        // bottom corners rounded.
        let rgn = CreateRoundRectRgn(0, -CORNER_RADIUS, window_width + 1, window_height + 1, CORNER_RADIUS, CORNER_RADIUS);
        SetWindowRgn(hwnd, rgn, 1);

        // Standard message loop: runs until PostQuitMessage makes GetMessageW return 0.
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

// Window procedure: handles every message sent to the banner window.
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // First message with creation data: stash our state pointer in the window.
    if msg == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTW;
        let state_ptr = (*cs).lpCreateParams as *mut AppState;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    // Retrieve the state; messages arriving before WM_NCCREATE get default handling.
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    if state_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let state = &mut *state_ptr;

    match msg {
        // Draw the banner: rounded background, border and the user's name.
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

            // Colours are GDI COLORREFs in 0x00BBGGRR order: light background,
            // black text, grey border.
            let bg_color = 0x00F3F3F3;
            let fg_color = 0x00000000;
            let border_color = 0x00555555;

            // Size of the drawable area.
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rect);

            // Create Pen (1px border) and Brush (background fill)
            let border_pen = CreatePen(PS_SOLID as i32, 1, border_color);
            let bg_brush = CreateSolidBrush(bg_color);

            // Select them into the DC, remembering the originals to restore later.
            let old_pen = SelectObject(hdc, border_pen as _);
            let old_brush = SelectObject(hdc, bg_brush as _);

            // Draw a rounded rectangle using native GDI shape rendering.
            // Top coordinate extends up to -radius so top corners remain square while bottom corners curve
            RoundRect(
                hdc,
                rect.left,
                rect.top - CORNER_RADIUS,
                rect.right,
                rect.bottom,
                CORNER_RADIUS,
                CORNER_RADIUS,
            );

            // Cleanup GDI objects
            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);
            DeleteObject(border_pen as _);
            DeleteObject(bg_brush as _);

            // Draw the name with a transparent text background so the fill shows through.
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, fg_color);

            let font = create_font();
            let old_font = SelectObject(hdc, font as _);

            // Centre the text horizontally and vertically on a single line.
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
        // Left click: Ctrl+Alt+click exits, otherwise begin dragging.
        WM_LBUTTONDOWN => {
            // The high bit of GetKeyState's result means the key is currently down.
            let ctrl_down = GetKeyState(VK_CONTROL as i32) < 0;
            let alt_down = GetKeyState(VK_MENU as i32) < 0;
            if ctrl_down && alt_down {
                // Destroying the window ends in WM_NCDESTROY -> PostQuitMessage.
                DestroyWindow(hwnd);
                return 0;
            }

            state.is_dragging = true;
            // Capture the mouse so we keep getting move events outside the window.
            SetCapture(hwnd);
            // Client-area X is the low word of lparam (signed 16-bit).
            let x = (lparam & 0xFFFF) as i16 as i32;
            state.drag_offset_x = x;
            0
        }
        // Mouse move: while dragging, slide the window horizontally only.
        WM_MOUSEMOVE => {
            if state.is_dragging {
                // Use the screen-space cursor position, minus where we grabbed the window.
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);

                let mut new_x = pt.x - state.drag_offset_x;
                let screen_width = GetSystemMetrics(SM_CXSCREEN);
                let max_x = screen_width - state.window_width;

                // Keep the banner fully on-screen.
                if new_x < 0 { new_x = 0; }
                if new_x > max_x { new_x = max_x; }

                // Y stays pinned at 0 (top of the screen); size and z-order unchanged.
                SetWindowPos(hwnd, 0 as _, new_x, 0, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
            }
            0
        }
        // Button released: stop dragging and give the mouse back.
        WM_LBUTTONUP => {
            state.is_dragging = false;
            ReleaseCapture();
            0
        }
        // Final message: free the state allocated in main and end the message loop.
        WM_NCDESTROY => {
            let _ = Box::from_raw(state_ptr);
            PostQuitMessage(0);
            0
        }
        // Anything else gets default Windows handling.
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// Returns the name of the logged-on Windows user, or "User" if the lookup fails.
unsafe fn get_windows_user_name() -> String {
    // UNLEN (256) + NUL terminator.
    let mut len: u32 = 257;
    let mut buf = vec![0u16; len as usize];

    // On success `len` is updated to the length including the trailing NUL.
    if windows_sys::Win32::System::WindowsProgramming::GetUserNameW(buf.as_mut_ptr(), &mut len) != 0 {
        return OsString::from_wide(&buf[..(len as usize - 1)]).to_string_lossy().into_owned();
    }
    "User".to_string()
}
