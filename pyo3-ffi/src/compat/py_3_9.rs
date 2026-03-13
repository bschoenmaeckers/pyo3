compat_function!(
    originally_defined_for(all(
        not(PyPy),
        any(Py_3_10, all(not(Py_LIMITED_API), Py_3_9)) // Added to python in 3.9 but to limited API in 3.10
    ));

    #[inline]
    pub unsafe fn PyObject_CallNoArgs(obj: *mut crate::PyObject) -> *mut crate::PyObject {
        crate::PyObject_CallObject(obj, std::ptr::null_mut())
    }
);

compat_function!(
    originally_defined_for(all(Py_3_9, not(any(Py_LIMITED_API, PyPy))));

    #[inline]
    pub unsafe fn PyObject_CallMethodNoArgs(obj: *mut crate::PyObject, name: *mut crate::PyObject) -> *mut crate::PyObject {
        crate::PyObject_CallMethodObjArgs(obj, name, std::ptr::null_mut::<crate::PyObject>())
    }
);

compat_function!(
    // Added to python in 3.9 but available in 3.8 using private vectorcall api
    originally_defined_for(all(Py_3_8, not(any(Py_LIMITED_API, PyPy))));

    #[inline]
    pub unsafe fn PyObject_CallOneArg(func: *mut crate::PyObject, arg: *mut crate::PyObject) -> *mut crate::PyObject {
        crate::PyObject_CallFunctionObjArgs(func, arg, std::ptr::null_mut::<crate::PyObject>())
    }
);
