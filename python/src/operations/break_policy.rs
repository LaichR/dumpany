use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyclass_enum;

#[gen_stub_pyclass_enum]
/// Control explicit line breaks inserted between Join items.
#[pyclass(module = "dumpany", eq, eq_int, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BreakPolicy {
    /// Insert no breaks of its own; items and delimiters follow surrounding wrapping rules.
    Inherit,
    /// Break after every delimiter unless already at the start of a line.
    Always,
    /// Break after a delimiter if the next item's first explicit line does not fit.
    /// Complete items and delimiters keep their own explicit breaks but do not wrap automatically.
    AsNeeded,
}

impl From<BreakPolicy> for dumpany_rs::core::BreakPolicy {
    fn from(value: BreakPolicy) -> Self {
        match value {
            BreakPolicy::Inherit => Self::Inherit,
            BreakPolicy::Always => Self::Always,
            BreakPolicy::AsNeeded => Self::AsNeeded,
        }
    }
}
