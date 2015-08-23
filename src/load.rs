use comptr::ComPtr;
use winapi::*;
use check_hresult;

use std::dynamic_lib::DynamicLibrary;
use std::path::Path;
use std::mem::{forget, transmute};

pub fn create_d2d1_factory() -> ComPtr<ID2D1Factory> {
    let factory: *mut ID2D1Factory = unsafe {
        let mut void_factory: *mut c_void = transmute(0usize);
        let result = ::d2d1_sys::D2D1CreateFactory(
            D2D1_FACTORY_TYPE_MULTI_THREADED,
            &UuidOfID2D1Factory,
            &D2D1_FACTORY_OPTIONS {
                debugLevel: D2D1_DEBUG_LEVEL_WARNING
            },
            &mut void_factory
        );
        check_hresult(result);
        transmute(void_factory)
    };

    ComPtr::wrap_existing(factory)
}

pub fn create_dwrite_factory() -> ComPtr<IDWriteFactory> {
    let mut factory: ComPtr<IDWriteFactory> = ComPtr::uninit();
    unsafe {
        check_hresult(::dwrite_sys::DWriteCreateFactory(
            DWRITE_FACTORY_TYPE_SHARED,
            &UuidOfIDWriteFactory,
            factory.addr() as *mut *mut IDWriteFactory as *mut *mut IUnknown
        ));
    }
    factory
}

pub fn dpi_aware() {
    let shcore_lib = DynamicLibrary::open(Some(Path::new("ShCore.dll"))).unwrap();
    let set_aware: unsafe extern "system" fn(awareness: PROCESS_DPI_AWARENESS) -> HRESULT;
    set_aware = unsafe { transmute(shcore_lib.symbol::<c_void>("SetProcessDpiAwareness").unwrap()) };
    forget(shcore_lib);

    unsafe { set_aware(PROCESS_DPI_AWARENESS::Process_Per_Monitor_DPI_Aware); }
}

