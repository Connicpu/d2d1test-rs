use winapi::*;
use user32::*;
use kernel32::*;
use wstr::*;
use std::mem;

pub trait WindowProcHandler {
    fn wnd_proc(&mut self, hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
}

impl WindowProcHandler for () {
    fn wnd_proc(&mut self, hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}

pub type HandlerPtr = Box<WindowProcHandler>;
type StoredHandler = Option<Box<HandlerPtr>>;
type StoredHandlerRef<'a> = Option<&'a mut HandlerPtr>;

unsafe extern "system" fn static_wnd_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let default_handling = || {
        let handler_lpv = GetWindowLongPtrW(hwnd, GWL_USERDATA);
        let handler: StoredHandlerRef = mem::transmute(handler_lpv);
        if let Some(handler) = handler {
            handler.wnd_proc(hwnd, msg, wparam, lparam)
        } else {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    };

    match msg {
        WM_CREATE => {
            let data: &CREATESTRUCTW = mem::transmute(lparam);
            let handler = data.lpCreateParams;
            SetWindowLongPtrW(hwnd, GWL_USERDATA, mem::transmute(handler));

            default_handling()
        },
        WM_DESTROY => {
            let result = default_handling();
            let _: StoredHandler = mem::transmute(GetWindowLongPtrW(hwnd, GWL_USERDATA));
            let handler: StoredHandler = None;
            SetWindowLongPtrW(hwnd, GWL_USERDATA, mem::transmute(handler));
            println!("Freeing window handler");
            result
        },
        _ => {
            default_handling()
        }
    }
}

pub fn make_game_window(width: c_int, height: c_int, handler: HandlerPtr) -> HWND {
    unsafe {
        let classname = WString::from_str("D2D1TestWindowClass");
        let windowname = WString::from_str("Test Direct2D Window");
        let h_instance = mem::transmute(GetModuleHandleW(mem::transmute(0usize)));

        RegisterClassExW(&WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as UINT,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(static_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: mem::size_of::<StoredHandler>() as c_int,
            hInstance: h_instance,
            hIcon: LoadIconW(h_instance, IDI_APPLICATION),
            hCursor: LoadCursorW(h_instance, IDC_ARROW),
            hbrBackground: GetSysColorBrush(COLOR_WINDOW),
            lpszMenuName: mem::transmute(0usize),
            lpszClassName: classname.lpcwstr(),
            hIconSm: mem::transmute(0usize),
        });

        let hwnd = CreateWindowExW(
            WS_EX_OVERLAPPEDWINDOW,
            classname.lpcwstr(),
            windowname.lpcwstr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            mem::transmute(0usize),
            mem::transmute(0usize),
            h_instance,
            mem::transmute(Some(Box::new(handler)))
        );

        ShowWindow(hwnd, SW_SHOW);

        hwnd
    }
}

pub fn process_message_loop(hwnd: HWND) {
    unsafe {
        let mut msg: MSG = mem::zeroed();
        loop {
            let ret = GetMessageW(&mut msg, hwnd, 0, 0);
            if ret == -1 || ret == 0 {
                return;
            }

            TranslateMessage(&mut msg);
            DispatchMessageW(&mut msg);
        }
    }
}
