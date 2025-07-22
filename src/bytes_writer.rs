use crate::ffi::compat::{
    PyBytesWriter_Create, PyBytesWriter_Discard, PyBytesWriter_Finish, PyBytesWriter_GetData,
    PyBytesWriter_GetSize, PyBytesWriter_Resize, PyBytesWriter_WriteBytes,
    _PyBytesWriter_GetAllocated, _PyBytesWriter_Resize_impl,
};
use crate::ffi_ptr_ext::FfiPtrExt;
use crate::py_result_ext::PyResultExt;
use crate::types::PyBytes;
use crate::{ffi, Bound, IntoPyObject, PyErr, PyResult, Python};
use std::io;
use std::io::IoSlice;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;

pub struct PyBytesWriter {
    writer: NonNull<ffi::PyBytesWriter>,
}

impl PyBytesWriter {
    pub fn new(py: Python<'_>) -> PyResult<Self> {
        match NonNull::new(unsafe { PyBytesWriter_Create(0) }) {
            Some(ptr) => Ok(PyBytesWriter { writer: ptr }),
            None => Err(PyErr::fetch(py)),
        }
    }

    /// Creates a new `PyUnicodeWriter` with the specified initial capacity.
    pub fn with_capacity(py: Python<'_>, capacity: usize) -> PyResult<Self> {
        let mut writer = Self::new(py)?;
        writer.reserve(capacity)?;
        Ok(writer)
    }

    pub fn reserve(&mut self, additional: usize) -> PyResult<()> {
        let size = self.len() + additional;
        let result =
            unsafe { _PyBytesWriter_Resize_impl(self.writer.as_ptr(), size as ffi::Py_ssize_t, 1) };

        if result < 0 {
            return Python::attach(|py| Err(PyErr::fetch(py)));
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        unsafe { PyBytesWriter_GetSize(self.writer.as_ptr()) as usize }
    }

    pub fn capacity(&self) -> usize {
        unsafe { _PyBytesWriter_GetAllocated(self.writer.as_ptr()) as usize }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            let ptr = PyBytesWriter_GetData(self.writer.as_ptr());
            std::slice::from_raw_parts_mut(ptr.cast(), self.len())
        }
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe {
            let ptr = PyBytesWriter_GetData(self.writer.as_ptr()).add(self.len());
            std::slice::from_raw_parts_mut(ptr.cast(), self.capacity() - self.len())
        }
    }

    pub unsafe fn set_len(&mut self, new_len: usize) -> PyResult<()> {
        debug_assert!(new_len <= self.capacity());

        let result =
            unsafe { PyBytesWriter_Resize(self.writer.as_ptr(), new_len as ffi::Py_ssize_t) };

        if result < 0 {
            return Python::attach(|py| Err(PyErr::fetch(py)));
        }
        Ok(())
    }
}

impl<'py> IntoPyObject<'py> for PyBytesWriter {
    type Target = PyBytes;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        unsafe {
            PyBytesWriter_Finish(ManuallyDrop::new(self).writer.as_ptr())
                .assume_owned_or_err(py)
                .downcast_into_unchecked()
        }
    }
}

impl Drop for PyBytesWriter {
    fn drop(&mut self) {
        unsafe {
            PyBytesWriter_Discard(self.writer.as_ptr());
        }
    }
}

impl io::Write for PyBytesWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = unsafe {
            PyBytesWriter_WriteBytes(
                self.writer.as_ptr(),
                buf.as_ptr().cast(),
                buf.len() as ffi::Py_ssize_t,
            )
        };
        if result < 0 {
            return Python::attach(|py| Err(PyErr::fetch(py).into()));
        }
        Ok(buf.len())
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let size = bufs.iter().map(|b| b.len()).sum::<usize>();
        self.reserve(size)?;
        for buf in bufs {
            self.write(buf)?;
        }
        Ok(size)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
