use comptr::ComPtr;
use winapi::*;
use check_hresult;

use std::dynamic_lib::DynamicLibrary;
use std::path::Path;
use std::mem::{forget, transmute};

pub fn create_d2d1_factory() -> ComPtr<ID2D1Factory> {
    let d2d1_lib = DynamicLibrary::open(Some(Path::new("D2D1.dll"))).unwrap();
    let make_factory: unsafe extern "system" fn(DWORD, REFGUID, *const D2D1_FACTORY_OPTIONS, *mut *mut c_void) -> HRESULT;
    make_factory = unsafe { transmute(d2d1_lib.symbol::<c_void>("D2D1CreateFactory").unwrap()) };
    forget(d2d1_lib); // I have no idea why, but it crashes upon dropping the library

    let factory: *mut ID2D1Factory = unsafe {
        let mut void_factory: *mut c_void = transmute(0usize);
        let result = make_factory(
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

