#![allow(dead_code)]

use std::{
    cell::RefCell,
    fs,
    io::{self, ErrorKind, Result, Write},
    path::Path,
    rc::Rc,
};

pub fn memory_writer() -> Rc<RefCell<Vec<u8>>> {
    Rc::new(RefCell::new(Vec::new()))
}

pub fn result_writer(test_name: &str) -> Result<Rc<RefCell<fs::File>>> {
    let test_data = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");
    let result_dir = test_data.join("result");
    fs::create_dir_all(&result_dir)?;
    let file = fs::File::create(result_dir.join(format!("{}.txt", test_name)))?;
    Ok(Rc::new(RefCell::new(file)))
}

pub fn check_result(test_name: &str) {
    let test_data = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");
    let filename = format!("{}.txt", test_name);
    let result_path = test_data.join("result").join(&filename);

    let check_path = test_data.join("check").join(&filename);
    match fs::read(&check_path) {
        Ok(reference) => {
            let result = fs::read(&result_path).expect("failed to read test result");
            assert_eq!(
                result,
                reference,
                "test result {} differs from reference {}",
                result_path.display(),
                check_path.display()
            );
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => panic!(
            "failed to read reference {}: {}",
            check_path.display(),
            error
        ),
    }
}

#[derive(Default)]
pub struct Writer {
    pub bytes: Vec<u8>,
    pub flushes: usize,
    pub fail_write: bool,
    pub fail_flush: bool,
}

impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.fail_write {
            // Simulate a partial write before a subsequent failure.
            if self.bytes.is_empty() && !bytes.is_empty() {
                self.bytes.push(bytes[0]);
                return Ok(1);
            }
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "write failed"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        if self.fail_flush {
            Err(io::Error::other("flush failed"))
        } else {
            Ok(())
        }
    }
}
