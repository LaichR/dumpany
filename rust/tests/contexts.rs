mod common;

use common::memory_writer;
use dumpany::context::Context;
use dumpany::core::{Dumper, DumperConfig, Indentation};
use std::io::{Error, ErrorKind, Result};

#[test]
fn closing_context_adjusts_indentation_after_its_final_line() -> Result<()> {
    for nested in [false, true] {
        let mut dumper = Dumper::new(Vec::new(), DumperConfig::default());
        if nested {
            dumper.open_context(None, None)?;
        }
        dumper.open_context(None, None)?;
        dumper.text("heading")?;
        dumper.close_context_with_indentation(Some(Indentation::Increment(2)))?;
        dumper.open_context(None, None)?;
        dumper.text("body")?;
        dumper.close_context_with_indentation(Some(Indentation::Decrement(255)))?;
        dumper.text("tail")?;
        if nested {
            dumper.close_context()?;
        }
        assert_eq!(
            dumper.into_inner().unwrap(),
            b"heading\n        body\ntail\n"
        );
    }
    Ok(())
}

#[test]
fn scoped_context_accepts_runtime_metadata() -> Result<()> {
    let prefix = String::from("- ");
    let name = String::from("runtime");
    let mut dumper = Dumper::new(Vec::new(), DumperConfig::default());
    Context::new(Some(&prefix), Some(&name)).with(&mut dumper, |d| d.text("hello"))?;
    assert_eq!(dumper.into_inner().unwrap(), b"- hello\n");
    Ok(())
}

#[test]
fn recorder_encodes_runtime_metadata_before_sources_are_dropped() -> Result<()> {
    use dumpany::recorder::Recorder;

    let mut recorder = {
        let prefix = String::from("- ");
        let name = String::from("runtime");
        Recorder::new(Some(&prefix), Some(&name))?
    };
    {
        let prefix = String::from("* ");
        let name = String::from("child");
        recorder.open_context(Some(&prefix), Some(&name))?;
    }
    recorder.text("hello")?;
    recorder.close_context()?;
    recorder.close_context()?;

    let mut expected = Recorder::new(Some("- "), Some("runtime"))?;
    expected.open_context(Some("* "), Some("child"))?;
    expected.text("hello")?;
    expected.close_context()?;
    expected.close_context()?;
    assert_eq!(recorder.encoded_content(), expected.encoded_content());
    Ok(())
}

#[test]
fn unnamed_contexts_remain_usable_without_sections() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default());
    Context::new(None, None).with(&mut dumper, |d| {
        for text in ["first", "second"] {
            Context::new(None, None).with(d, |d| d.text(text))?;
        }
        Ok(())
    })?;
    assert_eq!(dumper.get_ref(), b"first\nsecond\n\n");
    Ok(())
}

#[test]
fn scoped_contexts_nest_and_return_values() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default());
    let value = Context::new(Some("- "), None).with(&mut dumper, |d| {
        Context::new(Some("* "), None).with(d, |d| d.text("hello"))?;
        Ok(42)
    })?;
    assert_eq!(value, 42);
    assert_eq!(dumper.get_ref(), b"- * hello\n\n");
    Ok(())
}

#[test]
fn scoped_context_closes_after_body_error() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default());
    let error = Context::new(Some("- "), None)
        .with(&mut dumper, |_| Err::<(), _>(Error::other("body failed")))
        .unwrap_err();
    assert_eq!(error.to_string(), "body failed");
    dumper.text("after")?;
    dumper.new_line(None)?;
    assert_eq!(dumper.get_ref(), b"-\nafter\n");
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
