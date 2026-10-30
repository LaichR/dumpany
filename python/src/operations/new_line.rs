use dumpany_rs::core::Indentation;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass]
/// A dumper operation that inserts a line break and optionally changes indentation.
///
/// The indentation change applies to subsequent lines. With neither argument,
/// this operation is equivalent to dumping Ellipsis.
#[pyclass(frozen, module = "dumpany")]
pub struct NewLine {
    pub(crate) indentation: Option<Indentation>,
}

#[gen_stub_pymethods]
#[pymethods]
impl NewLine {
    /// Create a newline with an optional indentation increment or decrement.
    ///
    /// Counts must be integers from 0 to 255. Supplying both arguments raises
    /// ValueError, even when either count is zero.
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
