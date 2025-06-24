use crate::err::error_on_minusone;
use crate::exceptions::PyOSError;
use crate::ffi_ptr_ext::FfiPtrExt;
use crate::types::PyAnyMethods;
use crate::{ffi, Bound, FromPyObject, IntoPyObject, PyAny, PyErr, PyResult, Python};
use pyo3_ffi::c_str;
use std::fs::File;
#[cfg(unix)]
use std::os::unix::prelude::*;
#[cfg(windows)]
use std::os::windows::prelude::*;

#[cfg(windows)]
impl FromPyObject<'_> for OwnedHandle {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        let fd = unsafe { ffi::PyObject_AsFileDescriptor(ob.as_ptr()) };
        error_on_minusone(ob.py(), fd)?;

        let raw_handle = unsafe { libc::get_osfhandle(fd) };
        if raw_handle == -1 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(unsafe { BorrowedHandle::borrow_raw(raw_handle as _) }
            .try_clone_to_owned()?
            .into())
    }
}

#[cfg(windows)]
impl<'py> IntoPyObject<'py> for OwnedHandle {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let fd = unsafe {
            let raw_handle = self.into_raw_handle();
            libc::open_osfhandle(raw_handle as _, 0)
        };

        if fd < 0 {
            return Err(PyOSError::new_err("Cannot convert File to file descriptor"));
        }

        // We cannot determine the mode of the file descriptor in a portable way on Windows,
        // so we default to "rb+" mode, which allows reading and writing.
        let mode = c_str!("r+b");

        unsafe {
            ffi::PyFile_FromFd(
                fd,
                std::ptr::null(),
                mode.as_ptr(),
                -1,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
            .assume_owned_or_err(py)
        }
    }
}

#[cfg(unix)]
impl FromPyObject<'_> for OwnedFd {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        let fd = unsafe { ffi::PyObject_AsFileDescriptor(ob.as_ptr()) };
        error_on_minusone(ob.py(), fd)?;
        Ok(unsafe { BorrowedFd::borrow_raw(fd) }.try_clone_to_owned()?)
    }
}

#[cfg(unix)]
impl<'py> IntoPyObject<'py> for OwnedFd {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let fd = self.into_raw_fd();
        if fd < 0 {
            return Err(PyOSError::new_err("Cannot convert File to file descriptor"));
        }

        let mode = {
            let flags: i32 = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 {
                return Err(std::io::Error::last_os_error().into());
            }

            let appended = flags & libc::O_APPEND > 0;
            let created_exclusive = flags & libc::O_CREAT > 0 && flags & libc::O_EXCL > 0;
            let (readable, writable) = match flags & libc::O_ACCMODE {
                libc::O_RDONLY => (true, false),
                libc::O_WRONLY => (false, true),
                libc::O_RDWR => (true, true),
                libc::O_ACCMODE.. => unreachable!(),
            };

            debug_assert!(!appended || writable, "appended files must be writable");
            debug_assert!(
                !created_exclusive || writable,
                "created files must be writable"
            );

            if created_exclusive {
                if readable {
                    c_str!("xb+")
                } else {
                    c_str!("xb")
                }
            } else if appended {
                if readable {
                    c_str!("ab+")
                } else {
                    c_str!("ab")
                }
            } else if readable {
                if writable {
                    c_str!("rb+")
                } else {
                    c_str!("rb")
                }
            } else {
                c_str!("wb")
            }
        };

        unsafe {
            ffi::PyFile_FromFd(
                fd,
                std::ptr::null(),
                mode.as_ptr(),
                -1,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
            .assume_owned_or_err(py)
        }
    }
}

impl FromPyObject<'_> for File {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        #[cfg(unix)]
        {
            Ok(ob.extract::<OwnedFd>()?.into())
        }

        #[cfg(windows)]
        {
            Ok(ob.extract::<OwnedHandle>()?.into())
        }
    }
}

impl<'py> IntoPyObject<'py> for File {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        #[cfg(unix)]
        {
            OwnedFd::from(self).into_pyobject(py)
        }

        #[cfg(windows)]
        {
            OwnedHandle::from(self).into_pyobject(py)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use super::*;
    use crate::exceptions::PyTypeError;
    use crate::types::{PyAnyMethods, PyNone};
    use crate::Python;
    use std::io::{Read, Write};

    fn with_py_file(f: impl FnOnce(&Bound<'_, PyAny>)) {
        Python::with_gil(|py| {
            let temp_file = tempfile::NamedTempFile::new().unwrap();
            let path = temp_file.path().to_string_lossy().replace("\\", "\\\\");
            let code = CString::new(format!("open('{}', 'r')", path)).unwrap();
            let py_file = py
                .eval(&code, None, None)
                .unwrap();
            f(&py_file);
            py_file.call_method0("close").unwrap();
        });
    }

    #[test]
    fn test_not_a_file() {
        Python::with_gil(|py| {
            let none = PyNone::get(py);
            let error = none.extract::<File>().unwrap_err();
            assert!(error.is_instance_of::<PyTypeError>(py));
        })
    }

    #[test]
    fn test_writing_read_only_pyfile() {
        with_py_file(|py_file| {
            let mut file = py_file.extract::<File>().unwrap();
            assert!(
                file.write("some data".as_bytes()).is_err(),
                "you should not be able to write to a read-only file"
            );
        })
    }

    #[test]
    fn test_writing_read_only_rustfile() {
        Python::with_gil(|py| {
            let file = File::options().read(true).write(false).open("cargo.toml").unwrap();
            let py_file = file.into_pyobject(py).unwrap();
            #[cfg(not(windows))]
            assert!(
                !py_file.call_method0("writable").unwrap().extract::<bool>().unwrap(),
                "file should not be advertised as writable"
            );
            let write_failed = py_file.call_method1("write", (b"some data",)).is_err();
                // || py_file.call_method0("flush").is_err();
            py_file.call_method0("flush").unwrap();
            assert!(write_failed, "you should not be able to write to a read-only file");
            py_file.call_method0("close").unwrap();
        })
    }

    #[test]
    fn test_dropping_file() {
        with_py_file(|py_file| {
            let file = py_file.extract::<File>().unwrap();
            assert!(file.metadata().is_ok());
            drop(file);
            assert!(
                !py_file
                    .getattr("closed")
                    .unwrap()
                    .extract::<bool>()
                    .unwrap(),
                "python file should still be open after dropping in rust"
            );
            assert!(
                py_file.call_method0("read").is_ok(),
                "file is still readable"
            );
        })
    }

    #[test]
    fn test_into_pyobject() {
        Python::with_gil(|py| {
            let file = tempfile::tempfile().unwrap();
            let py_file: Bound<'_, PyAny> = file.into_pyobject(py).unwrap();
            py_file.call_method0("read").unwrap();
            py_file.call_method0("close").unwrap();
        })
    }
}
