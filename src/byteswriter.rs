//! A writer for efficiently building Python `bytes` objects.
//!
//! This type implements `std::io::Write` and `std::io::Seek`, allowing you to
//! write data to a buffer that can later be converted into a `PyBytes` object without
//! unnecessary copies.
//!
//! # Example
//! ```rust
//! use pyo3::prelude::*;
//! use pyo3::types::PyBytes;
//! use pyo3::bytes::PyBytesWriter;
//! use std::io::Write;
//!
//! Python::with_gil(|py| {
//!    let mut writer = PyBytesWriter::new(py).unwrap();
//!   writer.write_all(b"Hello, ").unwrap();
//!  writer.write_all(b"world!").unwrap();
//!  let py_bytes: &PyBytes = writer.into_pyobject(py).unwrap().as_ref(py);
//! assert_eq!(py_bytes.as_bytes(), b"Hello, world!");
//! });
//! ```
#[cfg(not(Py_LIMITED_API))]
use crate::{
    err::error_on_minusone,
    ffi::{
        self,
        compat::{
            PyBytesWriter_Create, PyBytesWriter_Discard, PyBytesWriter_Finish,
            PyBytesWriter_GetData, PyBytesWriter_GetSize, PyBytesWriter_Resize,
        },
    },
    ffi_ptr_ext::FfiPtrExt,
    py_result_ext::PyResultExt,
};
use crate::{types::PyBytes, Bound, IntoPyObject, PyErr, PyResult, Python};
use std::io::{IoSlice, SeekFrom};
#[cfg(not(Py_LIMITED_API))]
use std::{
    mem::ManuallyDrop,
    ptr::{self, NonNull},
};

/// A writer for efficiently building Python `bytes` objects.
///
/// This type implements `std::io::Write` and `std::io::Seek`, allowing you to
/// write data to a buffer that can later be converted into a `PyBytes` object without
/// unnecessary copies.
pub struct PyBytesWriter<'py> {
    python: Python<'py>,
    #[cfg(not(Py_LIMITED_API))]
    writer: NonNull<ffi::PyBytesWriter>,
    #[cfg(not(Py_LIMITED_API))]
    pos: usize,
    #[cfg(Py_LIMITED_API)]
    buffer: std::io::Cursor<Vec<u8>>,
}

impl<'py> PyBytesWriter<'py> {
    /// Create a new `PyBytesWriter` with a default initial capacity.
    #[inline]
    pub fn new(py: Python<'py>) -> PyResult<Self> {
        Self::with_capacity(py, 0)
    }

    /// Create a new `PyBytesWriter` with the specified initial capacity.
    #[inline]
    #[cfg_attr(Py_LIMITED_API, allow(clippy::unnecessary_wraps))]
    pub fn with_capacity(py: Python<'py>, capacity: usize) -> PyResult<Self> {
        #[cfg(not(Py_LIMITED_API))]
        {
            NonNull::new(unsafe { PyBytesWriter_Create(capacity as _) }).map_or_else(
                || Err(PyErr::fetch(py)),
                |writer| {
                    let mut writer = PyBytesWriter { python: py, writer, pos: 0};
                    // SAFETY: By setting the length to 0, we ensure no bytes are considered uninitialized.
                    unsafe { writer.set_len(0)? };
                    Ok(writer)
                },
            )
        }

        #[cfg(Py_LIMITED_API)]
        {
            Ok(PyBytesWriter {
                python: py,
                buffer: std::io::Cursor::new(Vec::with_capacity(capacity)),
            })
        }
    }

    /// Get the current length of the internal buffer.
    #[inline]
    fn len(&self) -> usize {
        #[cfg(not(Py_LIMITED_API))]
        unsafe {
            PyBytesWriter_GetSize(self.writer.as_ptr()) as _
        }

        #[cfg(Py_LIMITED_API)]
        {
            self.as_bytes().len()
        }
    }

    #[inline]
    #[cfg(not(Py_LIMITED_API))]
    fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { PyBytesWriter_GetData(self.writer.as_ptr()) as _ }
    }

    fn as_bytes(&self) -> &[u8] {
        #[cfg(not(Py_LIMITED_API))]
        // SAFETY: The buffer has valid data up to len().
        unsafe {
            std::slice::from_raw_parts(
                PyBytesWriter_GetData(self.writer.as_ptr()) as *const u8,
                self.len(),
            )
        }

        #[cfg(Py_LIMITED_API)]
        {
            &self.buffer.get_ref()
        }
    }

    fn as_mut_bytes(&mut self) -> &mut [u8] {
        #[cfg(not(Py_LIMITED_API))]
        // SAFETY: The buffer has valid data up to len().
        unsafe {
            std::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len())
        }

        #[cfg(Py_LIMITED_API)]
        {
            self.buffer.get_mut()
        }
    }

    /// Set the length of the internal buffer to `new_len`. The new bytes are uninitialized.
    ///
    /// # Safety
    /// The caller must ensure the new bytes are initialized.
    #[inline]
    #[cfg(not(Py_LIMITED_API))]
    unsafe fn set_len(&mut self, new_len: usize) -> PyResult<()> {
        unsafe {
            error_on_minusone(
                self.python,
                PyBytesWriter_Resize(self.writer.as_ptr(), new_len as _),
            )
        }
    }

    /// Reserve additional capacity and pad with zeros.
    ///
    /// # Safety
    /// The caller must ensure that the next `amount` bytes after pos will be written to.
    #[inline]
    #[cfg(not(Py_LIMITED_API))]
    unsafe fn reserve_and_pad(&mut self, amount: usize) -> PyResult<()> {
        let old_len = self.len();

        // Ensure enough capacity, so that pos + amount is valid.
        if self.pos + amount > old_len {
            // SAFETY: Caller upholds the safety contract.
            unsafe { self.set_len(self.pos + amount)? }
        }

        // pos is in unwritten area, so we need to pad with zeros until pos.
        if self.pos > old_len {
            // SAFETY: We have ensured enough capacity above.
            unsafe { ptr::write_bytes(self.as_mut_ptr().add(old_len), 0, self.pos - old_len) }
        }

        Ok(())
    }
}

impl<'py> TryFrom<PyBytesWriter<'py>> for Bound<'py, PyBytes> {
    type Error = PyErr;

    #[inline]
    fn try_from(value: PyBytesWriter<'py>) -> Result<Self, Self::Error> {
        let py = value.python;

        #[cfg(not(Py_LIMITED_API))]
        unsafe {
            PyBytesWriter_Finish(ManuallyDrop::new(value).writer.as_ptr())
                .assume_owned_or_err(py)
                .cast_into_unchecked()
        }

        #[cfg(Py_LIMITED_API)]
        {
            Ok(PyBytes::new(py, value.as_bytes()))
        }
    }
}

impl<'py> IntoPyObject<'py> for PyBytesWriter<'py> {
    type Target = PyBytes;
    type Output = Bound<'py, PyBytes>;
    type Error = PyErr;

    #[inline]
    fn into_pyobject(self, _py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.try_into()
    }
}

#[cfg(not(Py_LIMITED_API))]
impl<'py> Drop for PyBytesWriter<'py> {
    #[inline]
    fn drop(&mut self) {
        unsafe { PyBytesWriter_Discard(self.writer.as_ptr()) }
    }
}

#[cfg(not(Py_LIMITED_API))]
impl std::io::Write for PyBytesWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> std::io::Result<usize> {
        let len = bufs.iter().map(|b| b.len()).sum();

        // SAFETY: We write the new uninitialized bytes below.
        unsafe { self.reserve_and_pad(len)?; }

        // SAFETY: We ensure enough capacity above.
        let mut pos = unsafe { self.as_mut_ptr().add(self.pos) };
        for buf in bufs {
            // SAFETY: We have ensured enough capacity above.
            unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), pos, buf.len()) };

            // SAFETY: We just wrote buf.len() bytes
            pos = unsafe { pos.add(buf.len()) };
        }
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let len = buf.len();

        // SAFETY: We write the new uninitialized bytes below.
        unsafe { self.reserve_and_pad(len)?; }

        // SAFETY: We have ensured enough capacity above.
        unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), self.as_mut_ptr().add(self.pos), len) };

        self.pos += len;
        Ok(())
    }
}

#[cfg(Py_LIMITED_API)]
impl std::io::Write for PyBytesWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> std::io::Result<usize> {
        self.buffer.write_vectored(bufs)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.buffer.write_all(buf)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.buffer.write_fmt(args)
    }
}

impl std::io::Seek for PyBytesWriter<'_> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        #[cfg(Py_LIMITED_API)]
        {
            self.buffer.seek(pos)
        }

        #[cfg(not(Py_LIMITED_API))]
        {
            let new_pos: usize = match pos {
                SeekFrom::Start(offset) => offset as i64,
                SeekFrom::End(offset) => self.len() as i64 - offset,
                SeekFrom::Current(offset) => self.pos as i64 + offset,
            }
            .try_into()
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid seek position")
            })?;

            // if new_pos > self.len() {
            //     self.resize(new_pos, 0)?;
            // }

            self.pos = new_pos;
            Ok(self.pos as u64)
        }
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        #[cfg(Py_LIMITED_API)]
        {
            self.buffer.rewind()
        }

        #[cfg(not(Py_LIMITED_API))]
        {
            self.pos = 0;
            Ok(())
        }
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        #[cfg(Py_LIMITED_API)]
        {
            self.buffer.stream_position()
        }

        #[cfg(not(Py_LIMITED_API))]
        {
            Ok(self.pos as u64)
        }
    }

    #[cfg(Py_LIMITED_API)]
    fn seek_relative(&mut self, offset: i64) -> std::io::Result<()> {
        self.buffer.seek_relative(offset)
    }
}

impl AsRef<[u8]> for PyBytesWriter<'_> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsMut<[u8]> for PyBytesWriter<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_mut_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PyBytesMethods;
    use std::io::{Seek, Write};

    #[test]
    fn test_io_write() {
        Python::attach(|py| {
            let buf = b"hallo world";
            let mut writer = PyBytesWriter::new(py).unwrap();
            assert_eq!(writer.write(buf).unwrap(), 11);
            let bytes: Bound<'_, PyBytes> = writer.try_into().unwrap();
            assert_eq!(bytes.as_bytes(), buf);
        })
    }

    #[test]
    fn test_pre_allocated() {
        Python::attach(|py| {
            let buf = b"hallo world";
            let mut writer = PyBytesWriter::with_capacity(py, buf.len()).unwrap();
            assert_eq!(writer.len(), 0, "Writer position should be zero");
            assert_eq!(writer.write(buf).unwrap(), 11);
            let bytes: Bound<'_, PyBytes> = writer.try_into().unwrap();
            assert_eq!(bytes.as_bytes(), buf);
        })
    }

    #[test]
    fn test_io_write_vectored() {
        Python::attach(|py| {
            let bufs = [IoSlice::new(b"hallo "), IoSlice::new(b"world")];
            let mut writer = PyBytesWriter::new(py).unwrap();
            assert_eq!(writer.write_vectored(&bufs).unwrap(), 11);
            let bytes: Bound<'_, PyBytes> = writer.try_into().unwrap();
            assert_eq!(bytes.as_bytes(), b"hallo world");
        })
    }

    #[test]
    fn test_large_data() {
        Python::attach(|py| {
            let mut writer = PyBytesWriter::new(py).unwrap();
            let large_data = vec![0; 1024]; // 1 KB
            writer.write_all(&large_data).unwrap();
            let bytes: Bound<'_, PyBytes> = writer.try_into().unwrap();
            assert_eq!(bytes.as_bytes(), large_data.as_slice());
        })
    }

    #[test]
    fn test_seek() {
        Python::attach(|py| {
            let mut writer = PyBytesWriter::new(py).unwrap();
            writer.write_all(b"hello").unwrap();
            writer.seek_relative(1).unwrap();
            assert_eq!(writer.stream_position().unwrap(), 6);
            assert_eq!(writer.as_bytes(), b"hello", "Seeking past end should not change data");
            assert_eq!(writer.len(), 5, "Length should remain unchanged after seeking past end");
            writer.write_all(b"world").unwrap();
            assert_eq!(writer.as_bytes(), b"hello\0world", "unwritten bytes should be zeroed initialized");
            writer.rewind().unwrap();
            writer.write_all(b"hallo ").unwrap();
            let bytes: Bound<'_, PyBytes> = writer.try_into().unwrap();
            assert_eq!(bytes.as_bytes(), b"hallo world");
        })
    }
}
