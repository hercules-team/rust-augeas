//! See [augeas.h](https://github.com/hercules-team/augeas/blob/release-1.3.0/src/augeas.h) for details
extern crate libc;

use self::libc::{c_void, c_char, c_int, c_uint, FILE};


#[repr(C)]
#[derive(Copy,Clone)]
pub enum AugFlag {
    None = 0,
    SaveBackup = 1 << 0,
    SaveNewFile = 1 << 1,
    TypeCheck = 1 << 2,
    NoStdinc  = 1 << 3,
    SaveNoop = 1 << 4,
    NoLoad = 1 << 5,
    NoModlAutoload = 1 << 6,
    EnableSpan = 1 << 7,
    NoErrClose = 1 << 8,
    TraceModuleLoading = 1 << 9
}

impl std::ops::BitOr for AugFlag {
    type Output = c_uint;
    fn bitor(self, other: AugFlag) -> c_uint {
        self as c_uint | other as c_uint
    }
}

#[repr(C)]
#[derive(Copy,Clone,PartialEq,Debug)]
pub enum AugError {
    NoError,
    NoMem,
    Internal,
    PathExpr,
    NoMatch,
    ManyMatches,
    Syntax,
    NoLens,
    MultipleTransforms,
    NoSpan,
    MoveDescendant,
    CMDRun,
    BadArg,
    Label,
    CopyDescendant,
    // Not an error returned from Augeas but one we need to take care of as
    // part of the bindings. To make sure we don't change the code for this
    // as Augeas introduces new errors, move this to a high integer
    NulString = 4096
}

impl Default for AugError {
    fn default() -> AugError { AugError::NoError }
}

/// Opaque augeas type
#[allow(non_camel_case_types)]
pub type augeas_t = *mut c_void;
/// Opaque xmlNode type
#[allow(non_camel_case_types)]
pub enum xmlNode {}

#[link(name = "augeas")]
extern {
    pub fn aug_init(root: *const c_char, loadpath: *const c_char, flags: c_uint) -> augeas_t;
    pub fn aug_defvar(aug: augeas_t, name: *const c_char, expr: *const c_char) -> c_int;
    pub fn aug_defnode(aug: augeas_t, name: *const c_char, expr: *const c_char, value: *const c_char, created: *mut c_int) -> c_int;
    pub fn aug_get(aug: augeas_t, path: *const c_char, value: *mut *mut c_char) -> c_int;
    pub fn aug_label(aug: augeas_t, path: *const c_char, label: *mut *const c_char) -> c_int;
    pub fn aug_set(aug: augeas_t, path: *const c_char, value: *const c_char) -> c_int;
    pub fn aug_setm(aug: augeas_t, base: *const c_char, sub: *const c_char, value: *const c_char) -> c_int;
    pub fn aug_span(aug: augeas_t, path: *const c_char, filename: *mut *mut c_char,
        label_start: *mut c_uint, label_end: *mut c_uint,
        value_start: *mut c_uint, value_end: *mut c_uint,
        span_start: *mut c_uint, span_end: *mut c_uint
    ) -> c_int;
    pub fn aug_insert(aug: augeas_t, path: *const c_char, label: *const c_char, before: c_int) -> c_int;
    pub fn aug_rm(aug: augeas_t, path: *const c_char) -> c_int;
    pub fn aug_mv(aug: augeas_t, src: *const c_char, dst: *const c_char) -> c_int;
    pub fn aug_cp(aug: augeas_t, src: *const c_char, dst: *const c_char) -> c_int;
    pub fn aug_rename(aug: augeas_t, src: *const c_char, lbl: *const c_char) -> c_int;
    pub fn aug_match(aug: augeas_t, path: *const c_char, matches: *mut *mut *mut c_char ) -> c_int;
    pub fn aug_save(aug: augeas_t) -> c_int;
    pub fn aug_load(aug: augeas_t) -> c_int;
    pub fn aug_text_store(aug: augeas_t, lens: *const c_char,
        node: *const c_char, path: *const c_char) -> c_int;
    pub fn aug_text_retrieve(aug: augeas_t, lens: *const c_char,
        node_in: *const c_char, path: *const c_char,
        node_out: *const c_char) -> c_int;
    pub fn aug_print(out: *mut FILE, path: *const c_char) -> c_int;
    pub fn aug_to_xml(aug: augeas_t, path: *const c_char, xmldoc: *mut *mut xmlNode,
        flags: c_uint) -> c_int;
    pub fn aug_transform(aug: augeas_t, lens: *const c_char, file: *const c_char, excl: c_int) -> c_int;
    pub fn aug_srun(aug: augeas_t, out: *mut FILE, text: *const c_char) -> c_int;
    pub fn aug_close(aug: augeas_t);
    pub fn aug_error(aug: augeas_t) -> AugError;
    pub fn aug_error_message(aug: augeas_t) -> *const c_char;
    pub fn aug_error_minor_message(aug: augeas_t) -> *const c_char;
    pub fn aug_error_details(aug: augeas_t) -> *const c_char;
}
