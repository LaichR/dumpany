use pyo3::class::gc::{PyTraverseError, PyVisit};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass]
/// A dumper operation that separates iterable items with a delimiter.
///
/// Each item and the delimiter are dumped recursively, as with Dumper.dump().
/// Empty iterables emit nothing; no leading or trailing delimiter is emitted.
#[pyclass(module = "dumpany")]
pub struct Join {
    pub(crate) contents: Option<(Py<PyAny>, Py<PyAny>)>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Join {
    /// Create a join with a delimiter between adjacent items of iterable.
    ///
    /// The iterable is consumed only when the join is dumped.
    /// Generators and other one-shot iterators are not cached or rewound.
    #[new]
    #[pyo3(signature = (delimiter, iterable))]
    fn new(delimiter: Py<PyAny>, iterable: Py<PyAny>) -> Self {
        Self {
            contents: Some((delimiter, iterable)),
        }
    }
}

#[pymethods]
impl Join {
    /// Expose delimiter and iterable references to the garbage collector.
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some((delimiter, iterable)) = &self.contents {
            visit.call(delimiter)?;
            visit.call(iterable)?;
        }
        Ok(())
    }

    /// Break unreachable cycles through the delimiter or iterable.
    fn __clear__(&mut self) {
        self.contents = None;
    }
}
