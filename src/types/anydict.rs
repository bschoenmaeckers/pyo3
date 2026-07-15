use crate::err::{PyErr, PyResult};
use crate::ffi_ptr_ext::FfiPtrExt;
use crate::instance::{Borrowed, Bound, BoundObject};
use crate::type_object::{PyTypeCheck, PyTypeInfo};
#[cfg(Py_3_15)]
use crate::types::frozendict::PyFrozenDict;
#[cfg(Py_3_15)]
use crate::types::tuple::PyTuple;
use crate::types::{PyAny, PyDict, PyList, PyMapping};
use crate::{ffi, IntoPyObject, IntoPyObjectExt, Python};

/// Represents either a Python `dict` or `frozendict`.
///
/// Values of this type are accessed via PyO3's smart pointers, e.g. as
/// [`Py<PyAnyDict>`][crate::Py] or [`Bound<'py, PyAnyDict>`][Bound].
///
/// For APIs available on both dictionary types, see the [`PyAnyDictMethods`] trait.
#[repr(transparent)]
pub struct PyAnyDict(PyAny);

pyobject_native_type_named!(PyAnyDict);

unsafe impl PyTypeCheck for PyAnyDict {
    #[cfg(feature = "experimental-inspect")]
    const TYPE_HINT: crate::inspect::PyStaticExpr = {
        #[cfg(Py_3_15)]
        {
            crate::inspect::type_hint_union!(
                <PyDict as PyTypeInfo>::TYPE_HINT,
                <PyFrozenDict as PyTypeInfo>::TYPE_HINT
            )
        }
        #[cfg(not(Py_3_15))]
        {
            <PyDict as PyTypeInfo>::TYPE_HINT
        }
    };

    #[inline]
    fn type_check(object: &Bound<'_, PyAny>) -> bool {
        #[cfg(Py_3_15)]
        {
            unsafe {
                ffi::PyDict_Check(object.as_ptr()) > 0
                    || ffi::PyFrozenDict_Check(object.as_ptr()) > 0
            }
        }

        #[cfg(not(Py_3_15))]
        {
            unsafe { ffi::PyDict_Check(object.as_ptr()) > 0 }
        }
    }

    fn classinfo_object(py: Python<'_>) -> Bound<'_, PyAny> {
        #[cfg(Py_3_15)]
        {
            PyTuple::new(
                py,
                [
                    PyDict::type_object(py).into_any(),
                    PyFrozenDict::type_object(py).into_any(),
                ],
            )
            .expect("PyAnyDict classinfo tuple construction should not fail")
            .into_any()
        }

        #[cfg(not(Py_3_15))]
        {
            PyDict::type_object(py).into_any()
        }
    }
}

impl<'py> From<Bound<'py, PyAnyDict>> for Bound<'py, PyMapping> {
    fn from(dict: Bound<'py, PyAnyDict>) -> Self {
        dict.into_mapping()
    }
}

impl<'py> AsRef<Bound<'py, PyMapping>> for Bound<'py, PyAnyDict> {
    fn as_ref(&self) -> &Bound<'py, PyMapping> {
        self.as_mapping()
    }
}

/// Implementation of functionality shared by Python `dict` and `frozendict` objects.
#[doc(alias = "PyAnyDict")]
pub trait PyAnyDictMethods<'py>: crate::sealed::Sealed {
    /// Return the number of items in the dictionary.
    ///
    /// This is equivalent to the Python expression `len(self)`.
    fn len(&self) -> usize;

    /// Checks if the dictionary is empty, i.e. `len(self) == 0`.
    fn is_empty(&self) -> bool;

    /// Determines if the dictionary contains the specified key.
    ///
    /// This is equivalent to the Python expression `key in self`.
    fn contains<K>(&self, key: K) -> PyResult<bool>
    where
        K: IntoPyObject<'py>;

    /// Gets an item from the dictionary.
    ///
    /// Returns `None` if the item is not present, or if an error occurs.
    ///
    /// To get a `KeyError` for non-existing keys, use `PyAny::get_item`.
    fn get_item<K>(&self, key: K) -> PyResult<Option<Bound<'py, PyAny>>>
    where
        K: IntoPyObject<'py>;

    /// Returns a list of dictionary keys.
    ///
    /// This is equivalent to the Python expression `list(self.keys())`.
    fn keys(&self) -> Bound<'py, PyList>;

    /// Returns a list of dictionary values.
    ///
    /// This is equivalent to the Python expression `list(self.values())`.
    fn values(&self) -> Bound<'py, PyList>;

    /// Returns a list of dictionary items.
    ///
    /// This is equivalent to the Python expression `list(self.items())`.
    fn items(&self) -> Bound<'py, PyList>;

    /// Returns `self` cast as a `PyMapping`.
    fn as_mapping(&self) -> &Bound<'py, PyMapping>;

    /// Returns `self` cast as a `PyMapping`.
    fn into_mapping(self) -> Bound<'py, PyMapping>;
}

impl<'py> PyAnyDictMethods<'py> for Bound<'py, PyAnyDict> {
    #[inline]
    fn len(&self) -> usize {
        unsafe { ffi::PyDict_Size(self.as_ptr()) as usize }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn contains<K>(&self, key: K) -> PyResult<bool>
    where
        K: IntoPyObject<'py>,
    {
        fn inner(dict: &Bound<'_, PyAnyDict>, key: Borrowed<'_, '_, PyAny>) -> PyResult<bool> {
            match unsafe { ffi::PyDict_Contains(dict.as_ptr(), key.as_ptr()) } {
                1 => Ok(true),
                0 => Ok(false),
                _ => Err(PyErr::fetch(dict.py())),
            }
        }

        let py = self.py();
        inner(
            self,
            key.into_pyobject_or_pyerr(py)?.into_any().as_borrowed(),
        )
    }

    fn get_item<K>(&self, key: K) -> PyResult<Option<Bound<'py, PyAny>>>
    where
        K: IntoPyObject<'py>,
    {
        fn inner<'py>(
            dict: &Bound<'py, PyAnyDict>,
            key: Borrowed<'_, '_, PyAny>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            let py = dict.py();
            let mut result: *mut ffi::PyObject = core::ptr::null_mut();
            match unsafe {
                ffi::compat::PyDict_GetItemRef(dict.as_ptr(), key.as_ptr(), &mut result)
            } {
                core::ffi::c_int::MIN..=-1 => Err(PyErr::fetch(py)),
                0 => Ok(None),
                1..=core::ffi::c_int::MAX => {
                    // Safety: PyDict_GetItemRef positive return value means the result is a valid
                    // owned reference
                    Ok(Some(unsafe { result.assume_owned_unchecked(py) }))
                }
            }
        }

        let py = self.py();
        inner(
            self,
            key.into_pyobject_or_pyerr(py)?.into_any().as_borrowed(),
        )
    }

    fn keys(&self) -> Bound<'py, PyList> {
        unsafe {
            ffi::PyDict_Keys(self.as_ptr())
                .assume_owned(self.py())
                .cast_into_unchecked()
        }
    }

    fn values(&self) -> Bound<'py, PyList> {
        unsafe {
            ffi::PyDict_Values(self.as_ptr())
                .assume_owned(self.py())
                .cast_into_unchecked()
        }
    }

    fn items(&self) -> Bound<'py, PyList> {
        unsafe {
            ffi::PyDict_Items(self.as_ptr())
                .assume_owned(self.py())
                .cast_into_unchecked()
        }
    }

    fn as_mapping(&self) -> &Bound<'py, PyMapping> {
        unsafe { self.cast_unchecked() }
    }

    fn into_mapping(self) -> Bound<'py, PyMapping> {
        unsafe { self.cast_into_unchecked() }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{PyAnyDict, PyDict};
    use crate::{Bound, PyAny, Python};

    #[test]
    fn test_deref_dict() {
        Python::attach(|py| {
            let dict = PyDict::new(py);
            let _anydict: &Bound<'_, PyAnyDict> = &dict;
            let _any: &Bound<'_, PyAny> = &dict;
        });
    }
}
