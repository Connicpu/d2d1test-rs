use winapi::*;

pub struct WString {
    value: Vec<u16>
}

impl WString {
    pub fn from_str(s: &str) -> WString {
        let terminator = [0u16; 1];
        WString {
            value: s.utf16_units().chain(terminator.iter().cloned()).collect()
        }
    }

    pub fn lpcwstr(&self) -> &WCHAR {
        &self.value[0]
    }
}
