use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use crate::core::{Dumper, DumperConfig, Target};

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
    flushes: usize,
    fail_write: bool,
    fail_flush: bool,
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

#[test]
fn finalizes_pending_lines_once_and_leaves_writer_usable() -> io::Result<()> {
    for (text, expected) in [("", ""), ("hello", "hello\n"), ("hello\n", "hello\n")] {
        let writer = Rc::new(RefCell::new(Writer::default()));
        let mut d = Dumper::new(writer.clone(), DumperConfig::default());
        if !text.is_empty() {
            d.text(text)?;
        }
        d.dump_all()?;
        d.dump_all()?;
        assert_eq!(writer.borrow().bytes, expected.as_bytes());
        assert_eq!(writer.borrow().flushes, 1);
        assert!(d.text("more").is_err());
        assert!(d.new_line(None).is_err());
        assert!(d.inc_indent().is_err());
        assert!(d.dec_indent().is_err());
        assert!(d.open_context(None, None).is_err());
        assert!(d.close_context().is_err());
        assert!(
            d.iterator(std::iter::empty::<crate::core::NewLine>())
                .is_err()
        );
        writer.borrow_mut().write_all(b"still usable")?;
    }
    Ok(())
}

#[test]
fn finalizes_nested_contexts_and_sections() -> io::Result<()> {
    let writer = Rc::new(RefCell::new(Writer::default()));
    let mut d = Dumper::new(writer.clone(), DumperConfig::default());
    d.open_context(None, Some("root"))?;
    d.open_context(None, Some("child"))?;
    d.text("body")?;
    d.with_target(Target::Prologue("child"), |d| d.text("before:"))?;
    d.with_target(Target::Epilogue("child"), |d| d.text(":after"))?;
    d.dump_all()?;
    assert_eq!(writer.borrow().bytes, b"before:body:after\n\n");
    assert_eq!(writer.borrow().flushes, 1);
    Ok(())
}

#[test]
fn target_scope_rejection_does_not_finalize() -> io::Result<()> {
    let writer = Rc::new(RefCell::new(Writer::default()));
    let mut d = Dumper::new(writer.clone(), DumperConfig::default());
    d.open_context(None, Some("root"))?;
    d.with_target(Target::Prologue("root"), |d| {
        assert_eq!(
            d.dump_all().unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        d.text("before:")
    })?;
    d.text("body")?;
    d.dump_all()?;
    assert_eq!(writer.borrow().bytes, b"before:body\n");
    Ok(())
}

#[test]
fn failures_are_terminal_and_preserve_original_error() -> io::Result<()> {
    for fail_write in [true, false] {
        let writer = Rc::new(RefCell::new(Writer {
            fail_write,
            fail_flush: !fail_write,
            ..Writer::default()
        }));
        let mut d = Dumper::new(writer.clone(), DumperConfig::default());
        d.text("hello")?;
        let error = d.dump_all().unwrap_err();
        assert_eq!(
            error.to_string(),
            if fail_write {
                "write failed"
            } else {
                "flush failed"
            }
        );
        let bytes = writer.borrow().bytes.clone();
        let flushes = writer.borrow().flushes;
        assert!(d.dump_all().is_err());
        assert!(d.text("more").is_err());
        assert_eq!(writer.borrow().bytes, bytes);
        assert_eq!(writer.borrow().flushes, flushes);
    }
    Ok(())
}

#[test]
fn ordinary_write_failure_also_prevents_finalization_retry() -> io::Result<()> {
    let writer = Rc::new(RefCell::new(Writer {
        fail_write: true,
        ..Writer::default()
    }));
    let mut d = Dumper::new(writer.clone(), DumperConfig::default());
    assert!(d.text("hello\n").is_err());
    assert!(d.dump_all().is_err());
    assert_eq!(writer.borrow().bytes, b"h");
    Ok(())
}
