extern crate libc;
extern crate augeas_sys;
use augeas_sys as raw;
use std::ptr;
use std::mem::transmute;
use std::ffi::{CString, CStr, NulError};
use libc::c_char;
use std::convert::From;

use std::fmt;
use std::error::Error;

pub use augeas_sys::AugFlag;

pub struct Augeas {
    aug: raw::augeas_t
}

#[derive(Clone, PartialEq,Debug,Default)]
pub struct AugError {
    code          : raw::AugError,
    message       : Option<String>,
    minor_message : Option<String>,
    details       : Option<String>
}

impl Error for AugError {
    fn description(&self) -> &str {
        match self.message {
            None => "No description",
            Some(ref s) => s
        }
    }
}

fn maybe_write(f: &mut fmt::Formatter, opt : &Option<String>) -> fmt::Result {
    match *opt {
        Some(ref s) => write!(f, "      {}\n", s),
        None => Ok(())
    }
}

impl fmt::Display for AugError {
    // Write
    //   augeas error:{code}:{message}
    //                {minor_message}
    //                {details}
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let m = self.message.as_ref().map(String::as_ref).unwrap_or("");
        write!(f, "augeas error:{:?}:{}\n", self.code, m).
            and(maybe_write(f, &self.minor_message)).
            and(maybe_write(f, &self.details))
    }
}

pub type Result<T> = std::result::Result<T, AugError>;

impl From<NulError> for AugError {
    fn from(_ : NulError) -> AugError {
        AugError { code : raw::AugError::NulString,
                   message : Some(String::from("Rust string contains NUL")),
                   .. Default::default() }
    }
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
    fn make_error<T>(&self) -> Result<T> {
        let err = unsafe { raw::aug_error(self.aug) };
        let msg = to_option(unsafe { raw::aug_error_message(self.aug) });
        let mmsg = to_option(unsafe { raw::aug_error_minor_message(self.aug) });
        let det = to_option(unsafe { raw::aug_error_details(self.aug) });
        Err(AugError { code : err,
                       message : msg,
                       minor_message : mmsg,
                       details : det })
    }

    fn make_result<T>(&self, v : T) -> Result<T> {
        let err = unsafe { raw::aug_error(self.aug) };
        if err == raw::AugError::NoError {
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
            let m = String::from("Failed to initialize Augeas");
            Err(AugError { code : raw::AugError::NoMem, message : Some(m),
                           .. Default::default() })
        } else {
            Ok(Augeas{aug: augeas})
        }
    }

    pub fn get(&self, path: &str) -> Result<Option<String>> {
        let path_c = try!(CString::new(path));
        let mut return_value: *mut c_char = ptr::null_mut();

        unsafe { raw::aug_get(self.aug, path_c.as_ptr(), &mut return_value) };

        self.make_result(to_option(return_value))
    }

    pub fn label(&self, path: &str) -> Result<Option<String>> {
        let path_c = try!(CString::new(path));
        let mut return_value: *const c_char = ptr::null();

        unsafe {
            raw::aug_label(self.aug, path_c.as_ptr(), &mut return_value)
        };

        self.make_result(to_option(return_value))
    }

    pub fn matches(&self, path: &str) -> Result<Vec<String>> {
        let c_path = try!(CString::new(path));

        unsafe {
            let mut matches_ptr: *mut *mut c_char = ptr::null_mut();

            let nmatches = raw::aug_match(self.aug, c_path.as_ptr(), &mut matches_ptr);

            if nmatches < 0 {
                return self.make_error()
            }
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

    pub fn save(&mut self) -> Result<()> {
        unsafe { raw::aug_save(self.aug) };
        self.make_result(())
    }

    pub fn set(&mut self, path: &str, value: &str) -> Result<()> {
        let path_c = try!(CString::new(path.as_bytes()));
        let value_c = try!(CString::new(value.as_bytes()));

        unsafe { raw::aug_set(self.aug, path_c.as_ptr(), value_c.as_ptr()) };
        self.make_result(())
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

    let nothing = aug.get("/foo");
    assert!(nothing.is_ok());
    assert!(nothing.ok().unwrap().is_none());

    let many = aug.get("etc/passwd/*");
    assert!(many.is_err());
    assert!(many.err().unwrap().code == raw::AugError::ManyMatches)
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
    assert!(garbled.is_err());
    let err = garbled.err().unwrap();
    assert!(err.code == raw::AugError::PathExpr);
    println!("{}", err);
    assert!(err.message.unwrap() == "Invalid path expression");
    assert!(err.minor_message.unwrap() == "illegal string literal");
    assert!(err.details.unwrap() == "/invalid[|=|")
}
