extern crate libc;
extern crate augeas_sys;
use augeas_sys as raw;
use std::ptr;
use std::mem::transmute;
use std::ffi::CString;
use libc::c_char;
use std::convert::From;

pub mod error;
pub use error::Error;
use error::AugeasError;

mod util;
use util::ptr_to_string;

pub use augeas_sys::AugFlag;

pub struct Augeas {
    ptr: raw::augeas_t
}

pub type Result<T> = std::result::Result<T, Error>;

impl Augeas {
    fn make_error<T>(&self) -> Result<T> {
        Err(Error::from(self))
    }

    fn make_result<T>(&self, v : T) -> Result<T> {
        let err = unsafe { raw::aug_error(self.ptr) };
        if err == raw::ErrorCode::NoError {
            Ok(v)
        } else {
            self.make_error()
        }
    }

    pub fn new(root: &str, loadpath: &str, flags: AugFlag) -> Result<Augeas> {
        let root_c = try!(CString::new(root));
        let loadpath_c = try!(CString::new(loadpath));

        let augeas = unsafe {
            raw::aug_init(root_c.as_ptr(), loadpath_c.as_ptr(), flags as u32)
        };
        if augeas.is_null() {
            let message = String::from("Failed to initialize Augeas");
            Err(Error::Augeas(AugeasError::new_no_mem(message)))
        } else {
            Ok(Augeas{
                ptr: augeas
            })
        }
    }

    pub fn get(&self, path: &str) -> Result<Option<String>> {
        let path_c = try!(CString::new(path));
        let mut return_value: *mut c_char = ptr::null_mut();

        unsafe { raw::aug_get(self.ptr, path_c.as_ptr(), &mut return_value) };

        self.make_result(unsafe { ptr_to_string(return_value) })
    }

    pub fn label(&self, path: &str) -> Result<Option<String>> {
        let path_c = try!(CString::new(path));
        let mut return_value: *const c_char = ptr::null();

        unsafe {
            raw::aug_label(self.ptr, path_c.as_ptr(), &mut return_value)
        };

        self.make_result(unsafe { ptr_to_string(return_value) })
    }

    pub fn matches(&self, path: &str) -> Result<Vec<String>> {
        let c_path = try!(CString::new(path));

        unsafe {
            let mut matches_ptr: *mut *mut c_char = ptr::null_mut();

            let nmatches = raw::aug_match(self.ptr, c_path.as_ptr(), &mut matches_ptr);

            if nmatches < 0 {
                return self.make_error()
            }
            let matches_vec = (0 .. nmatches).map(|i| {
                let match_ptr: *const c_char = transmute(*matches_ptr.offset(i as isize));
                let str = ptr_to_string(match_ptr).unwrap();
                libc::free(transmute(match_ptr));
                str
            }).collect::<Vec<String>>();

            libc::free(transmute(matches_ptr));

            Ok(matches_vec)
        }
    }

    pub fn save(&mut self) -> Result<()> {
        unsafe { raw::aug_save(self.ptr) };
        self.make_result(())
    }

    pub fn set(&mut self, path: &str, value: &str) -> Result<()> {
        let path_c = try!(CString::new(path.as_bytes()));
        let value_c = try!(CString::new(value.as_bytes()));

        unsafe { raw::aug_set(self.ptr, path_c.as_ptr(), value_c.as_ptr()) };
        self.make_result(())
    }
}

impl Drop for Augeas {
    fn drop(&mut self) {
        unsafe {
            raw::aug_close(self.ptr);
        }
    }
}

#[test]
fn get_test() {
    let aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();
    let root_uid = aug.get("etc/passwd/root/uid").unwrap().unwrap_or("unknown".to_string());

    assert!(&root_uid == "0", "ID of root was {}", root_uid);

    let nothing = aug.get("/foo");
    assert!(nothing.is_ok());
    assert!(nothing.ok().unwrap().is_none());

    let many = aug.get("etc/passwd/*");

    if let Err(Error::Augeas(err)) = many {
        assert!(err.code == raw::ErrorCode::ManyMatches)
    } else {
        panic!("Unexpected value: {:?}", many)
    }
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

#[test]
fn error_test() {
    let aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();
    let garbled = aug.matches("/invalid[");

    if let Err(Error::Augeas(err)) = garbled {
        assert!(err.code == raw::ErrorCode::PathExpr);
        assert!(err.message.unwrap() == "Invalid path expression");
        assert!(err.minor_message.unwrap() == "illegal string literal");
        assert!(err.details.unwrap() == "/invalid[|=|")
    } else {
        panic!("Unexpected value: {:?}", garbled)
    }
}
