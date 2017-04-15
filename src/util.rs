use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use winapi::{HRESULT, PROCESS_DPI_AWARENESS, Process_System_DPI_Aware};

use libloading::{Library, Symbol};

#[derive(Debug)]
pub enum Error {
    Null,
    Hr(HRESULT),
}

impl From<HRESULT> for Error {
    fn from(hr: HRESULT) -> Error {
        Error::Hr(hr)
    }
}

pub trait ToWide {
    fn to_wide_sized(&self) -> Vec<u16>;
    fn to_wide(&self) -> Vec<u16>;
}

impl<T> ToWide for T where T: AsRef<OsStr> {
    fn to_wide_sized(&self) -> Vec<u16> {
        self.as_ref().encode_wide().collect()
    }
    fn to_wide(&self) -> Vec<u16> {
        self.as_ref().encode_wide().chain(Some(0)).collect()
    }
}

pub fn dpi_aware() {
    let shcore_lib = Library::new("ShCore.dll").unwrap();
    let set_aware: Symbol<unsafe extern "system" fn(awareness: PROCESS_DPI_AWARENESS) -> HRESULT>;
    unsafe {
        set_aware = shcore_lib.get(b"SetProcessDpiAwareness").unwrap();

        // We choose System DPI awareness here because per process has many pitfalls. The
        // best way to do this would be to set per process v2 dpi awareness in the manifest,
        // falling back to system dpi awareness for earlier versions.
        set_aware(Process_System_DPI_Aware);
    }
}
