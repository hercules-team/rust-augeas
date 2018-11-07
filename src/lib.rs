extern crate augeas_sys;
extern crate libc;
#[macro_use] extern crate bitflags;

use augeas_sys::*;
use std::ptr;
use std::mem::transmute;
use std::ffi::CString;
use std::os::raw::c_char;
use std::convert::From;

pub mod error;
pub use error::Error;
use error::AugeasError;
use error::ErrorCode;

mod flags;
pub use self::flags::Flags;

mod util;
use util::ptr_to_string;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Augeas {
    ptr: *mut augeas,
}

impl Augeas {
    pub fn init<'a>(root: impl Into<Option<&'a str>>, loadpath: &str, flags: Flags) -> Result<Self> {
        let ref root = match root.into() {
            Some(root) => Some(CString::new(root)?),
            None => None,
        };
        let root = match root {
            Some(root) => root.as_ptr(),
            None => ptr::null(),
        };
        let ref loadpath = CString::new(loadpath)?;
        let loadpath = loadpath.as_ptr();
        let flags = flags.bits();
        let augeas = unsafe { aug_init(root, loadpath, flags) };
        
        if augeas.is_null() {
            let message = String::from("Failed to initialize Augeas");
            return Err(Error::Augeas(AugeasError::new_no_mem(message)));
        }

        Ok(Augeas {
            ptr: augeas,
        })
    }

    pub fn get(&self, path: &str) -> Result<Option<String>> {
        let ref path = CString::new(path)?;
        let path = path.as_ptr();
        let mut value: *const c_char = ptr::null_mut();

        unsafe { aug_get(self.ptr, path, &mut value) };
        self.check_error()?;

        let value = unsafe { ptr_to_string(value) };

        Ok(value)
    }

    pub fn label(&self, path: &str) -> Result<Option<String>> {
        let path_c = CString::new(path)?;
        let mut value: *const c_char = ptr::null();

        unsafe { aug_label(self.ptr, path_c.as_ptr(), &mut value) };
        self.check_error()?;

        let value = unsafe { ptr_to_string(value) };

        Ok(value)
    }

    pub fn matches(&self, path: &str) -> Result<Vec<String>> {
        let c_path = CString::new(path)?;

        unsafe {
            let mut matches_ptr: *mut *mut c_char = ptr::null_mut();
            let nmatches = aug_match(self.ptr, c_path.as_ptr(), &mut matches_ptr);
            self.check_error()?;

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
        unsafe { aug_save(self.ptr) };
        self.check_error()?;

        Ok(())
    }

    pub fn set(&mut self, path: &str, value: &str) -> Result<()> {
        let path_c = CString::new(path.as_bytes())?;
        let value_c = CString::new(value.as_bytes())?;

        unsafe { aug_set(self.ptr, path_c.as_ptr(), value_c.as_ptr()) };
        self.check_error()?;

        Ok(())
    }

    fn check_error(&self) -> std::result::Result<(), AugeasError> {
        self.error().map(Err).unwrap_or(Ok(()))
    }

    fn error(&self) -> Option<AugeasError> {
        let err = unsafe { aug_error(self.ptr) };
        let err = ErrorCode::from_raw(err as _)?;
        let msg = unsafe { ptr_to_string(aug_error_message(self.ptr)) };
        let mmsg = unsafe { ptr_to_string(aug_error_minor_message(self.ptr)) };
        let det = unsafe { ptr_to_string(aug_error_details(self.ptr)) };

        Some(AugeasError {
            code: err,
            message: msg,
            minor_message: mmsg,
            details: det
       })
    }
}

impl Drop for Augeas {
    fn drop(&mut self) {
        unsafe {
            aug_close(self.ptr);
        }
    }
}

#[test]
fn get_test() {
    use error::ErrorCode;
    let aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();
    let root_uid = aug.get("etc/passwd/root/uid").unwrap().unwrap_or("unknown".to_string());

    assert!(&root_uid == "0", "ID of root was {}", root_uid);

    let nothing = aug.get("/foo");
    assert!(nothing.is_ok());
    assert!(nothing.ok().unwrap().is_none());

    let many = aug.get("etc/passwd/*");

    if let Err(Error::Augeas(err)) = many {
        assert!(err.code == ErrorCode::ManyMatches)
    } else {
        panic!("Unexpected value: {:?}", many)
    }
}

#[test]
fn label_test() {
    let aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();
    let root_name = aug.label("etc/passwd/root").unwrap().unwrap_or("unknown".to_string());

    assert!(&root_name == "root", "name of root was {}", root_name);

}

#[test]
fn matches_test() {
    let aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    let users = aug.matches("etc/passwd/*").unwrap();

    println!("Users in passwd:");
    for user in users.iter() {
        println!("{}", &aug.label(&user).unwrap().unwrap_or("unknown".to_string()));
    }
}

#[test]
fn error_test() {
    let aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();
    let garbled = aug.matches("/invalid[");

    if let Err(Error::Augeas(err)) = garbled {
        assert!(err.code == ErrorCode::PathExpr);
        assert!(err.message.unwrap() == "Invalid path expression");
        assert!(err.minor_message.unwrap() == "illegal string literal");
        assert!(err.details.unwrap() == "/invalid[|=|")
    } else {
        panic!("Unexpected value: {:?}", garbled)
    }
}
