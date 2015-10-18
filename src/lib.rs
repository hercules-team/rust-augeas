extern crate libc;
extern crate augeas_sys;
use augeas_sys as raw;
use std::ptr;
use std::mem::transmute;
use std::ffi::{CString, CStr, NulError};
use libc::c_char;

pub use augeas_sys::AugFlag;

pub struct Augeas {
    aug: raw::augeas_t
}

/// Make a String from s; s must not be null
fn to_string(s : * const c_char) -> String {
    let s = unsafe { CStr::from_ptr(s) };
    String::from_utf8_lossy(s.to_bytes()).into_owned()
}

fn to_option(s : * const c_char) -> Option<String> {
    if s.is_null() {
        None
    } else {
        Some(to_string(s))
    }
}

impl Augeas {
    pub fn new(root: &str, loadpath: &str, flags: AugFlag) -> Result<Augeas, NulError> {
        let root_c = try!(CString::new(root));
        let loadpath_c = try!(CString::new(loadpath));

        let augeas = unsafe {
            raw::aug_init(root_c.as_ptr(), loadpath_c.as_ptr(), flags as u32)
        };

        Ok(Augeas{aug: augeas})
    }

    pub fn get(&self, path: &str) -> Result<Option<String>, NulError> {
        let path_c = try!(CString::new(path));
        let mut return_value: *mut c_char = ptr::null_mut();

        unsafe {
            raw::aug_get(self.aug, path_c.as_ptr(), &mut return_value);
        }

        Ok(to_option(return_value))
    }

    pub fn label(&self, path: &str) -> Result<Option<String>, NulError> {
        let path_c = try!(CString::new(path));
        let mut return_value: *const c_char = ptr::null();

        unsafe {
            raw::aug_label(self.aug, path_c.as_ptr(), &mut return_value);
        }

        Ok(to_option(return_value))
    }

    pub fn matches(&self, path: &str) -> Result<Vec<String>, NulError> {
        let c_path = try!(CString::new(path));

        unsafe {
            let mut matches_ptr: *mut *mut c_char = ptr::null_mut();

            let nmatches = raw::aug_match(self.aug, c_path.as_ptr(), &mut matches_ptr);

            let matches_vec = (0 .. nmatches).map(|i| {
                let match_ptr: *const c_char = transmute(*matches_ptr.offset(i as isize));
                let str = to_string(match_ptr);
                libc::free(transmute(match_ptr));
                str
            }).collect::<Vec<String>>();

            libc::free(transmute(matches_ptr));

            Ok(matches_vec)
        }
    }

    pub fn save(&mut self) -> bool {
        unsafe {
            raw::aug_save(self.aug) >= 0
        }
    }

    pub fn set(&mut self, path: &str, value: &str) -> Result<bool, NulError> {
        let path_c = try!(CString::new(path.as_bytes()));
        let value_c = try!(CString::new(value.as_bytes()));

        unsafe {
            Ok(0 <= raw::aug_set(self.aug, path_c.as_ptr(), value_c.as_ptr()))
        }
    }
}

impl Drop for Augeas {
    fn drop(&mut self) {
        unsafe {
            raw::aug_close(self.aug);
        }
    }
}

#[test]
fn get_test() {
    let aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();
    let root_uid = aug.get("etc/passwd/root/uid").unwrap().unwrap_or("unknown".to_string());

    assert!(&root_uid == "0", "ID of root was {}", root_uid);
}

#[test]
fn label_test() {
    let aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();
    let root_name = aug.label("etc/passwd/root").unwrap().unwrap_or("unknown".to_string());

    assert!(&root_name == "root", "name of root was {}", root_name);

}

#[test]
fn matches_test() {
    let aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    let users = aug.matches("etc/passwd/*").unwrap();

    println!("Users in passwd:");
    for user in users.iter() {
        println!("{}", &aug.label(&user).unwrap().unwrap_or("unknown".to_string()));
    }
}
