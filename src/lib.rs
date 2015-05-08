//! See [augeas.h](https://github.com/hercules-team/augeas/blob/release-1.3.0/src/augeas.h) for details
extern crate libc;

use self::libc::{c_void, c_char, c_int, c_uint};

#[repr(C)]
#[derive(Copy,Clone)]
pub enum AugFlag {
    None = 0,
    SaveBackup = (1 << 0),
    SaveNewFile = (1 << 1),
    TypeCheck = (1 << 2),
    NoStdinc  = (1 << 3),
    SaveNoop = (1 << 4),
    NoLoad = (1 << 5),
    NoModlAutoload = (1 << 6),
    EnableSpan = (1 << 7),
    NoErrClose = (1 << 8),
    TraceModuleLoading = (1 << 9)
}

impl std::ops::BitOr for AugFlag {
    type Output = c_uint;
    fn bitor(self, other: AugFlag) -> c_uint {
        self as c_uint | other as c_uint
    }
} 

#[allow(non_camel_case_types)]
pub type augeas_t = *mut c_void;

#[link(name = "augeas")]
extern {
    pub fn aug_init(root: *const c_char, loadpath: *const c_char, flags: c_uint) -> augeas_t;
    pub fn aug_defvar(aug: augeas_t, name: *const c_char, expr: *const c_char) -> c_int;
    pub fn aug_get(aug: augeas_t, path: *const c_char, value: *mut *const c_char) -> c_int;
    pub fn aug_label(aug: augeas_t, path: *const c_char, label: *mut *const c_char) -> c_int;
    pub fn aug_close(aug: augeas_t);
    pub fn aug_save(aug: augeas_t) -> c_int;
    pub fn aug_set(aug: augeas_t, path: *const c_char, value: *const c_char) -> c_int;
    pub fn aug_match(aug: augeas_t, path: *const c_char, matches: *mut *mut *mut c_char ) -> c_int;
}
