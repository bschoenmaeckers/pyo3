compat_function!(
    originally_defined_for(Py_3_14);

    #[inline]
    pub unsafe fn Py_HashBuffer(
        ptr: *const std::ffi::c_void,
        len: crate::Py_ssize_t,
    ) -> crate::Py_hash_t {
        #[cfg(not(any(Py_LIMITED_API, PyPy, GraalPy)))]
        {
            crate::_Py_HashBytes(ptr, len)
        }

        #[cfg(any(Py_LIMITED_API, PyPy, GraalPy))]
        {
            let bytes = crate::PyBytes_FromStringAndSize(ptr as *const std::os::raw::c_char, len);
            if bytes.is_null() {
                -1
            } else {
                let result = crate::PyObject_Hash(bytes);
                crate::Py_DECREF(bytes);
                result
            }
        }
    }
);

compat_function!(
    originally_defined_for(Py_3_14);

    #[inline(always)]
    #[cfg(all(Py_3_12, not(Py_GIL_DISABLED)))]
    pub unsafe fn PyUnstable_IsImmortal(
        obj: *mut crate::PyObject,
    ) -> std::os::raw::c_int {
        #[cfg(target_pointer_width = "64")]
        {
            (((*obj).ob_refcnt.ob_refcnt as crate::PY_INT32_T) < 0) as  std::os::raw::c_int
        }

        #[cfg(target_pointer_width = "32")]
        {
            ((*obj).ob_refcnt.ob_refcnt == _Py_IMMORTAL_REFCNT) as  std::os::raw::c_int
        }
    }
);
