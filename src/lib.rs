extern crate augeas_sys;
extern crate libc;
#[macro_use] extern crate bitflags;

use augeas_sys::*;
use std::ptr;
use std::mem::transmute;
use std::ffi::CString;
use std::os::raw::{c_char,c_int};
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

#[derive(Clone, Copy)]
pub enum Position {
    Before,
    After
}

impl From<Position> for c_int {
    fn from(pos: Position) -> Self {
        match pos {
            Position::Before => 1,
            Position::After => 0
        }
    }
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
        let value = unsafe { ptr_to_string(value) };

        self.make_result(value)
    }

        pub fn label(&self, path: &str) -> Result<Option<String>> {
        let path_c = CString::new(path)?;
        let mut return_value: *const c_char = ptr::null();

        unsafe {
            aug_label(self.ptr, path_c.as_ptr(), &mut return_value)
        };

        self.make_result(unsafe { ptr_to_string(return_value) })
    }

    pub fn matches(&self, path: &str) -> Result<Vec<String>> {
        let c_path = CString::new(path)?;

        unsafe {
            let mut matches_ptr: *mut *mut c_char = ptr::null_mut();

            let nmatches = aug_match(self.ptr, c_path.as_ptr(), &mut matches_ptr);

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
        unsafe { aug_save(self.ptr) };
        self.make_result(())
    }

    pub fn set(&mut self, path: &str, value: &str) -> Result<()> {
        let path_c = CString::new(path.as_bytes())?;
        let value_c = CString::new(value.as_bytes())?;

        unsafe { aug_set(self.ptr, path_c.as_ptr(), value_c.as_ptr()) };
        self.make_result(())
    }

    pub fn insert(&mut self, path: &str, label: &str, pos:Position) -> Result<()> {
        let path = CString::new(path.as_bytes())?;
        let label = CString::new(label.as_bytes())?;

        unsafe { aug_insert(self.ptr, path.as_ptr(), label.as_ptr(), c_int::from(pos)) };
        self.make_result(())
    }

    pub fn rm(&mut self, path: &str) -> Result<u32> {
        let path = CString::new(path.as_bytes())?;
        let r = unsafe {
            aug_rm(self.ptr, path.as_ptr())
        };
        // coercing i32 to u32 is fine here since r is only negative
        // when an error occurred and make_result notices that from
        // the result of aug_error
        self.make_result(r as u32)
    }

    pub fn mv(&mut self, src: &str, dst: &str) -> Result<()> {
        let src = CString::new(src)?;
        let dst = CString::new(dst)?;

        unsafe { aug_mv(self.ptr, src.as_ptr(), dst.as_ptr()) };
        self.make_result(())
    }
}

impl Augeas {
    fn make_error<T>(&self) -> Result<T> {
        Err(Error::from(self))
    }

    fn make_result<T>(&self, v : T) -> Result<T> {
        let err = unsafe { aug_error(self.ptr) };
        let err = ErrorCode::from_raw(err as _);

        if err != ErrorCode::NoError {
            return self.make_error();
        }

        Ok(v)
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
fn insert_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    aug.insert("etc/passwd/root", "before", Position::Before).unwrap();
    aug.insert("etc/passwd/root", "after", Position::After).unwrap();
    let users = aug.matches("etc/passwd/*").unwrap();
    assert_eq!(["/files/etc/passwd/before",
                "/files/etc/passwd/root",
                "/files/etc/passwd/after"],
                users[0..3]);
}

#[test]
fn rm_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    let e = aug.rm("/augeas[");
    assert!(e.is_err());

    let r = aug.rm("etc/passwd").unwrap();
    assert_eq!(64, r);
}

#[test]
fn mv_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    let e = aug.mv("etc/passwd", "etc/passwd/root");
    assert!(e.is_err());

    aug.mv("etc/passwd", "etc/other").unwrap();
    assert_eq!(0, aug.matches("etc/passwd").unwrap().len());
    assert_eq!(1, aug.matches("etc/other").unwrap().len());
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
