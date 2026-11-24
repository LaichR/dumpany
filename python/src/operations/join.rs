use super::BreakPolicy;
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
    pub(crate) breaks: BreakPolicy,
}

#[gen_stub_pymethods]
#[pymethods]
impl Join {
    /// Create a join with a delimiter between adjacent items of iterable.
    ///
    /// The iterable is consumed only when the join is dumped.
    /// Generators and other one-shot iterators are not cached or rewound.
    ///
    /// :param delimiter: Any value accepted by Dumper.dump(), emitted between
    ///     adjacent items but never before the first or after the last item.
    /// :param iterable: An iterable whose items are dumped recursively.
    /// :param breaks: BreakPolicy.Inherit (default) inserts no breaks itself;
    ///     surrounding wrapping rules apply. Always breaks after each delimiter.
    ///     AsNeeded breaks there when the next item's first line does not fit,
    ///     even inside NoBreakRegion. Explicit policies keep complete items and
    ///     delimiters intact except for their own explicit newlines. Delimiters
    ///     stay with preceding items, even if this exceeds the width. The first
    ///     item starts at the current position.
    ///
    /// Contexts and regions must balance inside each item or delimiter. Switching
    /// section targets or changing indentation inside a join is not
    /// allowed. Callbacks run once.
    /// Errors fail the dumper without undoing any output already written.
    #[new]
    #[pyo3(signature = (delimiter, iterable, *, breaks=BreakPolicy::Inherit))]
    fn new(delimiter: Py<PyAny>, iterable: Py<PyAny>, breaks: BreakPolicy) -> Self {
        Self {
            contents: Some((delimiter, iterable)),
            breaks,
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
