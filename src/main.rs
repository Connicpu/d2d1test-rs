#![windows_subsystem = "windows"]

extern crate winapi;
extern crate direct2d;
extern crate directwrite;

extern crate libloading;

mod hwnd_rt;
mod util;
mod window;

use std::cell::RefCell;
use std::mem;
use std::ptr::{null, null_mut};
use std::rc::Rc;
use std::time::SystemTime;

// probably should be more selective about these imports...
use winapi::Interface;
use winapi::ctypes::c_void;
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgi1_3::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::dxgitype::*;
use winapi::shared::minwindef::*;
use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::*;
use winapi::shared::winerror::*;
use winapi::um::d2d1::*;
use winapi::um::d2d1_1::*;
use winapi::um::d3d11::*;
use winapi::um::d3dcommon::*;
use winapi::um::dcommon::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;
use winapi::um::unknwnbase::*;

use direct2d::{RenderTarget, brush};
use direct2d::math::*;
use direct2d::render_target::{DrawTextOption, RenderTargetBacking};
use directwrite::text_format::{self, TextFormat};

use hwnd_rt::HwndRtParams;
use util::{Error, ToWide};
use window::{create_window, WndProc};

struct Resources {
    fg: brush::SolidColor,
    bg: brush::SolidColor,
    text_format: TextFormat,
}

struct MainWinState {
    d2d_factory: direct2d::Factory,
    dwrite_factory: directwrite::Factory,
    render_target: Option<RenderTarget>,
    resources: Option<Resources>,
    swap_chain: *mut IDXGISwapChain1,
}

impl MainWinState {
    fn new() -> MainWinState {
        MainWinState {
            d2d_factory: direct2d::Factory::new().unwrap(),
            dwrite_factory: directwrite::Factory::new().unwrap(),
            render_target: None,
            resources: None,
            swap_chain: null_mut(),
        }
    }

    fn create_resources(&mut self) -> Resources {
        let rt = self.render_target.as_mut().unwrap();
        let text_format_params = text_format::ParamBuilder::new()
            .size(15.0)
            .family("Consolas")
            .build().unwrap();
        let text_format = self.dwrite_factory.create(text_format_params).unwrap();
        Resources {
            fg: rt.create_solid_color_brush(0xf0f0ea, &BrushProperties::default()).unwrap(),
            bg: rt.create_solid_color_brush(0x272822, &BrushProperties::default()).unwrap(),
            text_format: text_format,
        }
    }

    fn rebuild_render_target(&mut self) {
        unsafe {
            let mut buffer: *mut IDXGISurface = null_mut();
            let res = (*self.swap_chain).GetBuffer(0, &IDXGISurface::uuidof(),
                &mut buffer as *mut _ as *mut *mut c_void);
            //println!("buffer res = 0x{:x}, pointer = {:?}", res, buffer);
            if SUCCEEDED(res) {
                let backing = DxgiBacking(buffer);
                self.render_target = self.d2d_factory.create_render_target(backing).ok();
                (*buffer).Release();
            }
        }
    }

    fn set_swap_chain(&mut self, swap_chain: *mut IDXGISwapChain1) {
        self.swap_chain = swap_chain;
        self.rebuild_render_target();
    }

    fn render(&mut self) {
        let res = {
            if self.resources.is_none() {
                self.resources = Some(self.create_resources());
            }
            let resources = &self.resources.as_ref().unwrap();
            let rt = self.render_target.as_mut().unwrap();
            rt.begin_draw();
            let size = rt.get_size();
            let rect = RectF::from((0.0, 0.0, size.width, size.height));
            rt.fill_rectangle(&rect, &resources.bg);
            rt.draw_line(&Point2F::from((0.0, 0.0)), &Point2F::from((size.width, size.height)),
                &resources.fg, 1.0, None);
            let msg = "Hello DWrite! This is a somewhat longer string of text intended to provoke slightly longer draw times.";
            for i in 0..30 {
                rt.draw_text(
                    msg,
                    &resources.text_format,
                    &RectF::from((10.0, 10.0 + (i as f32) * 15.0, 900.0, 90.0 + (i as f32) * 15.0)),
                    &resources.fg,
                    &[DrawTextOption::EnableColorFont]
                );
            }
            rt.end_draw()
        };
        if res.is_err() {
            self.render_target = None;
        }
    }
}

struct MainWin {
    state: RefCell<MainWinState>,
    clock: SystemTime,
}

impl MainWin {
    fn new(state: MainWinState) -> MainWin {
        MainWin {
            state: RefCell::new(state),
            clock: SystemTime::now(),
        }
    }
}

impl WndProc for MainWin {
    fn window_proc(&self, hwnd: HWND, msg: UINT, _wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        //println!("{:x} {:x} {:x}", msg, _wparam, lparam);
        match msg {
            WM_DESTROY => unsafe {
                PostQuitMessage(0);
                None
            },
            WM_PAINT => unsafe {
                //println!("WM_PAINT");
                let mut state = self.state.borrow_mut();
                if state.render_target.is_none() {
                    let mut rect: RECT = mem::uninitialized();
                    GetClientRect(hwnd, &mut rect);
                    //println!("rect={:?}", rect);
                    let width = (rect.right - rect.left) as u32;
                    let height = (rect.bottom - rect.top) as u32;
                    let params = HwndRtParams { hwnd: hwnd, width: width, height: height };
                    state.render_target = state.d2d_factory.create_render_target(params).ok();
                }
                state.render();
                (*state.swap_chain).Present(0, 0);
                ValidateRect(hwnd, null_mut());
                Some(0)
            },
            WM_SIZE => unsafe {
                let mut state = self.state.borrow_mut();
                state.render_target = None;
                let res = (*state.swap_chain).ResizeBuffers(0, 0, 0, DXGI_FORMAT_UNKNOWN, 0);
                if SUCCEEDED(res) {
                    state.rebuild_render_target();
                } else {
                    println!("ResizeBuffers failed: 0x{:x}", res);
                }
                //println!("size {} x {} {:?}", LOWORD(lparam as u32), HIWORD(lparam as u32),
                //    self.clock.elapsed());
                /*
                state.render_target.as_mut().and_then(|rt|
                    rt.hwnd_rt().map(|hrt|
                        hrt.Resize(&D2D1_SIZE_U {
                            width: LOWORD(lparam as u32) as u32,
                            height: HIWORD(lparam as u32) as u32,
                        })
                    )
                );
                */
                Some(1)
            },
            _ => None
        }
    }

    fn set_swap_chain(&self, swap_chain: *mut IDXGISwapChain1) {
        self.state.borrow_mut().set_swap_chain(swap_chain);
    }
}

fn from_wide(wstr: &[u16]) -> String {
    let mut result = String::new();
    for &c in wstr {
        if c == 0 { break; }
        if let Some(c) = ::std::char::from_u32(c as u32) {
            result.push(c);
        }
    }
    result
}

struct DxgiBacking(*mut IDXGISurface);

unsafe impl RenderTargetBacking for DxgiBacking {
    fn create_target(self, factory: &mut ID2D1Factory1) -> Result<*mut ID2D1RenderTarget, HRESULT> {
        unsafe {
            /*
            let mut dxgi_device: *mut IDXGIDevice = null_mut();
            (*self.1).QueryInterface(&IDXGIDevice::uuidof(), &mut dxgi_device as *mut _ as *mut _);
            println!("dxgi device ptr = {:?}", dxgi_device);
            //let mut d2d_device: &mut ID2D1Device = null_mut();
            let mut device: *mut ID2D1Device = null_mut();
            let res = factory.CreateDevice(dxgi_device, &mut device as *mut _);
            println!("device res=0x{:x}, ptr = {:?}", res, device);
            */
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                _type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: 192.0, // TODO: get this from window etc.
                dpiY: 192.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };

            let mut render_target: *mut ID2D1RenderTarget = null_mut();
            let res = factory.CreateDxgiSurfaceRenderTarget(self.0, &props, &mut render_target);
            //println!("surface render target res=0x{:x}, ptr = {:?}", res, render_target);
            if SUCCEEDED(res) {
                Ok(render_target)
            } else {
                Err(res)
            }

            /*
            let mut device_context: *mut ID2D1DeviceContext = null_mut();
            let res = (*device).CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE, &mut device_context);
            println!("device context res=0x{:x}, ptr = {:?}", res, device_context);
            let mut bitmap: *mut ID2D1Bitmap1 = null_mut();
            let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_UNKNOWN,
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: 0.0,
                dpiY: 0.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: null(),
            };
            let res = (*device_context).CreateBitmapFromDxgiSurface(self.0, &bitmap_props, &mut bitmap);
            println!("bitmap res = 0x{:x}, ptr = {:?}", res, bitmap);

            let buf = [0xffu8; 256];
            let rect = D2D1_RECT_U { left: 0, top: 0, right: 8, bottom: 8};
            (*bitmap).CopyFromMemory(&rect, &buf as *const _ as *const c_void, 32);
            (*bitmap).Release();
            Err(0)
            */
        }
    }
}

fn create_main() -> Result<HWND, Error> {
    unsafe {
        let class_name = "d1d1test-rs".to_wide();
        let icon = LoadIconW(0 as HINSTANCE, IDI_APPLICATION);
        let cursor = LoadCursorW(0 as HINSTANCE, IDC_IBEAM);
        let brush = CreateSolidBrush(0xffffff);
        let wnd = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(window::win_proc_dispatch),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: 0 as HINSTANCE,
            hIcon: icon,
            hCursor: cursor,
            hbrBackground: brush,
            lpszMenuName: 0 as LPCWSTR,
            lpszClassName: class_name.as_ptr(),
        };
        let class_atom = RegisterClassW(&wnd);
        if class_atom == 0 {
            return Err(Error::Null);
        }
        let main_win: Rc<Box<WndProc>> = Rc::new(Box::new(MainWin::new(MainWinState::new())));
        let width = 500;  // TODO: scale by dpi
        let height = 400;
        let hwnd = create_window(WS_EX_OVERLAPPEDWINDOW, class_name.as_ptr(),
            class_name.as_ptr(), WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT, CW_USEDEFAULT, width, height, 0 as HWND, 0 as HMENU, 0 as HINSTANCE,
            main_win.clone());
        if hwnd.is_null() {
            return Err(Error::Null);
        }

        // Note: this API is Windows 10 only, we'll need to conditionally call it
        //println!("window dpi = {}", GetDpiForWindow(hwnd));

        let mut d3d11_device: *mut ID3D11Device = null_mut();
        let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;  // could probably set single threaded
        D3D11CreateDevice(null_mut(), D3D_DRIVER_TYPE_HARDWARE, null_mut(), flags,
            null(), 0, D3D11_SDK_VERSION, &mut d3d11_device, null_mut(), null_mut());
        println!("d3d11 device pointer = {:?}", d3d11_device);
        let mut factory: *mut IDXGIFactory2 = null_mut();
        let hres = CreateDXGIFactory2(0, &IID_IDXGIFactory2,
            &mut factory as *mut *mut IDXGIFactory2 as *mut *mut c_void);
        println!("dxgi factory pointer = {:?}", factory);
        for i in 0..4 {
            let mut adapter: *mut IDXGIAdapter = null_mut();
            (*factory).EnumAdapters(i, &mut adapter);
            println!("adapter {} = {:?}", i, adapter);
            if adapter != null_mut() {
                let mut desc: DXGI_ADAPTER_DESC = mem::uninitialized();
                (*adapter).GetDesc(&mut desc);
                println!("desc = {}", from_wide(&desc.Description));
            }
        }
        let mut swap_chain: *mut IDXGISwapChain1 = null_mut();
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: 0,
            Height: 0,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: FALSE,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0},
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_NONE,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_UNSPECIFIED,
            Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH,
        };
        let res = (*factory).CreateSwapChainForHwnd(d3d11_device as *mut IUnknown, hwnd, &desc,
            null(), null_mut(), &mut swap_chain);
        println!("swap chain res = 0x{:x}, pointer = {:?}", res, swap_chain);

        // for diagnostics; for real, we'd want to minimize the visual prominence
        let color = DXGI_RGBA { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
        (*swap_chain).SetBackgroundColor(&color);
        /*
        let mut rt_view: *mut ID3D11RenderTargetView = null_mut();
        (*d3d11_device).CreateRenderTargetView(buffer as *mut ID3D11Resource, null(),
            &mut rt_view);
        println!("render target view pointer = {:?}", rt_view);
        */

        main_win.set_swap_chain(swap_chain);

        Ok(hwnd)
    }
}

fn main() {
    unsafe {
        util::dpi_aware();
        let hwnd = create_main().unwrap();
        ShowWindow(hwnd, SW_SHOWNORMAL);
        UpdateWindow(hwnd);
        let mut msg = mem::uninitialized();
        loop {
            let bres = GetMessageW(&mut msg, null_mut(), 0, 0);
            if bres <= 0 {
                break;
            }
            TranslateMessage(&mut msg);
            DispatchMessageW(&mut msg);
        }
    }
}
