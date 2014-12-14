extern crate libc;

use self::libc::{c_void, c_char, c_int, c_uint};

#[allow(non_camel_case_types)]
pub type augeas = *mut c_void;

// https://github.com/hercules-team/augeas/blob/master/src/augeas.h
#[repr(C)]
#[deriving(Copy)]
pub enum AugFlags {
    None = 0,
    /// Keep the original file with a .augsave extension
    SaveBackup = (1 << 0),
    /// Save changes into a file with
    /// extension .augnew, and do not
    /// overwrite the original file. Takes
    /// precedence over SaveBackup
    SaveNewfile = (1 << 1),
    /// Typecheck lenses; since it can be very
    /// expensive it is not done by default
    TypeCheck = (1 << 2),
    /// Do not use the builtin load path for modules
    NoStdinc = (1 << 3),
    /// Make save a no-op process, just record what would have changed
    SaveNoop = (1 << 4),
    /// Do not load the tree from aug_init
    NoLoad = (1 << 5),
    NoModlAutoload = (1 << 6),
    /// Track the span in the input of nodes
    EnableSpan = (1 << 7),
    /// Do not close automatically when
    /// encountering error during aug_init
    NoErrClose = (1 << 8),
    /// For use by augparse -t
    TraceModuleLoading = (1 << 9)
}

#[link(name = "augeas")]
extern {
    pub fn aug_init(root: *const c_char, loadpath: *const c_char, flags: c_uint) -> augeas;
    pub fn aug_get(aug: augeas, path: *const c_char, value: *mut *const c_char) -> c_int;
    pub fn aug_label(aug: augeas, path: *const c_char, label: *mut *const c_char) -> c_int;
    pub fn aug_close(aug: augeas);
    pub fn aug_save(aug: augeas) -> c_int;
    pub fn aug_match(aug: augeas, path: *const c_char, matches: *mut *mut *mut c_char ) -> c_int;
}
