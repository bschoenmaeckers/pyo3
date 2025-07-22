use crate::{PyBytes_FromStringAndSize, PyBytes_Size, PyObject, _PyBytes_Resize};

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn PyBytesWriter_Create(length: crate::Py_ssize_t) -> *mut crate::PyBytesWriter {
        if length < 0 {
            crate::PyErr_SetString(
                crate::PyExc_ValueError,
                c_str!("size must be >= 0").as_ptr(),
            );
            return std::ptr::null_mut();
        }

        let size = size_of::<crate::PyBytesWriter>();
        let writer: *mut crate::PyBytesWriter = crate::PyMem_Malloc(size).cast();

        if writer.is_null() {
            crate::PyErr_NoMemory();
            return std::ptr::null_mut();
        }

        (*writer).obj = std::ptr::null_mut();
        (*writer).size = 0;

        if length > 0 {
            if _PyBytesWriter_Resize_impl(writer, size as crate::Py_ssize_t, 0) < 0 {
                PyBytesWriter_Discard(writer);
                return std::ptr::null_mut();
            }
            (*writer).size = length
        }

        writer
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn PyBytesWriter_Discard(writer: *mut crate::PyBytesWriter) -> () {
        if writer.is_null() {
            return;
        }

        crate::Py_XDECREF((*writer).obj);
        crate::PyMem_Free(writer.cast());
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn PyBytesWriter_Finish(writer: *mut crate::PyBytesWriter) -> *mut PyObject {
        PyBytesWriter_FinishWithSize(writer, (*writer).size)
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn PyBytesWriter_FinishWithSize(writer: *mut crate::PyBytesWriter, size: crate::Py_ssize_t) -> *mut PyObject {
        let result = if size == 0 {
            PyBytes_FromStringAndSize(c_str!("").as_ptr(), 0)
        } else if (*writer).obj.is_null() {
            PyBytes_FromStringAndSize((*writer).small_buffer.as_ptr(), size)
        } else {
            if size != PyBytes_Size((*writer).obj) {
                if _PyBytes_Resize(&mut (*writer).obj, size) < 1 {
                    PyBytesWriter_Discard(writer);
                    return std::ptr::null_mut();
                }
            }
            std::mem::take(&mut (*writer).obj)
        };

        PyBytesWriter_Discard(writer);
        result
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn PyBytesWriter_WriteBytes(writer: *mut crate::PyBytesWriter, bytes: *const std::ffi::c_void, size: crate::Py_ssize_t) -> std::os::raw::c_int {
        if size < 0 {
            todo!()
        }

        let pos = (*writer).size;
        if PyBytesWriter_Grow(writer, size) < 0 {
            return -1;
        }

        std::ptr::copy_nonoverlapping(
            bytes,
            PyBytesWriter_GetData(writer).add(pos as usize),
            size as usize,
        );

        0
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn PyBytesWriter_GetData(writer: *mut crate::PyBytesWriter) -> *mut std::ffi::c_void {
        if (*writer).obj.is_null() {
            (*writer).small_buffer.as_mut_ptr().cast()
        } else {
            crate::PyBytes_AsString((*writer).obj).cast()
        }
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn PyBytesWriter_GetSize(writer: *mut crate::PyBytesWriter) -> crate::Py_ssize_t {
        (*writer).size
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn PyBytesWriter_Resize(writer: *mut crate::PyBytesWriter, size: crate::Py_ssize_t) -> std::ffi::c_int {
        if size < 0 {
            crate::PyErr_SetString(
                crate::PyExc_ValueError,
                c_str!("size must be >= 0").as_ptr(),
            );
            return -1;
        }

        if _PyBytesWriter_Resize_impl(writer, size, 1) < 0 {
            return -1;
        }

        (*writer).size = size;
        0
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn PyBytesWriter_Grow(writer: *mut crate::PyBytesWriter, size: crate::Py_ssize_t) -> std::ffi::c_int {
        if size < 0 && (*writer).size + size < 0 {
            crate::PyErr_SetString(
                crate::PyExc_ValueError,
                c_str!("invalid size").as_ptr(),
            );
            return -1;
        }

        if size > crate::Py_ssize_t::MAX - (*writer).size {
            crate::PyErr_NoMemory();
            return -1;
        }

        let size = (*writer).size + size;
        if _PyBytesWriter_Resize_impl(writer, size, 1) < 0 {
            return -1;
        }
        (*writer).size = size;
        0
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn _PyBytesWriter_GetAllocated(writer: *mut crate::PyBytesWriter) -> crate::Py_ssize_t {
        if (*writer).obj.is_null() {
            return (*writer).small_buffer.len() as crate::Py_ssize_t;
        }
        crate::PyBytes_Size((*writer).obj)
    }
);

compat_function!(
    originally_defined_for(Py_3_15);

    pub unsafe fn _PyBytesWriter_Resize_impl(
        writer: *mut crate::PyBytesWriter,
        size: crate::Py_ssize_t,
        overallocate: std::os::raw::c_int,
    ) -> std::os::raw::c_int {
        debug_assert!(size > 0);

        if size <= _PyBytesWriter_GetAllocated(writer) {
            return 0; // No resize needed
        }

        let mut size = size;
        if overallocate < 1 {
            #[cfg(windows)]
            if size <= (crate::Py_ssize_t::MAX - size / 2) {
                size += size / 2;
            }

            #[cfg(not(windows))]
            if size <= (crate::Py_ssize_t::MAX - size / 4) {
                size += size / 4;
            }
        }

        if (*writer).obj.is_null() {
            (*writer).obj = crate::PyBytes_FromStringAndSize(std::ptr::null(), size);
            if (*writer).obj.is_null() {
                return -1;
            }

            debug_assert!(size > (*writer).small_buffer.len() as isize);
            std::ptr::copy_nonoverlapping(
                (*writer).small_buffer.as_ptr(),
                crate::PyBytes_AsString((*writer).obj).cast(),
                (*writer).small_buffer.len(),
            );
        } else {
            if crate::_PyBytes_Resize(&mut (*writer).obj, size) < 1 {
                return -1;
            }
            debug_assert!(!(*writer).obj.is_null())
        }

        0
    }
);
