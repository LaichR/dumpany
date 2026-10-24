/// Python interface to the dumpany Rust library.
mod dumper;
pub mod writer;

#[pyo3::pymodule]
mod dumpany {
    #[pymodule_export]
    use crate::dumper::PyDumper;

    #[pymodule_export]
    #[allow(non_upper_case_globals)]
    const __version__: &str = env!("CARGO_PKG_VERSION");
}
