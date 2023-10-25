//! Augeas bindings for Rust
//!
//! ## Usage
//!
//! In `Cargo.toml`
//! ```toml
//! [dependencies.augeas]
//! version = "0.1.0"
//! ```
//!
//! ## Summary
//!
//! This crate implements Rust bindings to [augeas](http://augeas.net/), a library for reading,
//! modifying, and writing structured file, like configuration files.
//!
//! A typical interaction looks like this:
//!
//! ```
//!   extern crate augeas_sys;
//!   extern crate augeas;
//!
//!   use augeas_sys;
//!   use augeas::{Augeas,Flags};
//!
//!   let mut aug = Augeas::init("/", "", Flags::None).unwrap();
//!
//!   let entry = aug.get("etc/hosts/*[canonical = 'host.example.com']/ip").unwrap();
//!
//!   if let Some(ip) = entry {
//!     println!("The ip for host.example.com is {}", ip);
//!   } else {
//!     println!("There is no entry for host.example.com in /etc/hosts");
//!   }
//!
//!   // Add an alias for host.example.com
//!   aug.set("etc/hosts/*[canonical = 'host.example.com']/alias[last()+1]", "server.example.com");
//! ```
extern crate augeas_sys;
extern crate libc;
#[macro_use] extern crate bitflags;

use augeas_sys::*;
use std::ptr;
use std::mem::transmute;
use std::ffi::CString;
use std::os::raw::{c_char,c_int};
use std::convert::From;
use std::ops::Range;

pub mod error;
pub use error::Error;
use error::AugeasError;
use error::ErrorCode;

mod flags;
pub use self::flags::Flags;

mod util;
use util::ptr_to_string;

pub type Result<T> = std::result::Result<T, Error>;

/// The handle for the Augeas library.
///
/// The Augeas handle points to the in-memory data that Augeas manages, in
/// particular, the tree generated from parsing configuration files.
pub struct Augeas {
    ptr: *mut augeas,
}

/// The insert position.
///
/// Use this enum with [`insert`](#method.insert) to indicate whether the
/// new node should be inserted before or after the node passed to
/// [`insert`](#method.insert)
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

/// A span in the tree
///
/// The [`span`](#method.span) indicates where in a file a particular node
/// was found. The `label` and `value` give the character range from which
/// the node's label and value were read, and `span` gives the entire region
/// in the file from which the node was construted. If any of these values are
/// not available, e.g., because the node has no value, the corresponding range
/// will be empty.
///
/// The `filename` provides the entire path to the file in the file system
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

#[derive(Debug)]
pub struct Attr {
    pub label : Option<String>,
    pub value : Option<String>,
    pub file_path : Option<String>
}

impl Augeas {
    /// Create a new Augeas handle.
    ///
    /// Use `root` as the filesystem root. If `root` is `None`, use the value
    /// of the environment variable `AUGEAS_ROOT`, if it is set; otherwise,
    /// use "/".
    ///
    /// The `loadpath` is a colon-separated list of directories that modules
    /// should be searched in. This is in addition to the standard load path
    /// and the directories listed in the environment variable AUGEAS_LENS_LIB
    ///
    /// The `flags` are a bitmask of values from `AugFlag`.
    ///
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

    /// Lookup the value associated with `path`.
    ///
    /// Return `None` if there is no value associated with `path` or if no
    /// node matches `path`, and `Some(value)` if there is exactly one node
    /// with a value.
    ///
    /// It is an error if `path` matches more than one node.
    pub fn get(&self, path: &str) -> Result<Option<String>> {
        let ref path = CString::new(path)?;
        let path = path.as_ptr();
        let mut value: *const c_char = ptr::null_mut();
        unsafe { aug_get(self.ptr, path, &mut value) };

        self.make_result(ptr_to_string(value))
    }

    /// Lookup the label associated with `path`.
    ///
    /// Return `Some(label)` if `path` matches a node that has a label, and
    /// `None` if `path` matches no node, or matches exactly one node
    /// without a label.
    ///
    /// It is an error if `path` matches more than one node.
    pub fn label(&self, path: &str) -> Result<Option<String>> {
        let path_c = CString::new(path)?;
        let mut return_value: *const c_char = ptr::null();

        unsafe {
            aug_label(self.ptr, path_c.as_ptr(), &mut return_value)
        };

        self.make_result(ptr_to_string(return_value))
    }

    /// Find all nodes matching `path`
    ///
    /// Find all nodes matching the path expression `path` and return their
    /// paths in an unambiguous form that can be used with
    /// [`get`](#method.get) to get their value.
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

    /// Count the nodes matching `path`
    ///
    /// Find all nodes matching the path expression `path` and return how
    /// many there are.
    pub fn count(&self, path: &str) -> Result<u32> {
        let path = CString::new(path)?;

        let r = unsafe { aug_match(self.ptr, path.as_ptr(), ptr::null_mut()) };

        self.make_result(r as u32)
    }

    /// Save all pending changes to disk
    ///
    /// Turn all files in the tree for which entries have been changed,
    /// added, or deleted back into text and write them back to file.
    pub fn save(&mut self) -> Result<()> {
        unsafe { aug_save(self.ptr) };
        self.make_result(())
    }

    /// Set the value of a node.
    ///
    /// Find the node matching `path` and change its value. If there is no
    /// such node, an attempt is made to create one, though that might not
    /// be possible depending on the complexity of the path expression
    /// `path`.
    ///
    /// It is an error if more than one node matches `path`
    pub fn set(&mut self, path: &str, value: &str) -> Result<()> {
        let path_c = CString::new(path.as_bytes())?;
        let value_c = CString::new(value.as_bytes())?;

        unsafe { aug_set(self.ptr, path_c.as_ptr(), value_c.as_ptr()) };
        self.make_result(())
    }

    /// Insert a new node: find the node matching `path` and create a new
    /// sibling for it with the given `label`. The sibling is created
    /// before or after the node `path`, depending on the value of `pos`.
    ///
    /// It is an error if `path` matches no nodes, or more than one
    /// node. The `label` must not contain a `/`, `*` or end with a
    /// bracketed index `[N]`.
    pub fn insert(&mut self, path: &str, label: &str, pos:Position) -> Result<()> {
        let path = CString::new(path.as_bytes())?;
        let label = CString::new(label.as_bytes())?;

        unsafe { aug_insert(self.ptr, path.as_ptr(), label.as_ptr(), c_int::from(pos)) };
        self.make_result(())
    }

    /// Remove `path` and all its children and return the number of nodes
    /// removed.
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

    /// Move the node matching `src` to `dst`.
    pub fn mv(&mut self, src: &str, dst: &str) -> Result<()> {
        let src = CString::new(src)?;
        let dst = CString::new(dst)?;

        unsafe { aug_mv(self.ptr, src.as_ptr(), dst.as_ptr()) };
        self.make_result(())
    }

    /// Define a variable `name` whose value is the result of evaluating
    /// `expr`. If a variable `name` already exists, its name will be
    /// replaced with the result of evaluating `expr`.  Context will not be
    /// applied to `expr`.
    ///
    /// Path variables can be used in path expressions later on by prefixing
    /// them with '$'.
    pub fn defvar(&mut self, name: &str, expr: &str) -> Result<()> {
        let name = CString::new(name)?;
        let expr = CString::new(expr)?;

        unsafe { aug_defvar(self.ptr, name.as_ptr(), expr.as_ptr()) };
        self.make_result(())
    }

    /// Remove the variable `name`.
    ///
    /// It is not an error if the variable does not exist.
    pub fn rmvar(&mut self, name: &str) -> Result<()> {
        let name = CString::new(name)?;

        unsafe { aug_defvar(self.ptr, name.as_ptr(), ptr::null_mut()) };
        self.make_result(())
    }

    /// Define a variable `name` whose value is the result of evaluating `expr`,
    /// which must be non-NULL and evaluate to a nodeset. If a variable `name`
    /// already exists, its name will be replaced with the result of evaluating
    /// `expr`.
    ///
    /// If `expr` evaluates to an empty nodeset, a node is created,
    /// equivalent to calling [`set(expr, value)`](#method.set) and `name`
    /// will be the nodeset containing that single node.
    ///
    /// If a node was created, the method returns `true`, and `false` if no
    /// node was created.
    pub fn defnode(&mut self, name: &str, expr: &str, value: &str) -> Result<bool> {
        let name = CString::new(name)?;
        let expr = CString::new(expr)?;
        let value = CString::new(value)?;
        let mut cr : i32 = 0;

        unsafe { aug_defnode(self.ptr, name.as_ptr(), expr.as_ptr(),
                             value.as_ptr(), &mut cr) };
        self.make_result(cr == 1)
    }

    /// Load the tree from file. If a tree was already loaded, reload the
    /// tree, erasing any changes that may have been made since the last
    /// `load` or `save`.
    pub fn load(&mut self) -> Result<()> {
        unsafe { aug_load(self.ptr) };
        self.make_result(())
    }

    pub fn setm(&mut self, base: &str, sub: &str, value: &str) -> Result<(u32)> {
        let base = CString::new(base)?;
        let sub = CString::new(sub)?;
        let value = CString::new(value)?;

        let r = unsafe { aug_setm(self.ptr, base.as_ptr(), sub.as_ptr(),
                                  value.as_ptr()) };
        self.make_result(r as u32)
    }

    pub fn span(&self, path: &str) -> Result<Option<Span>> {
        let path = CString::new(path)?;
        let mut filename : *mut c_char = ptr::null_mut();
        let mut result = Span::new();

        unsafe {
            aug_span(self.ptr, path.as_ptr(), &mut filename,
                     &mut result.label.start, &mut result.label.end,
                     &mut result.value.start, &mut result.value.end,
                     &mut result.span.start, &mut result.span.end);
        }

        let err = unsafe { aug_error(self.ptr) };
        let err = ErrorCode::from_raw(err as _);
        if err != ErrorCode::NoError {
            if err == ErrorCode::NoSpan {
                return Ok(None);
            } else {
                return self.make_result(None);
            }
        }

        result.filename = ptr_to_string(filename);
        unsafe { libc::free(filename as *mut libc::c_void) };
        Ok(Some(result))
    }

    pub fn text_store(&mut self, lens: &str, node: &str, path: &str) -> Result<()> {
        let err_path = format!("/augeas/text{}/error", path);

        let lens = CString::new(lens)?;
        let node = CString::new(node)?;
        let path = CString::new(path)?;

        unsafe { aug_text_store(self.ptr, lens.as_ptr(), node.as_ptr(),
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

        unsafe { aug_text_retrieve(self.ptr, lens.as_ptr(),
                                   node_in.as_ptr(), path.as_ptr(),
                                   node_out.as_ptr()) };

        let err = self.get(&err_path)?;
        if let Some(kind) = err {
            return Err(Error::from(kind));
        }

        self.make_result(())
    }

    pub fn rename(&mut self, src: &str, lbl: &str) -> Result<()> {
        let src = CString::new(src)?;
        let lbl = CString::new(lbl)?;

        unsafe { aug_rename(self.ptr, src.as_ptr(), lbl.as_ptr()) };
        self.make_result(())
    }

    pub fn transform(&mut self, lens: &str, file: &str,
                     excl : bool) -> Result<()> {
        let lens = CString::new(lens)?;
        let file = CString::new(file)?;

        unsafe { aug_transform(self.ptr, lens.as_ptr(),
                               file.as_ptr(), excl as i32) };
        self.make_result(())
    }

    pub fn cp(&mut self, src: &str, dst: &str) -> Result<()> {
        let src = CString::new(src)?;
        let dst = CString::new(dst)?;

        unsafe { aug_cp(self.ptr, src.as_ptr(), dst.as_ptr()) };
        self.make_result(())
    }

    pub fn escape_name(&self, inp: &str) -> Result<Option<String>> {
        let inp = CString::new(inp)?;
        let mut out : *mut c_char = ptr::null_mut();

        unsafe { aug_escape_name(self.ptr, inp.as_ptr(), &mut out) };

        let s = ptr_to_string(out);
        unsafe { libc::free(out as *mut libc::c_void) };
        self.make_result(s)
    }

    pub fn load_file(&mut self, file: &str) -> Result<()> {
        let file = CString::new(file)?;

        unsafe { aug_load_file(self.ptr, file.as_ptr()) };
        self.make_result(())
    }

    pub fn source(&self, path: &str) -> Result<Option<String>> {
        let path = CString::new(path)?;
        let mut file_path : *mut c_char = ptr::null_mut();

        unsafe { aug_source(self.ptr, path.as_ptr(), &mut file_path) };
        let s = ptr_to_string(file_path);
        unsafe { libc::free(file_path as *mut libc::c_void) };
        self.make_result(s)
    }

    pub fn ns_attr(&self, var: &str, i: u32) -> Result<Attr> {
        let var = CString::new(var)?;

        let mut value : *const c_char = ptr::null_mut();
        let mut label : *const c_char = ptr::null_mut();
        let mut file_path : *mut c_char = ptr::null_mut();

        let rc = unsafe { aug_ns_attr(self.ptr, var.as_ptr(), i as c_int,
                                           &mut value, &mut label, &mut file_path) };
        if rc < 0 {
            return self.make_error()
        }

        let attr = Attr {
            label: ptr_to_string(label),
            value: ptr_to_string(value),
            file_path: ptr_to_string(file_path) };

        unsafe { libc::free(file_path as *mut libc::c_void) };

        self.make_result(attr)
    }

    pub fn ns_label(&self, var: &str, i: u32) -> Result<String> {
        let var = CString::new(var)?;

        let mut label : *const c_char = ptr::null_mut();

        let rc = unsafe { aug_ns_label(self.ptr, var.as_ptr(), i as c_int,
                          &mut label, ptr::null_mut() ) };
        if rc < 0 {
            return self.make_error()
        }

        match ptr_to_string(label) {
            Some(label) => Ok(label),
            None => Err(Error::from(ErrorCode::NoMatch))
        }
    }

    pub fn ns_index(&self, var: &str, i: u32) -> Result<u32> {
        let var = CString::new(var)?;

        let mut index : c_int = 0;

        unsafe { aug_ns_label(self.ptr, var.as_ptr(), i as c_int,
                 ptr::null_mut(), &mut index ) };
        return self.make_result(index as u32)
    }

    pub fn ns_value(&self, var: &str, i: u32) -> Result<Option<String>> {
        let var = CString::new(var)?;

        let mut value : *const c_char = ptr::null_mut();
        unsafe { aug_ns_value(self.ptr, var.as_ptr(), i as c_int,
                                   &mut value) };

        self.make_result(ptr_to_string(value))
    }

    pub fn ns_count(&self, var: &str) -> Result<u32> {
        let var = CString::new(var)?;

        let rc = unsafe { aug_ns_count(self.ptr, var.as_ptr()) };
        self.make_result(rc as u32)
    }

    pub fn ns_path(&self, var: &str, i: u32) -> Result<Option<String>> {
        let var = CString::new(var)?;

        let mut path : *mut c_char = ptr::null_mut();

        unsafe { aug_ns_path(self.ptr, var.as_ptr(), i as c_int, &mut path) };
        let p = ptr_to_string(path);
        unsafe { libc::free(path as *mut libc::c_void) };

        self.make_result(p)
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
    let count = aug.count("etc/passwd/*").unwrap();

    assert_eq!(9, users.len());
    assert_eq!(9, count);
    assert_eq!("/files/etc/passwd/root", users[0]);
    assert_eq!("/files/etc/passwd/nobody", users[8]);
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
    assert_eq!(0, aug.count("etc/passwd").unwrap());
    assert_eq!(1, aug.count("etc/other").unwrap());
}

#[test]
fn defvar_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    aug.defvar("x", "etc/passwd/*").unwrap();
    let n = aug.count("$x").unwrap();

    assert_eq!(9, n);
}

#[test]
fn defnode_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    let created = aug.defnode("y", "etc/notthere", "there").unwrap();
    assert!(created);

    let there = aug.get("$y").unwrap();
    assert_eq!("there", there.expect("failed to get etc/notthere"));

    let created = aug.defnode("z", "etc/passwd", "there").unwrap();
    assert!(! created);
}

#[test]
fn load_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    aug.set("etc/passwd/root/uid", "42").unwrap();
    aug.load().unwrap();
    let uid = aug.get("etc/passwd/root/uid").unwrap();
    assert_eq!("0", uid.expect("expected value for root/uid"));
}

#[test]
fn setm_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    let count = aug.setm("etc/passwd", "*/shell", "/bin/zsh").unwrap();
    assert_eq!(9, count);
}

#[test]
fn span_test() {
    let aug = Augeas::init("tests/test_root", "", Flags::EnableSpan).unwrap();

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
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

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
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    aug.rename("etc/passwd/root", "ruth").unwrap();

    let ruth = aug.get("etc/passwd/ruth/uid").unwrap().unwrap();
    assert_eq!("0", ruth);

    let root = aug.get("etc/passwd/root/uid").unwrap();
    assert!(root.is_none());
}

#[test]
fn transform_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    aug.transform("Hosts.lns", "/usr/local/etc/hosts", false).unwrap();
    let p = aug.get("/augeas/load/Hosts/incl[. = '/usr/local/etc/hosts']").unwrap();
    assert!(p.is_some());
}

#[test]
fn cp_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    aug.cp("etc/passwd/root", "etc/passwd/ruth").unwrap();
    let ruth = aug.get("etc/passwd/ruth/uid").unwrap().unwrap();
    assert_eq!("0", ruth);

    let root = aug.get("etc/passwd/root/uid").unwrap().unwrap();
    assert_eq!("0", root);
}

#[test]
fn escape_test() {
    let aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    // no escaping needed
    let n = aug.escape_name("foo");
    assert_eq!(Ok(None), n);

    let n = aug.escape_name("foo[");
    assert_eq!(Ok(Some(String::from("foo\\["))), n);
}

#[test]
fn load_file_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::NoLoad).unwrap();

    aug.load_file("/etc/passwd").unwrap();
    let root = aug.get("etc/passwd/root/uid").unwrap();
    assert!(root.is_some());

    let err = aug.load_file("/var/no/lens/for/this");
    assert!(err.is_err());
    let e = err.err().unwrap();
    assert!(e.is_code(ErrorCode::NoLens));
}

#[test]
fn source_test() {
    let aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    let s = aug.source("etc/passwd/root/uid").unwrap();
    // s should be Some("/files/etc/passwd") but Augeas versions before
    // 1.11 had a bug that makes the result always None
    assert!(s.is_none() || s.unwrap() == "/files/etc/passwd")
}

#[test]
fn ns_test() {
    let mut aug = Augeas::init("tests/test_root", "", Flags::None).unwrap();

    aug.defvar("x", "etc/passwd/*").unwrap();
    aug.defvar("uid", "etc/passwd/*/uid").unwrap();

    let attr = aug.ns_attr("x", 0).unwrap();
    assert_eq!("root", attr.label.unwrap());
    assert!(attr.value.is_none());
    assert_eq!("/files/etc/passwd", attr.file_path.unwrap());

    let attr = aug.ns_attr("x", 10000).unwrap_err();
    assert!(attr.is_code(ErrorCode::NoMatch));

    let attr = aug.ns_attr("y", 0).unwrap_err();
    assert!(attr.is_code(ErrorCode::NoMatch));

    let label = aug.ns_label("x", 0).unwrap();
    assert_eq!("root", label);

    let index = aug.ns_index("x", 4).unwrap();
    assert_eq!(0, index);

    let uid = aug.ns_value("uid", 2).unwrap().unwrap();
    assert_eq!("2", &uid);

    let count = aug.ns_count("uid").unwrap();
    assert_eq!(9, count);

    let path = aug.ns_path("uid", 0).unwrap().unwrap();
    assert_eq!("/files/etc/passwd/root/uid", path);
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
