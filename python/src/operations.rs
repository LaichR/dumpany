use dumpany::core::Indentation;
use pyo3::class::gc::{PyTraverseError, PyVisit};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod context;
pub(crate) use context::PyContext;

/// Lazily dump iterable items with a delimiter between them.
#[pyclass(module = "dumpany")]
pub struct Join {
    pub(crate) contents: Option<(Py<PyAny>, Py<PyAny>)>,
}

#[pymethods]
impl Join {
    #[new]
    #[pyo3(signature = (delimiter, iterable))]
    fn new(delimiter: Py<PyAny>, iterable: Py<PyAny>) -> Self {
        Self {
            contents: Some((delimiter, iterable)),
        }
    }
    /// used for garbage collection
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some((delimiter, iterable)) = &self.contents {
            visit.call(delimiter)?;
            visit.call(iterable)?;
        }
        Ok(())
    }
    /// collect python objects
    fn __clear__(&mut self) {
        self.contents = None;
    }
}

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
