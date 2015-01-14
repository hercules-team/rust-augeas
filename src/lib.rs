extern crate libc;

use std::ptr;
use std::mem::transmute;
use std::ffi::CString;
use std::ffi::c_str_to_bytes;
use libc::c_char;
pub use raw::AugFlags;

mod raw;

pub struct Augeas {
    aug: raw::augeas
}

impl Augeas {
    pub fn new(root: &str, loadpath: &str, flags: AugFlags) -> Augeas {
        let root_c = CString::from_slice(root.as_bytes());
        let loadpath_c = CString::from_slice(loadpath.as_bytes());

        let augeas = unsafe {
            raw::aug_init(root_c.as_ptr(), loadpath_c.as_ptr(), flags as u32)
        };

        Augeas{aug: augeas}
    }

    pub fn get(&self, path: &str) -> Option<String> {
        let path_c = CString::from_slice(path.as_bytes());
        let mut return_value: *const c_char = ptr::null();

        unsafe {
            raw::aug_get(self.aug, path_c.as_ptr(), &mut return_value);
        }

        match return_value.is_null() {
            true => None,
            false => unsafe {
                Some(String::from_utf8_lossy(c_str_to_bytes(&return_value)).into_owned())
            }
        }
    }

    pub fn label(&self, path: &str) -> Option<String> {
        let path_c = CString::from_slice(path.as_bytes());
        let mut return_value: *const c_char = ptr::null();

        unsafe {
            raw::aug_label(self.aug, path_c.as_ptr(), &mut return_value);
        }

        match return_value.is_null() {
            true => None,
            false => unsafe {
                Some(String::from_utf8_lossy(c_str_to_bytes(&return_value)).into_owned())
            }
        }
    }

    pub fn matches(&self, path: &str) -> Vec<String> {
        let c_path = CString::from_slice(path.as_bytes());

        unsafe {
            let mut matches_ptr: *mut *mut c_char = ptr::null_mut();

            let nmatches = raw::aug_match(self.aug, c_path.as_ptr(), &mut matches_ptr);

            let matches_vec = range(0, nmatches).map(|i| {
                let match_ptr: *const c_char = transmute(*matches_ptr.offset(i as isize));
                let str = String::from_utf8_lossy(c_str_to_bytes(&match_ptr)).into_owned();
                libc::free(transmute(match_ptr));
                str
            }).collect::<Vec<String>>();

            libc::free(transmute(matches_ptr));

            matches_vec
        }
    }

    pub fn save(&mut self) -> bool {
        unsafe {
            raw::aug_save(self.aug) >= 0
        }
    }

    pub fn set(&mut self, path: &str, value: &str) -> bool {
        let path_c = CString::from_slice(path.as_bytes());
        let value_c = CString::from_slice(value.as_bytes());

        unsafe {
            0 <= raw::aug_set(self.aug, path_c.as_ptr(), value_c.as_ptr())
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
    let aug = Augeas::new("", "", AugFlags::None);
    let root_uid = aug.get("etc/passwd/root/uid").unwrap_or("unknown".to_string());

    assert!(root_uid.as_slice() == "0", "ID of root was {}", root_uid);
}

#[test]
fn label_test() {
    let aug = Augeas::new("", "", AugFlags::None);
    let root_name = aug.label("etc/passwd/root").unwrap_or("unknown".to_string());

    assert!(root_name.as_slice() == "root", "name of root was {}", root_name);

}

#[test]
fn matches_test() {
    let aug = Augeas::new("", "", AugFlags::None);
    
    let users = aug.matches("etc/passwd/*");

    println!("Users in passwd:");
    for user in users.iter() {
        println!("{}", aug.label(user.as_slice()).unwrap_or("unknown".to_string()).as_slice());
    }
}
