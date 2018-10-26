use std::ffi::CStr;
use libc::c_char;

pub fn ptr_to_string(s: *const c_char) -> Option<String> {
    if s.is_null() {
        None
    } else {
        let s = unsafe { CStr::from_ptr(s) };
        let s = String::from_utf8_lossy(s.to_bytes()).into_owned();
        Some(s)
    }
}
