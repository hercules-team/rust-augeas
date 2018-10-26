extern crate libc;
extern crate augeas_sys;
use augeas_sys as raw;
use std::ptr;
use std::mem::transmute;
use std::ffi::CString;
use libc::{c_char,c_int};
use std::convert::From;
use std::ops::Range;

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

#[derive(Debug)]
pub struct Span {
    pub label : Range<u32>,
    pub value : Range<u32>,
    pub span  : Range<u32>,
    pub filename : Option<String>
}

impl Span {
    fn new() -> Span {
        Span {
            label: 0..0,
            value: 0..0,
            span: 0..0,
            filename: None
        }
    }
}

impl Augeas {
    fn make_error<T>(&self) -> Result<T> {
        Err(Error::from(self))
    }

    // Check self for an error, and return the resulting error. If there
    // was no error, return Ok(v)
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

    pub fn count(&self, path: &str) -> Result<u32> {
        let path = CString::new(path)?;

        let r = unsafe { raw::aug_match(self.ptr, path.as_ptr(), ptr::null_mut()) };

        self.make_result(r as u32)
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

    pub fn insert(&mut self, path: &str, label: &str, pos:Position) -> Result<()> {
        let path = CString::new(path.as_bytes())?;
        let label = CString::new(label.as_bytes())?;

        unsafe { raw::aug_insert(self.ptr, path.as_ptr(), label.as_ptr(), c_int::from(pos)) };
        self.make_result(())
    }

    pub fn rm(&mut self, path: &str) -> Result<u32> {
        let path = CString::new(path.as_bytes())?;
        let r = unsafe {
            raw::aug_rm(self.ptr, path.as_ptr())
        };
        // coercing i32 to u32 is fine here since r is only negative
        // when an error occurred and make_result notices that from
        // the result of aug_error
        self.make_result(r as u32)
    }

    pub fn mv(&mut self, src: &str, dst: &str) -> Result<()> {
        let src = CString::new(src)?;
        let dst = CString::new(dst)?;

        unsafe { raw::aug_mv(self.ptr, src.as_ptr(), dst.as_ptr()) };
        self.make_result(())
    }

    pub fn defvar(&mut self, name: &str, expr: &str) -> Result<()> {
        let name = CString::new(name)?;
        let expr = CString::new(expr)?;

        unsafe { raw::aug_defvar(self.ptr, name.as_ptr(), expr.as_ptr()) };
        self.make_result(())
    }

    pub fn defnode(&mut self, name: &str, expr: &str, value: &str) -> Result<bool> {
        let name = CString::new(name)?;
        let expr = CString::new(expr)?;
        let value = CString::new(value)?;
        let mut cr : i32 = 0;

        unsafe { raw::aug_defnode(self.ptr, name.as_ptr(), expr.as_ptr(),
                                  value.as_ptr(), &mut cr) };
        self.make_result(cr == 1)
    }

    pub fn load(&mut self) -> Result<()> {
        unsafe { raw::aug_load(self.ptr) };
        self.make_result(())
    }

    pub fn setm(&mut self, base: &str, sub: &str, value: &str) -> Result<(u32)> {
        let base = CString::new(base)?;
        let sub = CString::new(sub)?;
        let value = CString::new(value)?;

        let r = unsafe { raw::aug_setm(self.ptr, base.as_ptr(), sub.as_ptr(),
                                       value.as_ptr()) };
        self.make_result(r as u32)
    }

    pub fn span(&self, path: &str) -> Result<Option<Span>> {
        let path = CString::new(path)?;
        let mut filename : *mut c_char = ptr::null_mut();
        let mut result = Span::new();

        unsafe {
         raw::aug_span(self.ptr, path.as_ptr(), &mut filename,
            &mut result.label.start, &mut result.label.end,
            &mut result.value.start, &mut result.value.end,
            &mut result.span.start, &mut result.span.end);
        }

        let err = unsafe { raw::aug_error(self.ptr) };
        if err != raw::ErrorCode::NoError {
            if err == raw::ErrorCode::NoSpan {
                return Ok(None);
            } else {
                return self.make_result(None);
            }
        }

        result.filename = unsafe { ptr_to_string(filename) };
        unsafe { libc::free(filename as *mut libc::c_void) };
        Ok(Some(result))
    }

    pub fn text_store(&mut self, lens: &str, node: &str, path: &str) -> Result<()> {
        let err_path = format!("/augeas/text{}/error", path);

        let lens = CString::new(lens)?;
        let node = CString::new(node)?;
        let path = CString::new(path)?;

        unsafe { raw::aug_text_store(self.ptr, lens.as_ptr(), node.as_ptr(), 
                                     path.as_ptr()) };
        
        let err = self.get(&err_path)?;
        if let Some(kind) = err {
            return Err(Error::from(kind));
        }
        self.make_result(())
    }

    pub fn text_retrieve(&mut self, lens: &str,
        node_in: &str, path: &str,
        node_out: &str) -> Result<()> {
        let err_path = format!("/augeas/text{}/error", path);

        let lens = CString::new(lens)?;
        let node_in = CString::new(node_in)?;
        let path = CString::new(path)?;
        let node_out = CString::new(node_out)?;

        unsafe { raw::aug_text_retrieve(self.ptr, lens.as_ptr(),
                    node_in.as_ptr(), path.as_ptr(), node_out.as_ptr()) };

        let err = self.get(&err_path)?;
        if let Some(kind) = err {
            return Err(Error::from(kind));
        }

        self.make_result(())
    }   

    pub fn rename(&mut self, src: &str, lbl: &str) -> Result<()> {
        let src = CString::new(src)?;
        let lbl = CString::new(lbl)?;

        unsafe { raw::aug_rename(self.ptr, src.as_ptr(), lbl.as_ptr()) };
        self.make_result(())
    }

    pub fn transform(&mut self, lens: &str, file: &str, excl : bool) -> Result<()> {
        let lens = CString::new(lens)?;
        let file = CString::new(file)?;

        unsafe { raw::aug_transform(self.ptr, lens.as_ptr(),
                                    file.as_ptr(), excl as i32) };
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
    let count = aug.count("etc/passwd/*").unwrap();

    assert_eq!(9, users.len());
    assert_eq!(9, count);
    assert_eq!("/files/etc/passwd/root", users[0]);
    assert_eq!("/files/etc/passwd/nobody", users[8]);
}

#[test]
fn insert_test() {
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

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
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    let e = aug.rm("/augeas[");
    assert!(e.is_err());

    let r = aug.rm("etc/passwd").unwrap();
    assert_eq!(64, r);
}

#[test]
fn mv_test() {
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    let e = aug.mv("etc/passwd", "etc/passwd/root");
    assert!(e.is_err());

    aug.mv("etc/passwd", "etc/other").unwrap();
    assert_eq!(0, aug.count("etc/passwd").unwrap());
    assert_eq!(1, aug.count("etc/other").unwrap());
}

#[test]
fn defvar_test() {
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    aug.defvar("x", "etc/passwd/*").unwrap();
    let n = aug.count("$x").unwrap();

    assert_eq!(9, n);
}

#[test]
fn defnode_test() {
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    let created = aug.defnode("y", "etc/notthere", "there").unwrap();
    assert!(created);

    let there = aug.get("$y").unwrap();
    assert_eq!("there", there.expect("failed to get etc/notthere"));

    let created = aug.defnode("z", "etc/passwd", "there").unwrap();
    assert!(! created);
}

#[test]
fn load_test() {
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    aug.set("etc/passwd/root/uid", "42").unwrap();
    aug.load().unwrap();
    let uid = aug.get("etc/passwd/root/uid").unwrap();
    assert_eq!("0", uid.expect("expected value for root/uid"));
}

#[test]
fn setm_test() {
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    let count = aug.setm("etc/passwd", "*/shell", "/bin/zsh").unwrap();
    assert_eq!(9, count);
}

#[test]
fn span_test() {
    let aug = Augeas::new("tests/test_root", "", AugFlag::EnableSpan).unwrap();

    // happy path
    let span = aug.span("etc/passwd/root").unwrap().unwrap();
    assert_eq!(0..4, span.label);
    assert_eq!(0..0, span.value);
    assert_eq!(0..32, span.span);
    assert_eq!("tests/test_root/etc/passwd", span.filename.unwrap());
    
    // no span info associated with node
    let span = aug.span("/augeas/load").unwrap();
    assert!(span.is_none());

    // too many matches
    let span = aug.span("etc/passwd/*");
    assert!(span.is_err());
}

#[test]
fn store_retrieve_test() {
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    aug.set("/text/in", "alex:x:12:12:Alex:/home/alex:/bin/sh\n").unwrap();
    aug.text_store("Passwd.lns", "/text/in", "/stored").unwrap();
    aug.set("/stored/alex/uid", "17").unwrap();

    aug.text_retrieve("Passwd.lns", "/text/in", "/stored", "/text/out").unwrap();
    let text = aug.get("/text/out").unwrap().unwrap();
    assert_eq!("alex:x:17:12:Alex:/home/alex:/bin/sh\n", text);

    // Invalidate the tree; 'shell' must be present
    aug.rm("/stored/alex/shell").unwrap();
    let err = aug.text_retrieve("Passwd.lns", "/text/in", "/stored", "/text/out").err().unwrap();
    assert_eq!("parse error of kind put_failed", format!("{}", err));

    aug.set("/text/in", "alex:invalid passwd entry").unwrap();
    let err = aug.text_store("Passwd.lns", "/text/in", "/stored").err().unwrap();
    assert_eq!("parse error of kind parse_failed", format!("{}", err));
}

#[test]
fn rename_test() {
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    aug.rename("etc/passwd/root", "ruth").unwrap();

    let ruth = aug.get("etc/passwd/ruth/uid").unwrap().unwrap();
    assert_eq!("0", ruth);

    let root = aug.get("etc/passwd/root/uid").unwrap();
    assert!(root.is_none());
}

#[test]
fn transform_test() {
    let mut aug = Augeas::new("tests/test_root", "", AugFlag::None).unwrap();

    aug.transform("Hosts.lns", "/usr/local/etc/hosts", false).unwrap();
    let p = aug.get("/augeas/load/Hosts/incl[. = '/usr/local/etc/hosts']").unwrap();
    assert!(p.is_some());
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
