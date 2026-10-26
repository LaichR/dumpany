/// Python interface to the dumpany Rust library.
mod dumper;
mod operations;
pub mod writer;

#[pyo3::pymodule]
mod dumpany {
    #[pymodule_export]
    use crate::dumper::PyDumper;

    #[pymodule_export]
    use crate::operations::NewLine;

    #[pymodule_export]
    use crate::operations::Join;

    #[pymodule_export]
    use crate::operations::PyContext;

    #[pymodule_export]
    use crate::operations::Prologue;

    #[pymodule_export]
    use crate::operations::Epilogue;

    #[pymodule_export]
    #[allow(non_upper_case_globals)]
    const __version__: &str = env!("CARGO_PKG_VERSION");
}
