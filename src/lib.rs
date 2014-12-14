extern crate libc;

use std::ptr;
use std::mem::transmute;
use libc::c_char;
pub use raw::AugFlags;

mod raw;

pub struct Augeas {
    aug: raw::augeas
}

impl Augeas {
    pub fn new(root: &str, loadpath: &str, flags: AugFlags) -> Augeas {
        let root_c = (*root).to_c_str();
        let loadpath_c = (*loadpath).to_c_str();

        let augeas = unsafe {
            raw::aug_init(root_c.as_ptr(), loadpath_c.as_ptr(), flags as u32)
        };

        Augeas{aug: augeas}
    }

    pub fn get(&self, path: &str) -> Option<String> {
        let path_c = path.to_c_str();
        let return_value = &mut ptr::null();

        unsafe {
            raw::aug_get(self.aug, path_c.as_ptr(), return_value);
        }

        match (*return_value).is_null() {
            true => None,
            false => unsafe {
                Some(String::from_raw_buf(transmute(*return_value)))
            }
        }
    }

    pub fn label(&self, path: &str) -> Option<String> {
        let path_c = path.to_c_str();
        let return_value = &mut ptr::null();

        unsafe {
            raw::aug_label(self.aug, path_c.as_ptr(), return_value);
        }

        match (*return_value).is_null() {
            true => None,
            false => unsafe {
                Some(String::from_raw_buf(transmute(*return_value)))
            }
        }
    }

    pub fn matches(&self, path: &str) -> Vec<String> {
        let c_path = path.to_c_str();

        unsafe {
            let mut matches_ptr: *mut *mut c_char = ptr::null_mut();

            let nmatches = raw::aug_match(self.aug, c_path.as_ptr(), transmute(&mut matches_ptr)) as uint;

            let matches_vec = Vec::from_fn(nmatches, |i| {
                let match_ptr = *matches_ptr.offset(i as int);
                let str = String::from_raw_buf(transmute(match_ptr));
                libc::free(transmute(match_ptr));
                str
            });

            libc::free(transmute(matches_ptr));

            matches_vec
        }
    }

    pub fn save(&mut self) -> bool {
        unsafe {
            raw::aug_save(self.aug) >= 0
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
        println!("{}", user);
    }
}
