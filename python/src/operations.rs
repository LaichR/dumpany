use dumpany::core::Indentation;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Emit a newline, then optionally change indentation.
#[pyclass(frozen, module = "dumpany")]
pub struct NewLine {
    pub(crate) indentation: Option<Indentation>,
}

#[pymethods]
impl NewLine {
    #[new]
    #[pyo3(signature = (*, increment=None, decrement=None))]
    fn new(increment: Option<u8>, decrement: Option<u8>) -> PyResult<Self> {
        let indentation = match (increment, decrement) {
            (Some(_), Some(_)) => {
                return Err(PyValueError::new_err(
                    "specify either increment or decrement, not both",
                ));
            }
            (Some(count), None) => Some(Indentation::Increment(count)),
            (None, Some(count)) => Some(Indentation::Decrement(count)),
            (None, None) => None,
        };
        Ok(Self { indentation })
    }
}
