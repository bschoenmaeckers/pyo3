use crate::err::{PyErr, PyResult};
use crate::instance::{Borrowed, Bound, BoundObject};
use crate::type_object::{PyTypeCheck, PyTypeInfo};
use crate::types::tuple::PyTuple;
use crate::types::{PyAny, PyFrozenSet, PySet};
use crate::{ffi, IntoPyObject, IntoPyObjectExt, Python};

/// Represents either a Python `set` or `frozenset`.
///
/// Values of this type are accessed via PyO3's smart pointers, e.g. as
/// [`Py<PyAnySet>`][crate::Py] or [`Bound<'py, PyAnySet>`][Bound].
///
/// For APIs available on both set types, see the [`PyAnySetMethods`] trait.
#[repr(transparent)]
pub struct PyAnySet(PyAny);

pyobject_native_type_named!(PyAnySet);

unsafe impl PyTypeCheck for PyAnySet {
    #[cfg(feature = "experimental-inspect")]
    const TYPE_HINT: crate::inspect::PyStaticExpr = crate::inspect::type_hint_union!(
        <PySet as PyTypeInfo>::TYPE_HINT,
        <PyFrozenSet as PyTypeInfo>::TYPE_HINT
    );

    #[inline]
    fn type_check(object: &Bound<'_, PyAny>) -> bool {
        unsafe { ffi::PyAnySet_Check(object.as_ptr()) > 0 }
    }

    fn classinfo_object(py: Python<'_>) -> Bound<'_, PyAny> {
        PyTuple::new(
            py,
            [
                PySet::type_object(py).into_any(),
                PyFrozenSet::type_object(py).into_any(),
            ],
        )
        .expect("PyAnySet classinfo tuple construction should not fail")
        .into_any()
    }
}

/// Implementation of functionality shared by Python `set` and `frozenset` objects.
#[doc(alias = "PyAnySet")]
pub trait PyAnySetMethods<'py>: crate::sealed::Sealed {
    /// Returns the number of items in the set.
    ///
    /// This is equivalent to the Python expression `len(self)`.
    fn len(&self) -> usize;

    /// Checks if set is empty.
    fn is_empty(&self) -> bool;

    /// Determines if the set contains the specified key.
    ///
    /// This is equivalent to the Python expression `key in self`.
    fn contains<K>(&self, key: K) -> PyResult<bool>
    where
        K: IntoPyObject<'py>;
}

impl<'py> PyAnySetMethods<'py> for Bound<'py, PyAnySet> {
    #[inline]
    fn len(&self) -> usize {
        unsafe { ffi::PySet_Size(self.as_ptr()) as usize }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn contains<K>(&self, key: K) -> PyResult<bool>
    where
        K: IntoPyObject<'py>,
    {
        fn inner(set: &Bound<'_, PyAnySet>, key: Borrowed<'_, '_, PyAny>) -> PyResult<bool> {
            match unsafe { ffi::PySet_Contains(set.as_ptr(), key.as_ptr()) } {
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
}
