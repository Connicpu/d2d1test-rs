#![windows_subsystem = "windows"]

extern crate winapi;
extern crate direct2d;
extern crate directwrite;

extern crate libloading;

mod dcomp;
//mod hwnd_rt;
mod util;
mod window;

use std::cell::RefCell;
use std::convert::From;
use std::mem;
use std::ptr::{null, null_mut};
use std::rc::Rc;
use std::time::SystemTime;

// probably should be more selective about these imports...
use winapi::shared::minwindef::*;
use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

use direct2d::{RenderTarget, brush};
use direct2d::math::*;
use direct2d::render_target::DrawTextOption;
use directwrite::text_format::{self, TextFormat};

use util::{Error, ToWide};
use window::{create_window, WndProc};

struct Resources {
    fg: brush::SolidColor,
    bg: brush::SolidColor,
    text_format: TextFormat,
}

// Things that are passed in at window creation time
pub struct Stuff {
    dcomp_device: dcomp::DCompositionDevice,
    surface: dcomp::DCompositionVirtualSurface,
    visual: dcomp::DCompositionVisual,    
}

struct MainWinState {
    d2d_factory: direct2d::Factory,
    dwrite_factory: directwrite::Factory,
    resources: Option<Resources>,
    stuff: Option<Stuff>,
}

fn create_resources(dwrite_factory: &directwrite::Factory, rt: &RenderTarget)
    -> Resources
{
    let text_format_params = text_format::ParamBuilder::new()
        .size(15.0)
        .family("Consolas")
        .build().unwrap();
    let text_format = dwrite_factory.create(text_format_params).unwrap();
    Resources {
        fg: rt.create_solid_color_brush(0xf0f0ea, &BrushProperties::default()).unwrap(),
        bg: rt.create_solid_color_brush(0x272822, &BrushProperties::default()).unwrap(),
        text_format: text_format,
    }
}

impl MainWinState {
    fn new() -> MainWinState {
        MainWinState {
            d2d_factory: direct2d::Factory::new().unwrap(),
            dwrite_factory: directwrite::Factory::new().unwrap(),
            resources: None,
            stuff: None,
        }
    }

    fn set(&mut self, stuff: Stuff) {
        self.stuff = Some(stuff);
    }

    fn render_dcomp(&mut self, width: u32, height: u32) {
        let stuff = self.stuff.as_mut().unwrap();

        let mut rt = stuff.surface.begin_draw(&self.d2d_factory, None).unwrap();
        if self.resources.is_none() {
            self.resources = Some(create_resources(&self.dwrite_factory, &rt));
        }

        let resources = &self.resources.as_ref().unwrap();

        let rect = RectF::from((0.0, 0.0, width as f32, height as f32));
        rt.fill_rectangle(&rect, &resources.bg);
        rt.draw_line(&Point2F::from((0.0, 0.0)), &Point2F::from((width as f32, height as f32)),
            &resources.fg, 1.0, None);

        let msg = "Hello DWrite! This is a somewhat longer string of text intended to provoke slightly longer draw times.";
        let dy = 15.0;
        let x0 = 10.0;
        let y0 = 10.0;
        for i in 0..60 {
            let y = y0 + (i as f32) * dy;
            rt.draw_text(
                msg,
                &resources.text_format,
                &RectF::from((x0, y, x0 + 900.0, y + 80.0)),
                &resources.fg,
                &[DrawTextOption::EnableColorFont]
            );
        }

        stuff.surface.end_draw().unwrap();
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
            WM_WINDOWPOSCHANGING =>  unsafe {
                let windowpos = &*(lparam as *const WINDOWPOS);
                if windowpos.cx != 0 && windowpos.cy != 0 {
                    let mut rect = mem::zeroed();
                    GetWindowRect(hwnd, &mut rect);
                    let mut client_rect = mem::zeroed();
                    GetClientRect(hwnd, &mut client_rect);
                    let width_pad = rect.right - rect.left - client_rect.right;
                    let height_pad = rect.bottom - rect.top - client_rect.bottom;
                    let width = (windowpos.cx - width_pad) as u32;
                    let height = (windowpos.cy - height_pad) as u32;
                    let mut state = self.state.borrow_mut();
                    state.stuff.as_mut().unwrap().surface
                        .resize(width as u32, height as u32).unwrap();
                    state.render_dcomp(width as u32, height as u32);
                    state.stuff.as_mut().unwrap().dcomp_device.commit().unwrap();
                }
                Some(0)
            },
            WM_PAINT => unsafe {
                //println!("WM_PAINT");

                ValidateRect(hwnd, null());
                Some(0)
            },
            WM_SIZE => unsafe {
                println!("size {} x {} {:?}", LOWORD(lparam as u32), HIWORD(lparam as u32),
                    self.clock.elapsed());
                Some(1)
            },
            WM_MOUSEMOVE => {
                let x = LOWORD(lparam as u32);
                let y = HIWORD(lparam as u32);
                let mut state = self.state.borrow_mut();
                let stuff = state.stuff.as_mut().unwrap();
                stuff.visual.set_pos(&stuff.dcomp_device, x as f32, y as f32);
                stuff.dcomp_device.commit();
                Some(0)
            }
            WM_ERASEBKGND => Some(1),
            _ => None
        }
    }

    fn set(&self, stuff: Stuff) {
        self.state.borrow_mut().set(stuff);
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

fn create_main() -> Result<HWND, Error> {
    unsafe {
        let class_name = "d1d1test-rs".to_wide();
        let icon = LoadIconW(0 as HINSTANCE, IDI_APPLICATION);
        let cursor = LoadCursorW(0 as HINSTANCE, IDC_IBEAM);
        let brush = CreateSolidBrush(0x00ff00);
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
        let hwnd = create_window(/* WS_EX_OVERLAPPEDWINDOW | */ WS_EX_NOREDIRECTIONBITMAP, class_name.as_ptr(),
            class_name.as_ptr(), WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT, CW_USEDEFAULT, width, height, 0 as HWND, 0 as HMENU, 0 as HINSTANCE,
            main_win.clone());
        if hwnd.is_null() {
            return Err(Error::Null);
        }

        // Note: this API is Windows 10 only, we'll need to conditionally call it
        //println!("window dpi = {}", GetDpiForWindow(hwnd));

        let mut d3d11_device = dcomp::D3D11Device::new_simple()?;
        let mut d2d1_device = d3d11_device.create_d2d1_device()?;
        let mut dcomp_device = d2d1_device.create_composition_device()?;
        let mut dcomp_target = dcomp_device.create_target_for_hwnd(hwnd, true)?;

        let mut visual = dcomp_device.create_visual()?;
        let mut surface = dcomp_device.create_virtual_surface(width as u32, height as u32)?;

        visual.set_content(&mut surface)?;
        dcomp_target.set_root(&mut visual)?;

        main_win.set(Stuff { dcomp_device, surface, visual });

        // TODO: maybe should store this in window state instead of leaking.
        mem::forget(dcomp_target);

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
