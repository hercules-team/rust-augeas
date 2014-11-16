extern crate libc;

use self::libc::{c_void, c_char, c_int, c_uint};

//use libc::size_t;

#[allow(non_camel_case_types)]
pub type augeas = *mut c_void;

#[link(name = "augeas")]
extern {
  pub fn aug_init(root: *const c_char, loadpath: *const c_char, flags: c_uint) -> augeas;
  pub fn aug_get(aug: augeas, path: *const c_char, value: *mut *const c_char) -> c_int;
  pub fn aug_label(aug: augeas, path: *const c_char, label: *mut *const c_char) -> c_int;
  pub fn aug_close(aug: augeas);
  pub fn aug_match(aug: augeas, path: *const c_char, matches: *mut *mut *mut c_char ) -> c_int;
}