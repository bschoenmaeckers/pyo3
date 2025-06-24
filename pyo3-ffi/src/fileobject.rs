use crate::object::PyObject;
use std::os::raw::{c_char, c_int};

pub const PY_STDIOTEXTMODE: &str = "b";

extern "C" {
    pub fn PyFile_FromFd(
        fd: c_int,
        name: *const c_char,
        mode: *const c_char,
        buffering: c_int,
        encoding: *const c_char,
        errors: *const c_char,
        newline: *const c_char,
        closefd: c_int,
    ) -> *mut PyObject;
    #[cfg_attr(PyPy, link_name = "PyPyFile_GetLine")]
    pub fn PyFile_GetLine(arg1: *mut PyObject, arg2: c_int) -> *mut PyObject;
    #[cfg_attr(PyPy, link_name = "PyPyFile_WriteObject")]
    pub fn PyFile_WriteObject(arg1: *mut PyObject, arg2: *mut PyObject, arg3: c_int) -> c_int;
    #[cfg_attr(PyPy, link_name = "PyPyFile_WriteString")]
    pub fn PyFile_WriteString(arg1: *const c_char, arg2: *mut PyObject) -> c_int;
    #[cfg_attr(PyPy, link_name = "PyPyFile_AsFileDescriptor")]
    pub fn PyObject_AsFileDescriptor(arg1: *mut PyObject) -> c_int;
}

#[cfg_attr(windows, link(name = "pythonXY"))]
extern "C" {
    #[deprecated(note = "Python 3.12")]
    pub static mut Py_FileSystemDefaultEncoding: *const c_char;
    #[deprecated(note = "Python 3.12")]
    pub static mut Py_FileSystemDefaultEncodeErrors: *const c_char;
    #[deprecated(note = "Python 3.12")]
    pub static mut Py_HasFileSystemDefaultEncoding: c_int;
    // skipped 3.12-deprecated Py_UTF8Mode
}

// skipped _PyIsSelectable_fd
