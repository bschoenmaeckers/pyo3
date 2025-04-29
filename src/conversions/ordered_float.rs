#![cfg(feature = "ordered-float")]

use crate::types::{PyAnyMethods, PyFloat};
use crate::{Bound, FromPyObject, IntoPyObject, PyAny, PyResult, Python};
use ordered_float::{FloatCore, OrderedFloat};
use std::convert::Infallible;

impl<'py, T: FloatCore> IntoPyObject<'py> for OrderedFloat<T>
where
    T: IntoPyObject<'py, Output = Bound<'py, PyFloat>, Error = Infallible>,
{
    type Target = PyFloat;
    type Output = Bound<'py, PyFloat>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.into_inner().into_pyobject(py)
    }
}

impl<'py, T: FloatCore> IntoPyObject<'py> for &OrderedFloat<T>
where
    T: IntoPyObject<'py, Output = Bound<'py, PyFloat>, Error = Infallible>,
{
    type Target = PyFloat;
    type Output = Bound<'py, PyFloat>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        (*self).into_pyobject(py)
    }
}

impl<'py, T: FloatCore> FromPyObject<'py> for OrderedFloat<T>
where
    T: FromPyObject<'py>,
{
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        ob.extract().map(OrderedFloat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Python;

    #[test]
    fn test_ordered_float() {
        Python::with_gil(|py| {
            let ordered_float = OrderedFloat(3.14);
            let py_obj: Bound<'_, _> = ordered_float.into_pyobject(py).unwrap();
            assert_eq!(
                py_obj.extract::<OrderedFloat<f64>>().unwrap(),
                ordered_float
            );
        });
    }
}
