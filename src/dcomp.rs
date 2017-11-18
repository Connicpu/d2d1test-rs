//! Safe-ish wrappers for DirectComposition and related interfaces.

use std::ops::{Deref, DerefMut};
use std::mem;
use std::ptr::{null, null_mut};
use winapi::Interface;
use winapi::shared::dxgi::IDXGIDevice;
use winapi::shared::dxgi1_2::DXGI_ALPHA_MODE_IGNORE;
use winapi::shared::dxgiformat::DXGI_FORMAT_B8G8R8A8_UNORM;
use winapi::shared::minwindef::{FALSE, TRUE};
use winapi::shared::windef::{HWND, POINT, RECT};
use winapi::shared::winerror::SUCCEEDED;
use winapi::um::d2d1::*;
use winapi::um::d2d1_1::*;
use winapi::um::d3d11::*;
use winapi::um::d3dcommon::D3D_DRIVER_TYPE_HARDWARE;
use winapi::um::unknwnbase::IUnknown;
use winapi::um::dcomp::*;
use winapi::um::winnt::HRESULT;

use direct2d::{self, RenderTarget};
use direct2d::error::D2D1Error;
use direct2d::math::Matrix3x2F;
use direct2d::render_target::RenderTargetBacking;

/// A ComPtr abstraction.
///
/// This is cut'n'paste from wio. Transition to the real thing when winapi 0.3 is
/// published.
pub struct ComPtr<T>(*mut T);
impl<T> ComPtr<T> {
    pub unsafe fn new(ptr: *mut T) -> ComPtr<T> { ComPtr(ptr) }

    fn as_unknown(&self) -> &mut IUnknown {
        unsafe { &mut *(self.0 as *mut IUnknown) }
    }

    fn into_raw(self) -> *mut T {
        let p = self.0;
        mem::forget(p);
        p
    }

    fn query_interface<U: Interface>(&self) -> Option<ComPtr<U>> {
        unsafe {
            let mut result: *mut U = null_mut();
            let hr = self.as_unknown().QueryInterface(&U::uuidof(), &mut result as *mut _ as *mut _);
            if SUCCEEDED(hr) {
                Some(ComPtr(result))
            } else {
                None
            }
        }
    }
}

unsafe fn wrap<T, U, F>(hr: HRESULT, ptr: *mut T, f: F) -> Result<U, HRESULT>
    where F: Fn(ComPtr<T>) -> U
{
    if SUCCEEDED(hr) {
        Ok(f(ComPtr::new(ptr)))
    } else {
        Err(hr)
    }
}

fn unit_err(hr: HRESULT) -> Result<(), HRESULT> {
    if SUCCEEDED(hr) { Ok(()) } else { Err(hr) }
}

impl<T> Deref for ComPtr<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

impl<T> DerefMut for ComPtr<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0 }
    }
}

impl<T> Drop for ComPtr<T> {
    fn drop(&mut self) {
        unsafe { self.as_unknown().Release(); }
    }
}

pub struct D3D11Device(ComPtr<ID3D11Device>);
pub struct D2D1Device(ComPtr<ID2D1Device>);
pub struct DCompositionDevice(ComPtr<IDCompositionDevice>);
pub struct DCompositionTarget(ComPtr<IDCompositionTarget>);
pub struct DCompositionVisual(ComPtr<IDCompositionVisual>);
pub struct DCompositionVirtualSurface(ComPtr<IDCompositionVirtualSurface>);

/// A trait for content which can be added to a visual.
pub trait Content {
    unsafe fn unknown_ptr(&mut self) -> *mut IUnknown;
}

impl D3D11Device {
    /// Creates a new device with basic defaults.
    pub fn new_simple() -> Result<D3D11Device, HRESULT> {
        unsafe {
            let mut d3d11_device: *mut ID3D11Device = null_mut();
            let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;  // could probably set single threaded
            let hr = D3D11CreateDevice(null_mut(), D3D_DRIVER_TYPE_HARDWARE, null_mut(), flags,
                null(), 0, D3D11_SDK_VERSION, &mut d3d11_device, null_mut(), null_mut());
            wrap(hr, d3d11_device, D3D11Device)
        }
    }

    pub fn create_d2d1_device(&mut self) -> Result<D2D1Device, HRESULT> {
        unsafe {
            let mut dxgi_device: ComPtr<IDXGIDevice> = self.0.query_interface().ok_or(0)?;
            let mut d2d1_device: *mut ID2D1Device = null_mut();
            let hr = D2D1CreateDevice(dxgi_device.deref_mut(), null(), &mut d2d1_device);
            wrap(hr, d2d1_device, D2D1Device)
        }
    }
}

impl D2D1Device {
    pub fn create_composition_device(&mut self) -> Result<DCompositionDevice, HRESULT> {
        unsafe {
            let mut dcomp_device: *mut IDCompositionDevice = null_mut();
            let hr = DCompositionCreateDevice2(self.0.as_unknown(), &IDCompositionDevice::uuidof(),
                &mut dcomp_device as *mut _ as *mut _);
            wrap(hr, dcomp_device, DCompositionDevice)
        }
    }
}

impl DCompositionDevice {
    pub unsafe fn create_target_for_hwnd(&mut self, hwnd: HWND, topmost: bool)
        -> Result<DCompositionTarget, HRESULT>
    {
        let mut dcomp_target: *mut IDCompositionTarget = null_mut();
        let hr = self.0.CreateTargetForHwnd(hwnd, if topmost { TRUE } else { FALSE },
            &mut dcomp_target);
        wrap(hr, dcomp_target, DCompositionTarget)
    }

    pub fn create_visual(&mut self) -> Result<DCompositionVisual, HRESULT> {
        unsafe {
            let mut visual: *mut IDCompositionVisual = null_mut();
            let hr = self.0.CreateVisual(&mut visual);
            wrap(hr, visual, DCompositionVisual)
        }
    }

    /// Creates an RGB surface. Probably should allow more options (including alpha).
    pub fn create_virtual_surface(&mut self, height: u32, width: u32)
        -> Result<DCompositionVirtualSurface, HRESULT>
    {
        unsafe {
            let mut surface: *mut IDCompositionVirtualSurface = null_mut();
            let hr = self.0.CreateVirtualSurface(width, height, DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_ALPHA_MODE_IGNORE, &mut surface);
            wrap(hr, surface, DCompositionVirtualSurface)
        }
    }

    pub fn commit(&mut self) -> Result<(), HRESULT> {
        unsafe {
            unit_err(self.0.Commit())
        }
    }
}

impl DCompositionTarget {
    pub fn set_root(&mut self, visual: &mut DCompositionVisual) -> Result<(), HRESULT> {
        unsafe {
            unit_err(self.0.SetRoot(visual.0.deref_mut()))
        }
    }
}

impl DCompositionVisual {
    pub fn set_content<T: Content>(&mut self, content: &mut T) -> Result<(), HRESULT> {
        unsafe {
            unit_err(self.0.SetContent(content.unknown_ptr()))
        }
    }
}

struct DcBacking(*mut ID2D1DeviceContext);
unsafe impl RenderTargetBacking for DcBacking {
    fn create_target(self, _factory: &mut ID2D1Factory1) -> Result<*mut ID2D1RenderTarget, HRESULT> {
        Ok(self.0 as *mut ID2D1RenderTarget)
    }
}

// TODO: support common methods with DCompositionSurface, probably should be trait
impl DCompositionVirtualSurface {
    // could try to expose more DeviceContext capability
    pub fn begin_draw(&mut self, d2d_factory: &direct2d::Factory, rect: Option<RECT>)
        -> Result<RenderTarget, HRESULT>
    {
        unsafe {
            let mut dc: *mut ID2D1DeviceContext = null_mut();
            let rect_ptr = match rect {
                None => null(),
                Some(r) => &r,
            };
            let mut offset: POINT = mem::uninitialized();
            let hr = self.0.BeginDraw(rect_ptr, &ID2D1DeviceContext::uuidof(),
                &mut dc as *mut _ as *mut _, &mut offset);
            if !SUCCEEDED(hr) {
                return Err(hr);
            }
            let backing = DcBacking(dc);
            let mut rt = d2d_factory.create_render_target(backing).map_err(|e|
                match e {
                    D2D1Error::ComError(hr) => hr,
                    _ => 0,
                })?;
            // TODO: either move dpi scaling somewhere else or figure out how to
            // set it correctly here.
            rt.set_transform(&Matrix3x2F::new([[2.0, 0.0], [0.0, 2.0],
                [offset.x as f32, offset.y as f32]]));
            Ok(rt)
        }
    }

    pub fn end_draw(&mut self) -> Result<(), HRESULT> {
        unsafe {
            unit_err(self.0.EndDraw())
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), HRESULT> {
        unsafe {
            unit_err(self.0.Resize(width, height))
        }
    }
}

impl Content for DCompositionVirtualSurface {
    unsafe fn unknown_ptr(&mut self) -> *mut IUnknown {
        self.0.as_unknown()
    }
}
