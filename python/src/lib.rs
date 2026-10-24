/// Python interface to the dumpany Rust library.
#[pyo3::pymodule]
mod dumpany {
    #[pymodule_export]
    #[allow(non_upper_case_globals)]
    const __version__: &str = env!("CARGO_PKG_VERSION");
}
