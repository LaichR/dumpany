use super::*;
use dumpany::core::{Dumper, DumperConfig};
use pyo3::exceptions::PyValueError;
use std::ffi::CString;

fn stream(code: &str) -> Py<PyAny> {
    Python::initialize();
    Python::attach(|py| {
        let code = CString::new(code).unwrap();
        PyModule::from_code(py, &code, c"test_writer.py", c"test_writer")
            .unwrap()
            .getattr("stream")
            .unwrap()
            .unbind()
    })
}

#[test]
fn dumper_writes_unicode_and_leaves_stream_open() {
    let stream = stream("import io\nstream = io.StringIO()");
    let writer = Python::attach(|py| PythonWriter::new(stream.clone_ref(py)));
    let mut dumper = Dumper::new(writer, DumperConfig::default());
    dumper.text("héllo 世界 🦀").unwrap();
    dumper.dump_all().unwrap();
    drop(dumper);
    Python::attach(|py| {
        let stream = stream.bind(py);
        assert_eq!(
            stream
                .call_method0("getvalue")
                .unwrap()
                .extract::<String>()
                .unwrap(),
            "héllo 世界 🦀\n"
        );
        assert!(!stream.getattr("closed").unwrap().extract::<bool>().unwrap());
    });
}

#[test]
fn short_writes_translate_character_counts_to_bytes() {
    let stream = stream(
        "class Stream:\n    text = ''\n    def write(self, text):\n        self.text += text[:1]\n        return 1\nstream = Stream()",
    );
    let mut writer = Python::attach(|py| PythonWriter::new(stream.clone_ref(py)));
    assert_eq!(writer.write("é🦀".as_bytes()).unwrap(), 2);
    writer.write_all("🦀中".as_bytes()).unwrap();
    Python::attach(|py| {
        assert_eq!(
            stream
                .bind(py)
                .getattr("text")
                .unwrap()
                .extract::<String>()
                .unwrap(),
            "é🦀中"
        );
    });
}

#[test]
fn flush_is_forwarded() {
    let stream = stream(
        "class Stream:\n    flushed = False\n    def flush(self):\n        self.flushed = True\nstream = Stream()",
    );
    let mut writer = Python::attach(|py| PythonWriter::new(stream.clone_ref(py)));
    writer.flush().unwrap();
    Python::attach(|py| {
        assert!(
            stream
                .bind(py)
                .getattr("flushed")
                .unwrap()
                .extract::<bool>()
                .unwrap()
        );
    });
}

#[test]
fn python_exceptions_are_preserved() {
    let mut writer = PythonWriter::new(stream(
        "class Stream:\n    def write(self, text):\n        raise ValueError('write failed')\n    def flush(self):\n        raise ValueError('flush failed')\nstream = Stream()",
    ));
    for error in [
        writer.write(b"text").unwrap_err(),
        writer.flush().unwrap_err(),
    ] {
        let exception = error.get_ref().unwrap().downcast_ref::<PyErr>().unwrap();
        Python::attach(|py| assert!(exception.is_instance_of::<PyValueError>(py)));
    }
}

#[test]
fn invalid_utf8_is_rejected_before_calling_python() {
    let mut writer = PythonWriter::new(stream("stream = None"));
    assert_eq!(
        writer.write(&[0xff]).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        writer.write(&[0xc3]).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(writer.write(b"").unwrap(), 0);
}

#[test]
fn invalid_write_results_are_rejected() {
    for result in ["None", "-1", "'one'", "10"] {
        let mut writer = PythonWriter::new(stream(&format!(
            "class Stream:\n    def write(self, text):\n        return {result}\nstream = Stream()"
        )));
        assert!(writer.write(b"x").is_err(), "accepted {result}");
    }
}

#[test]
fn zero_write_becomes_write_zero() {
    let mut writer = PythonWriter::new(stream(
        "class Stream:\n    def write(self, text):\n        return 0\nstream = Stream()",
    ));
    assert_eq!(
        writer.write_all(b"text").unwrap_err().kind(),
        io::ErrorKind::WriteZero
    );
}
