#![feature(dynamic_lib)]
#![feature(str_utf16)]

extern crate winapi;
extern crate d2d1 as d2d1_sys;
extern crate dwrite as dwrite_sys;
extern crate user32;
extern crate kernel32;

pub mod comptr;
pub mod load;
pub mod window;
pub mod wstr;

use winapi::*;
use std::mem;
use comptr::ComPtr;
use wstr::WString;

fn check_hresult(result: HRESULT) {
    if result < 0 {
        panic!("HRESULT Failure");
    }
}

struct GameInstance {
    pub size: D2D1_SIZE_U,
    pub dpi_scale: f32,
    pub factory: ComPtr<ID2D1Factory>,
    pub dwrite: ComPtr<IDWriteFactory>,

    pub render_target: Option<ComPtr<ID2D1HwndRenderTarget>>,
    pub text_format: Option<ComPtr<IDWriteTextFormat>>,
}

impl GameInstance {
    fn new(factory: ComPtr<ID2D1Factory>, dwrite: ComPtr<IDWriteFactory>, size: D2D1_SIZE_U) -> GameInstance {
        GameInstance {
            size: size,
            dpi_scale: 1.0,
            factory: factory,
            dwrite: dwrite,
            render_target: None,
            text_format: None,
        }
    }

    unsafe fn initialize(&mut self, hwnd: HWND) {
        let mut dpi_x = 0.0;
        let mut dpi_y = 0.0;
        self.factory.GetDesktopDpi(&mut dpi_x, &mut dpi_y);

        self.dpi_scale = dpi_x / 96.0;

        let mut render_target = mem::zeroed();
        check_hresult(self.factory.CreateHwndRenderTarget(
            &D2D1_RENDER_TARGET_PROPERTIES {
                _type: D2D1_RENDER_TARGET_TYPE_HARDWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM as u32,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: dpi_x,
                dpiY: dpi_y,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_10,
            },
            &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: hwnd,
                pixelSize: self.size,
                presentOptions: D2D1_PRESENT_OPTIONS_IMMEDIATELY,
            },
            &mut render_target
        ));

        self.render_target = Some(ComPtr::wrap_existing(render_target));
        println!("We have an ID2D1RenderTarget: {:?}", self.render_target);

        let mut text_format = mem::zeroed();
        check_hresult(self.dwrite.CreateTextFormat(
            WString::from_str("Comic Sans MS").lpcwstr(),
            mem::zeroed(), // Font collection- use null for default
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            26.0, // Font size
            WString::from_str("en-US").lpcwstr(),
            &mut text_format
        ));

        let format: &mut IDWriteTextFormat = mem::transmute(text_format);
        check_hresult(format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER));
        check_hresult(format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER));

        self.text_format = Some(ComPtr::wrap_existing(text_format));
        println!("We have an IDWriteTextFormat: {:?}", self.text_format);
    }

    unsafe fn paint(&mut self) {
        let rt = self.render_target.as_mut().unwrap();
        rt.BeginDraw();
        rt.Clear(&D2D1_COLOR_F { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });

        let render_size = *rt.GetSize(&mut mem::uninitialized());

        let mut brush = ComPtr::<ID2D1SolidColorBrush>::uninit();
        check_hresult(rt.CreateSolidColorBrush(
            &D2D1_COLOR_F { r: 0.0, g: 0.0, b: 1.0, a: 1.0 },
            &D2D1_BRUSH_PROPERTIES {
                opacity: 1.0,
                transform: D2D1_MATRIX_3X2_F {
                    matrix: [
                        [1.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0],
                    ]
                }
            },
            brush.addr()
        ));

        let mut text_brush = ComPtr::<ID2D1SolidColorBrush>::uninit();
        check_hresult(rt.CreateSolidColorBrush(
            &D2D1_COLOR_F { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
            &D2D1_BRUSH_PROPERTIES {
                opacity: 1.0,
                transform: D2D1_MATRIX_3X2_F {
                    matrix: [
                        [1.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0],
                    ]
                }
            },
            text_brush.addr()
        ));

        rt.FillRoundedRectangle(
            &D2D1_ROUNDED_RECT {
                rect: D2D1_RECT_F {
                    left: 20.0,
                    top: 20.0,
                    right: render_size.width - 20.0,
                    bottom: render_size.height - 20.0,
                },
                radiusX: 100.0,
                radiusY: 100.0,
            },
            brush.ptr_mut() as *mut winapi::d2d1::ID2D1Brush
        );

        let message = WString::from_str("This is a very nice font! 🖕🏻\n\
Try displaying some 🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰🐇🐰");

        rt.DrawText(
            message.lpcwstr(),
            message.len(),
            self.text_format.as_mut().unwrap().ptr_mut(),
            &D2D1_RECT_F {
                left: 20.0,
                top: 20.0,
                right: render_size.width - 20.0,
                bottom: render_size.height - 20.0,
            },
            text_brush.ptr_mut() as *mut winapi::d2d1::ID2D1Brush,
            D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
            DWRITE_MEASURING_MODE_NATURAL
        );

        check_hresult(rt.EndDraw(&mut 0, &mut 0));
    }
}

impl window::WindowProcHandler for GameInstance {
    fn wnd_proc(&mut self, hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match msg {
                WM_CREATE => {
                    self.initialize(hwnd);
                    0
                },
                WM_PAINT => {
                    let mut pstruct: PAINTSTRUCT = mem::zeroed();
                    user32::BeginPaint(hwnd, &mut pstruct);
                    self.paint();
                    user32::EndPaint(hwnd, &pstruct);
                    0
                },
                WM_SIZE => {
                    let width = LOWORD(lparam as u32) as u32;
                    let height = HIWORD(lparam as u32) as u32;

                    self.size = D2D1_SIZE_U { width: width, height: height };
                    let rt = self.render_target.as_mut().unwrap();
                    rt.Resize(&self.size);

                    user32::DefWindowProcW(hwnd, msg, wparam, lparam)
                },
                _ => user32::DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }
}

fn main() {
    load::dpi_aware();

    let window_size = D2D1_SIZE_U {
        width: 1024,
        height: 1024,
    };

    let factory = load::create_d2d1_factory();
    let dwrite = load::create_dwrite_factory();

    println!("We have an ID2D1Factory: {:?}", factory);

    let hwnd = window::make_game_window(
        window_size.width as c_int,
        window_size.height as c_int,
        Box::new(GameInstance::new(factory, dwrite, window_size))
    );
    println!("We have a window: {:?}", hwnd);

    window::process_message_loop(hwnd);
}
