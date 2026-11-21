use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

/// Emit text without inserting line breaks within it.
///
/// Wraps before the text if needed; oversized text overflows intact.
/// Existing line endings are preserved, not escaped.
/// Text exceeding 65,535 Unicode characters raises OSError when dumped.
#[gen_stub_pyclass]
#[pyclass(module = "dumpany", frozen)]
pub struct NoBreak {
    pub(crate) text: String,
}

#[gen_stub_pymethods]
#[pymethods]
impl NoBreak {
    /// Create an indivisible text operation.
    ///
    /// :param text: Text to emit intact when the operation is dumped.
    #[new]
    fn new(text: String) -> Self {
        Self { text }
    }
}
