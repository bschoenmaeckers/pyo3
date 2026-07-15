use crate::conversion;
use crate::err::{self, PyResult};
use crate::ffi_ptr_ext::FfiPtrExt;
use crate::instance::Bound;
use crate::types::anydict::{PyAnyDict, PyAnyDictMethods};
use crate::types::{PyAny, PyMapping};
use crate::{ffi, BoundObject, IntoPyObject, Py, Python};
#[cfg(Py_LIMITED_API)]
use crate::{
    sync::PyOnceLock,
    type_object::PyTypeInfo,
    types::{PyType, PyTypeMethods},
    Py,
};
use core::ops::Deref;
#[cfg(not(Py_LIMITED_API))]
use core::ptr;

/// Represents a Python `frozendict`.
///
/// Values of this type are accessed via PyO3's smart pointers, e.g. as
/// [`Py<PyFrozenDict>`][crate::Py] or [`Bound<'py, PyFrozenDict>`][Bound].
///
/// For APIs available on `frozendict` objects, see the [`PyFrozenDictMethods`] trait which is implemented for
/// [`Bound<'py, PyFrozenDict>`][Bound].
///
/// This type is only available on Python 3.15+.
#[repr(transparent)]
pub struct PyFrozenDict(PyAny);

#[cfg(not(Py_LIMITED_API))]
pyobject_native_type_info!(
    PyFrozenDict,
    pyobject_native_static_type_object!(ffi::PyFrozenDict_Type),
    "builtins",
    "frozendict",
    Some("builtins"),
    #checkfunction=ffi::PyFrozenDict_Check
);

#[cfg(Py_LIMITED_API)]
pyobject_native_type_info!(
    PyFrozenDict,
    |py| {
        static TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        TYPE.import(py, "builtins", "frozendict")
            .unwrap()
            .as_type_ptr()
    },
    "builtins",
    "frozendict",
    Some("builtins")
);

impl<'py> Deref for Bound<'py, PyFrozenDict> {
    type Target = Bound<'py, PyAnyDict>;

    fn deref(&self) -> &Self::Target {
        self.as_any_dict()
    }
}

impl From<Bound<'_, PyFrozenDict>> for Py<PyAny> {
    fn from(dict: Bound<'_, PyFrozenDict>) -> Self {
        dict.into_any().unbind()
    }
}

impl From<Py<PyFrozenDict>> for Py<PyAny> {
    fn from(dict: Py<PyFrozenDict>) -> Self {
        dict.into_any()
    }
}

impl<'py> From<Bound<'py, PyFrozenDict>> for Bound<'py, PyAnyDict> {
    fn from(dict: Bound<'py, PyFrozenDict>) -> Self {
        dict.into_any_dict()
    }
}

impl<'py> AsRef<Bound<'py, PyAnyDict>> for Bound<'py, PyFrozenDict> {
    fn as_ref(&self) -> &Bound<'py, PyAnyDict> {
        self.as_any_dict()
    }
}

impl<'py> From<Bound<'py, PyFrozenDict>> for Bound<'py, PyMapping> {
    fn from(dict: Bound<'py, PyFrozenDict>) -> Self {
        dict.into_any_dict().into_mapping()
    }
}

impl<'py> AsRef<Bound<'py, PyMapping>> for Bound<'py, PyFrozenDict> {
    fn as_ref(&self) -> &Bound<'py, PyMapping> {
        self.as_any_dict().as_mapping()
    }
}

impl PyFrozenDict {
    /// Creates a new frozendict.
    pub fn new<'py, T>(py: Python<'py>, iterable: T) -> PyResult<Bound<'py, PyFrozenDict>>
    where
        T: IntoPyObject<'py>,
        err::PyErr: core::convert::From<<T as conversion::IntoPyObject<'py>>::Error>,
    {
        #[cfg(Py_LIMITED_API)]
        {
            PyFrozenDict::type_object(py)
                .call1((iterable,))
                .map(|obj| unsafe { obj.cast_into_unchecked() })
        }
        #[cfg(not(Py_LIMITED_API))]
        {
            let obj = iterable.into_pyobject(py)?;
            unsafe {
                ffi::PyFrozenDict_New(obj.as_ptr())
                    .assume_owned_or_err(py)
                    .map(|obj| obj.cast_into_unchecked())
            }
        }
    }

    /// Creates a new empty frozendict
    pub fn empty(py: Python<'_>) -> PyResult<Bound<'_, PyFrozenDict>> {
        #[cfg(Py_LIMITED_API)]
        {
            PyFrozenDict::type_object(py)
                .call0()
                .map(|obj| unsafe { obj.cast_into_unchecked() })
        }
        #[cfg(not(Py_LIMITED_API))]
        unsafe {
            ffi::PyFrozenDict_New(ptr::null_mut())
                .assume_owned_or_err(py)
                .map(|obj| obj.cast_into_unchecked())
        }
    }
}

/// Implementation of functionality for [`PyFrozenDict`].
///
/// These methods are defined for the `Bound<'py, PyFrozenDict>` smart pointer,
/// so to use method call syntax these methods are separated into a trait,
/// because stable Rust does not yet support`arbitrary_self_types`.
#[doc(alias = "PyFrozenDict")]
pub trait PyFrozenDictMethods<'py>: crate::sealed::Sealed {
    /// Returns `self` cast as [`PyAnyDict`].
    fn as_any_dict(&self) -> &Bound<'py, PyAnyDict>;

    /// Returns `self` cast as [`PyAnyDict`].
    fn into_any_dict(self) -> Bound<'py, PyAnyDict>;

    /// Returns an iterator of `(key, value)` tuples in this frozendict.
    ///
    /// Since `frozendict` objects are immutable, iteration does not need the
    /// mutation guards that are required for a standard dict.
    fn iter(&self) -> BoundFrozenDictIterator<'py>;
}

impl<'py> PyFrozenDictMethods<'py> for Bound<'py, PyFrozenDict> {
    fn as_any_dict(&self) -> &Bound<'py, PyAnyDict> {
        unsafe { self.cast_unchecked() }
    }

    fn into_any_dict(self) -> Bound<'py, PyAnyDict> {
        unsafe { self.cast_into_unchecked() }
    }

    fn iter(&self) -> BoundFrozenDictIterator<'py> {
        BoundFrozenDictIterator::new(self.clone())
    }
}

/// An iterator over the items in a frozendict.
///
/// Created by the `iter()` method on `Bound<'py, PyFrozenDict>`.
///
/// Because the underlying mapping cannot be mutated, this iterator simply
/// walks the current contents as a stable snapshot.
pub struct BoundFrozenDictIterator<'py> {
    fd: Bound<'py, PyFrozenDict>,
    ppos: isize,
    remaining: usize,
}

impl<'py> BoundFrozenDictIterator<'py> {
    fn new(fd: Bound<'py, PyFrozenDict>) -> Self {
        let remaining = fd.len();
        BoundFrozenDictIterator {
            fd,
            ppos: 0,
            remaining,
        }
    }
}

impl<'py> Iterator for BoundFrozenDictIterator<'py> {
    type Item = (Bound<'py, PyAny>, Bound<'py, PyAny>);

    fn next(&mut self) -> Option<Self::Item> {
        let ppos: *mut ffi::Py_ssize_t = &mut self.ppos;
        let mut key: *mut ffi::PyObject = core::ptr::null_mut();
        let mut value: *mut ffi::PyObject = core::ptr::null_mut();

        if unsafe { ffi::PyDict_Next(self.fd.as_ptr(), ppos, &mut key, &mut value) != 0 } {
            let py = self.fd.py();
            self.remaining -= 1;
            // Safety:
            // - PyDict_Next returns borrowed values
            // - we have already checked that `PyDict_Next` succeeded, so we can assume these to be non-null
            Some((
                unsafe { key.assume_borrowed_unchecked(py).to_owned() },
                unsafe { value.assume_borrowed_unchecked(py).to_owned() },
            ))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len()
    }
}

impl ExactSizeIterator for BoundFrozenDictIterator<'_> {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<'py> IntoIterator for Bound<'py, PyFrozenDict> {
    type Item = (Bound<'py, PyAny>, Bound<'py, PyAny>);
    type IntoIter = BoundFrozenDictIterator<'py>;

    /// Returns an iterator over the `(key, value)` pairs in this frozendict.
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'py> IntoIterator for &Bound<'py, PyFrozenDict> {
    type Item = (Bound<'py, PyAny>, Bound<'py, PyAny>);
    type IntoIter = BoundFrozenDictIterator<'py>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(all(Py_3_15, test))]
mod tests {
    use super::*;
    use crate::types::{list::PyListMethods, mapping::PyMappingMethods, PyAnyMethods};

    use alloc::string::{String, ToString};
    use alloc::vec::Vec;

    #[test]
    fn test_frozendict_new() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1), ("b", 2)]).unwrap();
            assert_eq!(fd.len(), 2);
        })
    }

    #[test]
    fn test_frozendict_empty() {
        Python::attach(|py| {
            let fd = PyFrozenDict::empty(py).unwrap();
            assert!(fd.is_empty());
            assert_eq!(fd.len(), 0);
        })
    }

    #[test]
    fn test_frozendict_contains() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1), ("b", 2)]).unwrap();
            assert!(fd.contains("a").unwrap());
            assert!(!fd.contains("c").unwrap());
        })
    }

    #[test]
    fn test_frozendict_get_item() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1), ("b", 2)]).unwrap();
            let val = fd.get_item("a").unwrap();
            assert!(val.is_some_and(|v| v.extract::<i32>().unwrap() == 1));
        })
    }

    #[test]
    fn test_frozendict_keys() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1), ("b", 2)]).unwrap();
            let keys = fd.keys();
            assert_eq!(keys.len(), 2);
            assert!(keys.contains("a").unwrap());
            assert!(keys.contains("b").unwrap());
        })
    }

    #[test]
    fn test_frozendict_values() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1), ("b", 2)]).unwrap();
            let values = fd.values();
            assert_eq!(values.len(), 2);
            assert!(values.contains(1).unwrap());
            assert!(values.contains(2).unwrap());
        })
    }

    #[test]
    fn test_frozendict_items() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1), ("b", 2)]).unwrap();
            let items = fd.items();
            assert_eq!(items.len(), 2);
            assert!(items.contains(("a", 1)).unwrap());
            assert!(items.contains(("b", 2)).unwrap());
        })
    }

    #[test]
    fn test_frozendict_iter() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1), ("b", 2)]).unwrap();
            let mut count = 0;
            for ((k, v), (expected_k, expected_v)) in fd.iter().zip([("a", 1), ("b", 2)].iter()) {
                count += 1;
                assert_eq!(
                    (k.extract::<String>().unwrap(), v.extract::<i32>().unwrap()),
                    (expected_k.to_string(), *expected_v)
                );
            }
            assert_eq!(count, 2);
        })
    }

    #[test]
    fn test_frozendict_iter_size_hint() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1), ("b", 2)]).unwrap();

            let mut iter = fd.iter();
            assert_eq!(iter.size_hint(), (2, Some(2)));
            iter.next();
            assert_eq!(iter.size_hint(), (1, Some(1)));

            for _ in &mut iter {}
            assert_eq!(iter.size_hint(), (0, Some(0)));
            assert!(iter.next().is_none());
        })
    }

    #[test]
    fn test_frozendict_into_iter() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1), ("b", 2)]).unwrap();
            let mut items = Vec::new();

            for (key, value) in fd {
                items.push((
                    key.extract::<String>().unwrap(),
                    value.extract::<i32>().unwrap(),
                ));
            }

            assert_eq!(items.len(), 2);
            assert!(items.contains(&("a".to_string(), 1)));
            assert!(items.contains(&("b".to_string(), 2)));
        })
    }

    #[test]
    fn test_frozendict_as_mapping() {
        Python::attach(|py| {
            let fd = PyFrozenDict::new(py, vec![("a", 1)]).unwrap();
            let mapping = fd.as_mapping();
            assert!(PyMappingMethods::len(mapping).unwrap() == 1);
        })
    }
}
