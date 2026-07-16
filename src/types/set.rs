use crate::types::anyset::{PyAnySet, PyAnySetMethods};
use crate::types::PyIterator;
use crate::{
    err::{self, PyErr, PyResult},
    ffi_ptr_ext::FfiPtrExt,
    instance::Bound,
    py_result_ext::PyResultExt,
};
use crate::{ffi, Borrowed, BoundObject, IntoPyObject, IntoPyObjectExt, PyAny, Python};
#[cfg(RustPython)]
use crate::{
    sync::PyOnceLock,
    types::{PyType, PyTypeMethods},
    Py,
};
use core::ops::Deref;
use core::ptr;

/// Represents a Python `set`.
///
/// Values of this type are accessed via PyO3's smart pointers, e.g. as
/// [`Py<PySet>`][crate::Py] or [`Bound<'py, PySet>`][Bound].
///
/// For APIs available on `set` objects, see the [`PySetMethods`] trait which is implemented for
/// [`Bound<'py, PySet>`][Bound].
#[repr(transparent)]
pub struct PySet(PyAny);

#[cfg(not(any(PyPy, GraalPy)))]
pyobject_subclassable_native_type!(PySet, crate::ffi::PySetObject);

#[cfg(all(not(any(PyPy, GraalPy)), not(RustPython)))]
pyobject_native_type_info!(
    PySet,
    pyobject_native_static_type_object!(ffi::PySet_Type),
    "builtins",
    "set",
    Some("builtins"),
    #checkfunction=ffi::PySet_Check
);

#[cfg(all(not(any(PyPy, GraalPy, RustPython)), not(Py_LIMITED_API)))]
pyobject_native_type_sized!(PySet, ffi::PySetObject);

#[cfg(all(not(any(PyPy, GraalPy)), RustPython))]
pyobject_native_type_info!(
    PySet,
    |py| {
        static TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        TYPE.import(py, "builtins", "set").unwrap().as_type_ptr()
    },
    "builtins",
    "set",
    Some("builtins"),
    #checkfunction=ffi::PySet_Check
);

#[cfg(any(PyPy, GraalPy))]
pyobject_native_type_info!(
    PySet,
    pyobject_native_static_type_object!(ffi::PySet_Type),
    "builtins",
    "set",
    Some("builtins"),
    #checkfunction=ffi::PySet_Check
);

impl<'py> Deref for Bound<'py, PySet> {
    type Target = Bound<'py, PyAnySet>;

    fn deref(&self) -> &Self::Target {
        self.as_any_set()
    }
}

impl<'py> From<Bound<'py, PySet>> for Bound<'py, PyAnySet> {
    fn from(set: Bound<'py, PySet>) -> Self {
        set.into_any_set()
    }
}

impl<'py> AsRef<Bound<'py, PyAnySet>> for Bound<'py, PySet> {
    fn as_ref(&self) -> &Bound<'py, PyAnySet> {
        self.as_any_set()
    }
}

impl PySet {
    /// Creates a new set with elements from the given slice.
    ///
    /// Returns an error if some element is not hashable.
    #[inline]
    pub fn new<'py, T>(
        py: Python<'py>,
        elements: impl IntoIterator<Item = T>,
    ) -> PyResult<Bound<'py, PySet>>
    where
        T: IntoPyObject<'py>,
    {
        let set = Self::empty(py)?;
        for e in elements {
            set.add(e)?;
        }
        Ok(set)
    }

    /// Creates a new empty set.
    pub fn empty(py: Python<'_>) -> PyResult<Bound<'_, PySet>> {
        unsafe {
            ffi::PySet_New(ptr::null_mut())
                .assume_owned_or_err(py)
                .cast_into_unchecked()
        }
    }
}

/// Implementation of functionality for [`PySet`].
///
/// These methods are defined for the `Bound<'py, PySet>` smart pointer, so to use method call
/// syntax these methods are separated into a trait, because stable Rust does not yet support
/// `arbitrary_self_types`.
#[doc(alias = "PySet")]
pub trait PySetMethods<'py>: crate::sealed::Sealed {
    /// Removes all elements from the set.
    fn clear(&self);

    /// Returns the number of items in the set.
    ///
    /// This is equivalent to the Python expression `len(self)`.
    fn len(&self) -> usize;

    /// Checks if set is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Determines if the set contains the specified key.
    ///
    /// This is equivalent to the Python expression `key in self`.
    fn contains<K>(&self, key: K) -> PyResult<bool>
    where
        K: IntoPyObject<'py>;

    /// Removes the element from the set if it is present.
    ///
    /// Returns `true` if the element was present in the set.
    fn discard<K>(&self, key: K) -> PyResult<bool>
    where
        K: IntoPyObject<'py>;

    /// Adds an element to the set.
    fn add<K>(&self, key: K) -> PyResult<()>
    where
        K: IntoPyObject<'py>;

    /// Removes and returns an arbitrary element from the set.
    fn pop(&self) -> Option<Bound<'py, PyAny>>;

    /// Returns `self` cast as [`PyAnySet`].
    fn as_any_set(&self) -> &Bound<'py, PyAnySet>;

    /// Returns `self` cast as [`PyAnySet`].
    fn into_any_set(self) -> Bound<'py, PyAnySet>;

    /// Returns an iterator of values in this set.
    ///
    /// # Panics
    ///
    /// If PyO3 detects that the set is mutated during iteration, it will panic.
    fn iter(&self) -> BoundSetIterator<'py>;
}

impl<'py> PySetMethods<'py> for Bound<'py, PySet> {
    #[inline]
    fn clear(&self) {
        unsafe {
            ffi::PySet_Clear(self.as_ptr());
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.as_any_set().len()
    }

    fn contains<K>(&self, key: K) -> PyResult<bool>
    where
        K: IntoPyObject<'py>,
    {
        self.as_any_set().contains(key)
    }

    fn discard<K>(&self, key: K) -> PyResult<bool>
    where
        K: IntoPyObject<'py>,
    {
        fn inner(set: &Bound<'_, PySet>, key: Borrowed<'_, '_, PyAny>) -> PyResult<bool> {
            match unsafe { ffi::PySet_Discard(set.as_ptr(), key.as_ptr()) } {
                1 => Ok(true),
                0 => Ok(false),
                _ => Err(PyErr::fetch(set.py())),
            }
        }

        let py = self.py();
        inner(
            self,
            key.into_pyobject_or_pyerr(py)?.into_any().as_borrowed(),
        )
    }

    fn add<K>(&self, key: K) -> PyResult<()>
    where
        K: IntoPyObject<'py>,
    {
        fn inner(set: &Bound<'_, PySet>, key: Borrowed<'_, '_, PyAny>) -> PyResult<()> {
            err::error_on_minusone(set.py(), unsafe {
                ffi::PySet_Add(set.as_ptr(), key.as_ptr())
            })
        }

        let py = self.py();
        inner(
            self,
            key.into_pyobject_or_pyerr(py)?.into_any().as_borrowed(),
        )
    }

    fn pop(&self) -> Option<Bound<'py, PyAny>> {
        let element = unsafe { ffi::PySet_Pop(self.as_ptr()).assume_owned_or_err(self.py()) };
        element.ok()
    }

    fn as_any_set(&self) -> &Bound<'py, PyAnySet> {
        unsafe { self.cast_unchecked() }
    }

    fn into_any_set(self) -> Bound<'py, PyAnySet> {
        unsafe { self.cast_into_unchecked() }
    }

    fn iter(&self) -> BoundSetIterator<'py> {
        BoundSetIterator::new(self.clone())
    }
}

impl<'py> IntoIterator for Bound<'py, PySet> {
    type Item = Bound<'py, PyAny>;
    type IntoIter = BoundSetIterator<'py>;

    /// Returns an iterator of values in this set.
    ///
    /// # Panics
    ///
    /// If PyO3 detects that the set is mutated during iteration, it will panic.
    fn into_iter(self) -> Self::IntoIter {
        BoundSetIterator::new(self)
    }
}

impl<'py> IntoIterator for &Bound<'py, PySet> {
    type Item = Bound<'py, PyAny>;
    type IntoIter = BoundSetIterator<'py>;

    /// Returns an iterator of values in this set.
    ///
    /// # Panics
    ///
    /// If PyO3 detects that the set is mutated during iteration, it will panic.
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// PyO3 implementation of an iterator for a Python `set` object.
pub struct BoundSetIterator<'py>(Bound<'py, PyIterator>);

impl<'py> BoundSetIterator<'py> {
    pub(super) fn new(set: Bound<'py, PySet>) -> Self {
        Self(PyIterator::from_object(&set).expect("set should always be iterable"))
    }
}

impl<'py> Iterator for BoundSetIterator<'py> {
    type Item = Bound<'py, super::PyAny>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|result| result.expect("set iteration should be infallible"))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = ExactSizeIterator::len(self);
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

impl ExactSizeIterator for BoundSetIterator<'_> {
    fn len(&self) -> usize {
        self.0.size_hint().0
    }
}

#[cfg(test)]
mod tests {
    use super::PySet;
    use crate::platform::HashSet;
    use crate::{
        conversion::IntoPyObject,
        types::{PyAnyMethods, PySetMethods},
        Python,
    };

    #[test]
    fn test_set_new() {
        Python::attach(|py| {
            let set = PySet::new(py, [1]).unwrap();
            assert_eq!(1, set.len());

            let v = vec![1];
            assert!(PySet::new(py, &[v]).is_err());
        });
    }

    #[test]
    fn test_set_empty() {
        Python::attach(|py| {
            let set = PySet::empty(py).unwrap();
            assert_eq!(0, set.len());
            assert!(set.is_empty());
        });
    }

    #[test]
    fn test_set_len() {
        Python::attach(|py| {
            let mut v = HashSet::<i32>::new();
            let ob = (&v).into_pyobject(py).unwrap();
            let set = ob.cast::<PySet>().unwrap();
            assert_eq!(0, set.len());
            v.insert(7);
            let ob = v.into_pyobject(py).unwrap();
            let set2 = ob.cast::<PySet>().unwrap();
            assert_eq!(1, set2.len());
        });
    }

    #[test]
    fn test_set_clear() {
        Python::attach(|py| {
            let set = PySet::new(py, [1]).unwrap();
            assert_eq!(1, set.len());
            set.clear();
            assert_eq!(0, set.len());
        });
    }

    #[test]
    fn test_set_contains() {
        Python::attach(|py| {
            let set = PySet::new(py, [1]).unwrap();
            assert!(set.contains(1).unwrap());
        });
    }

    #[test]
    fn test_set_discard() {
        Python::attach(|py| {
            let set = PySet::new(py, [1]).unwrap();
            assert!(!set.discard(2).unwrap());
            assert_eq!(1, set.len());

            assert!(set.discard(1).unwrap());
            assert_eq!(0, set.len());
            assert!(!set.discard(1).unwrap());

            assert!(set.discard(vec![1, 2]).is_err());
        });
    }

    #[test]
    fn test_set_add() {
        Python::attach(|py| {
            let set = PySet::new(py, [1, 2]).unwrap();
            set.add(1).unwrap();
            assert!(set.contains(1).unwrap());
        });
    }

    #[test]
    fn test_set_pop() {
        Python::attach(|py| {
            let set = PySet::new(py, [1]).unwrap();
            let val = set.pop();
            assert!(val.is_some());
            let val2 = set.pop();
            assert!(val2.is_none());
            assert!(py
                .eval(c"print('Exception state should not be set.')", None, None)
                .is_ok());
        });
    }

    #[test]
    fn test_set_iter() {
        Python::attach(|py| {
            let set = PySet::new(py, [1]).unwrap();

            for el in set {
                assert_eq!(1i32, el.extract::<'_, i32>().unwrap());
            }
        });
    }

    #[test]
    fn test_set_iter_bound() {
        use crate::types::any::PyAnyMethods;

        Python::attach(|py| {
            let set = PySet::new(py, [1]).unwrap();

            for el in &set {
                assert_eq!(1i32, el.extract::<i32>().unwrap());
            }
        });
    }

    #[test]
    #[should_panic]
    fn test_set_iter_mutation() {
        Python::attach(|py| {
            let set = PySet::new(py, [1, 2, 3, 4, 5]).unwrap();

            for _ in &set {
                let _ = set.add(42);
            }
        });
    }

    #[test]
    #[should_panic]
    fn test_set_iter_mutation_same_len() {
        Python::attach(|py| {
            let set = PySet::new(py, [1, 2, 3, 4, 5]).unwrap();

            for item in &set {
                let item: i32 = item.extract().unwrap();
                let _ = set.del_item(item);
                let _ = set.add(item + 10);
            }
        });
    }

    #[test]
    fn test_set_iter_size_hint() {
        Python::attach(|py| {
            let set = PySet::new(py, [1]).unwrap();
            let mut iter = set.iter();

            assert_eq!(iter.len(), 1);
            assert_eq!(iter.size_hint(), (1, Some(1)));
            iter.next();
            assert_eq!(iter.len(), 0);
            assert_eq!(iter.size_hint(), (0, Some(0)));
        });
    }

    #[test]
    fn test_iter_count() {
        Python::attach(|py| {
            let set = PySet::new(py, vec![1, 2, 3]).unwrap();
            assert_eq!(set.iter().count(), 3);
        })
    }
}
