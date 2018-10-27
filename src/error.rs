use std::fmt;
use std::ffi::NulError;
use ::Augeas;
use ::util::ptr_to_string;
use augeas_sys::*;

#[derive(Clone,PartialEq,Debug)]
pub enum Error {
    Augeas(AugeasError),
    Parse(ParseError),
    Nul(NulError)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Augeas(ref err) => err.fmt(f),
            Error::Nul(ref err) => err.fmt(f),
            Error::Parse(ref err) => err.fmt(f)
        }
    }
}

#[derive(Clone,PartialEq,Eq,Debug,Default)]
pub struct AugeasError {
    pub code          : ErrorCode,
    pub message       : Option<String>,
    pub minor_message : Option<String>,
    pub details       : Option<String>
}

impl AugeasError {
    pub fn new_no_mem(message: impl Into<String>) -> AugeasError {
        AugeasError {
            code : ErrorCode::NoMem,
            message : Some(message.into()),
            .. Default::default()
        }
    }
}

impl ::std::error::Error for AugeasError {
    fn description(&self) -> &str {
        match self.message {
            None => "No description",
            Some(ref s) => s
        }
    }
}

fn maybe_write(f: &mut fmt::Formatter, opt: &Option<String>) -> fmt::Result {
    match *opt {
        Some(ref s) => write!(f, "      {}\n", s),
        None => Ok(())
    }
}

impl fmt::Display for AugeasError {
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

impl Error {
    pub fn is_code(&self, code : ErrorCode) -> bool {
        match self {
            Error::Augeas(err) => err.code == code,
            _ => false
        }
    }
}

impl From<NulError> for Error {
    fn from(err: NulError) -> Error {
        Error::Nul(err)
    }
}

impl <'a> From<&'a Augeas> for Error {
    fn from(aug: &'a Augeas) -> Error {
        let err = unsafe { aug_error(aug.ptr) };
        let err = ErrorCode::from_raw(err as _);
        let msg = unsafe { ptr_to_string(aug_error_message(aug.ptr)) };
        let mmsg = unsafe { ptr_to_string(aug_error_minor_message(aug.ptr)) };
        let det = unsafe { ptr_to_string(aug_error_details(aug.ptr)) };
        Error::Augeas(AugeasError {
            code: err,
            message: msg,
            minor_message: mmsg,
            details: det
       })
    }
}

#[repr(C)]
#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub enum ErrorCode {
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
    Unknown
}

impl ErrorCode {
    #[allow(non_upper_case_globals)]
    pub fn from_raw(code: aug_errcode_t) -> ErrorCode {
        match code {
            aug_errcode_t_AUG_NOERROR => ErrorCode::NoError,
            aug_errcode_t_AUG_ENOMEM => ErrorCode::NoMem,
            aug_errcode_t_AUG_EINTERNAL => ErrorCode::Internal,
            aug_errcode_t_AUG_EPATHX => ErrorCode::PathExpr,
            aug_errcode_t_AUG_ENOMATCH => ErrorCode::NoMatch,
            aug_errcode_t_AUG_EMMATCH => ErrorCode::ManyMatches,
            aug_errcode_t_AUG_ESYNTAX => ErrorCode::Syntax,
            aug_errcode_t_AUG_ENOLENS => ErrorCode::NoLens,
            aug_errcode_t_AUG_EMXFM => ErrorCode::MultipleTransforms,
            aug_errcode_t_AUG_ENOSPAN => ErrorCode::NoSpan,
            aug_errcode_t_AUG_EMVDESC => ErrorCode::MoveDescendant,
            aug_errcode_t_AUG_ECMDRUN => ErrorCode::CMDRun,
            aug_errcode_t_AUG_EBADARG => ErrorCode::BadArg,
            aug_errcode_t_AUG_ELABEL => ErrorCode::Label,
            aug_errcode_t_AUG_ECPDESC => ErrorCode::CopyDescendant,
            _ => ErrorCode::Unknown,
        }
    }
}

impl Default for ErrorCode {
    fn default() -> ErrorCode { ErrorCode::NoError }
}

impl From<ErrorCode> for Error {
    fn from(code : ErrorCode) -> Error {
        Error::Augeas(AugeasError {
            code : code,
            message : None,
            minor_message : None,
            details : None
        })
    }
}

impl From<String> for Error {
    fn from(kind: String) -> Error {
        Error::Parse(ParseError {
            kind: kind
        })
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
        write!(f, "parse error of kind {}", self.kind)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ParseError {
    // There's a lot more information we can/should pull out of the
    // tree when parsing goes wrong
    pub kind : String
}
