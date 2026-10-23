mod common;

use common::memory_writer;
use dumpany::context::Context;
use dumpany::core::{Dumper, DumperConfig};
use std::io::{Error, ErrorKind, Result};

#[test]
fn unnamed_contexts_remain_usable_without_sections() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
    Context::new(None, None).with(&mut dumper, |d| {
        for text in ["first", "second"] {
            Context::new(None, None).with(d, |d| d.text(text))?;
        }
        Ok(())
    })?;
    assert_eq!(&*writer.borrow(), b"first\nsecond\n\n");
    Ok(())
}

#[test]
fn scoped_contexts_nest_and_return_values() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
    let value = Context::new(Some("- "), None).with(&mut dumper, |d| {
        Context::new(Some("* "), None).with(d, |d| d.text("hello"))?;
        Ok(42)
    })?;
    assert_eq!(value, 42);
    assert_eq!(&*writer.borrow(), b"- * hello\n\n");
    Ok(())
}

#[test]
fn scoped_context_closes_after_body_error() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
    let error = Context::new(Some("- "), None)
        .with(&mut dumper, |_| Err::<(), _>(Error::other("body failed")))
        .unwrap_err();
    assert_eq!(error.to_string(), "body failed");
    dumper.text("after")?;
    dumper.new_line(None)?;
    assert_eq!(&*writer.borrow(), b"-\nafter\n");
    Ok(())
}

#[test]
fn scoped_context_reports_close_errors_and_prefers_body_errors() {
    let writer = memory_writer();
    let mut dumper = Dumper::new(
        writer,
        DumperConfig {
            line_width: 1,
            ..DumperConfig::default()
        },
    );
    let context = Context::new(Some("too long"), None);
    let error = context.with(&mut dumper, |_| Ok(())).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    let error = context
        .with(&mut dumper, |_| Err::<(), _>(Error::other("body failed")))
        .unwrap_err();
    assert_eq!(error.to_string(), "body failed");
}
