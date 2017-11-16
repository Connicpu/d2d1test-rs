
use winapi::ctypes::c_int;
use winapi::shared::basetsd::LONG_PTR;
use winapi::shared::dxgi1_2::IDXGISwapChain1;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::dcomp::*;
use winapi::um::winuser::*;
use winapi::um::winnt::LPCWSTR;

use std::mem;
use std::rc::Rc;

pub trait WndProc {
    fn window_proc(&self, hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT>;

    // This clearly doesn't belong here but is expedient while we experiment with state wiring.
    fn set(&self, dcomp_device: *mut IDCompositionDevice, surface: *mut IDCompositionVirtualSurface,
        visual: *mut IDCompositionVisual);
}

pub unsafe extern "system" fn win_proc_dispatch(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM)
    -> LRESULT
{
    if msg == WM_CREATE {
        let create_struct = &*(lparam as *const CREATESTRUCTW);
        let wndproc_ptr = create_struct.lpCreateParams;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, wndproc_ptr as LONG_PTR);
    }
    let wndproc_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Box<WndProc>;
    let result = {
        if wndproc_ptr.is_null() {
            None
        } else {
            let wndproc = &*(wndproc_ptr as *const Box<WndProc>);
            wndproc.window_proc(hwnd, msg, wparam, lparam)
        }
    };
    if msg == WM_NCDESTROY {
        if !wndproc_ptr.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            mem::drop(Rc::from_raw(wndproc_ptr));
        }
    }
    match result {
        Some(lresult) => lresult,
        None => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Create a window (same parameters as CreateWindowExW) with associated WndProc.
#[allow(non_snake_case)]
pub unsafe fn create_window(
        dwExStyle: DWORD, lpClassName: LPCWSTR, lpWindowName: LPCWSTR, dwStyle: DWORD, x: c_int,
        y: c_int, nWidth: c_int, nHeight: c_int, hWndParent: HWND, hMenu: HMENU,
        hInstance: HINSTANCE, wndproc: Rc<Box<WndProc>>) -> HWND
{
    let hwnd = CreateWindowExW(dwExStyle, lpClassName, lpWindowName, dwStyle, x, y,
        nWidth, nHeight, hWndParent, hMenu, hInstance, Rc::into_raw(wndproc) as LPVOID);
    hwnd
}
