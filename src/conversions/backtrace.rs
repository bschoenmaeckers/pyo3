#![cfg(feature = "backtrace")]

use crate::types::PyFrame;
use crate::{Bound, IntoPyObject, PyErr, Python};
use std::ffi::CString;

impl<'py> IntoPyObject<'py> for btparse::Frame {
    type Target = PyFrame;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        (&self).into_pyobject(py)
    }
}

impl<'py> IntoPyObject<'py> for &btparse::Frame {
    type Target = PyFrame;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let file_name = self.file.as_ref().map_or_else(
            || CString::new("<unknown>"),
            |line| CString::new(line.as_str()),
        )?;

        let func_name = CString::new(self.function.as_str())?;
        let line_number = self.line.unwrap_or_default().try_into()?;

        PyFrame::new(py, &file_name, &func_name, line_number)
    }
}
