mod common;

use common::memory_writer;
use dumpany::context::Context;
use dumpany::core::{Dumper, DumperConfig, Indentation};
use std::io::{Error, ErrorKind, Result};

#[test]
fn unmatched_close_is_rejected_without_changing_output_or_indentation() -> Result<()> {
    for new_line in [false, true] {
        for change in [
            None,
            Some(Indentation::Increment(1)),
            Some(Indentation::Decrement(1)),
        ] {
            let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
            out.inc_indent()?;
            out.text("before")?;
            assert_eq!(
                out.close_context(change, new_line).unwrap_err().kind(),
                ErrorKind::InvalidInput
            );
            assert!(out.get_ref().is_empty());
            out.text(" after")?;
            out.new_line(None)?;
            out.open_context(None, None)?;
            out.text("inside")?;
            out.close_context(None, true)?;
            assert_eq!(
                out.close_context(change, new_line).unwrap_err().kind(),
                ErrorKind::InvalidInput
            );
            out.text("tail")?;
            assert_eq!(
                out.into_inner().unwrap(),
                b"    before after\n    inside\n    tail\n"
            );
        }
    }
    Ok(())
}

#[test]
fn inline_context_restores_parent_alignment_and_keeps_epilogue_inline() -> Result<()> {
    use dumpany::core::Target;
    let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
    Context::new(Some("- "), None).with(&mut out, |out| {
        let mut inner = Context::new(Some("call("), Some("call"));
        inner.new_line = false;
        inner.with(out, |out| {
            out.text("first")?;
            out.new_line(None)?;
            out.text("second")?;
            out.with_target(Target::Epilogue("call"), |out| out.text(")"))
        })?;
        out.text(";")?;
        out.new_line(None)?;
        out.text("tail")
    })?;
    assert_eq!(
        out.into_inner().unwrap(),
        b"- call(first\n       second);\n  tail\n"
    );
    Ok(())
}

#[test]
fn indentation_change_forces_newline_even_when_inline_was_requested() -> Result<()> {
    for change in [
        Indentation::Increment(0),
        Indentation::Increment(1),
        Indentation::Decrement(0),
    ] {
        let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
        out.open_context(None, None)?;
        out.text("first")?;
        out.close_context(Some(change), false)?;
        out.text("second")?;
        let expected = if change == Indentation::Increment(1) {
            "first\n    second\n"
        } else {
            "first\nsecond\n"
        };
        assert_eq!(out.into_inner().unwrap(), expected.as_bytes());
    }
    Ok(())
}

#[test]
fn manual_root_context_can_continue_inline() -> Result<()> {
    let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
    out.open_context(Some("("), None)?;
    out.text("value")?;
    out.close_context(None, false)?;
    out.text(")")?;
    assert_eq!(out.into_inner().unwrap(), b"(value)\n");
    Ok(())
}

#[test]
fn nested_context_preserves_alignment_after_explicit_newlines() -> Result<()> {
    let mut dumper = Dumper::new(Vec::new(), DumperConfig::default())?;
    Context::new(Some("- "), None).with(&mut dumper, |d| {
        d.text("heading")?;
        d.new_line(None)?;
        Context::new(Some("* "), None).with(d, |d| {
            d.text("first")?;
            d.new_line(None)?;
            d.text("second")
        })?;
        d.text("tail")
    })?;
    assert_eq!(
        String::from_utf8(dumper.into_inner().unwrap()).unwrap(),
        "- heading\n  * first\n    second\n  tail\n"
    );
    Ok(())
}

#[test]
fn closing_context_adjusts_indentation_after_its_final_line() -> Result<()> {
    for nested in [false, true] {
        let mut dumper = Dumper::new(Vec::new(), DumperConfig::default())?;
        if nested {
            dumper.open_context(None, None)?;
        }
        dumper.open_context(None, None)?;
        dumper.text("heading")?;
        dumper.close_context(Some(Indentation::Increment(2)), true)?;
        dumper.open_context(None, None)?;
        dumper.text("body")?;
        dumper.close_context(Some(Indentation::Decrement(255)), true)?;
        dumper.text("tail")?;
        if nested {
            dumper.close_context(None, true)?;
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
    let mut dumper = Dumper::new(Vec::new(), DumperConfig::default())?;
    Context::new(Some(&prefix), Some(&name)).with(&mut dumper, |d| d.text("hello"))?;
    assert_eq!(dumper.into_inner().unwrap(), b"- hello\n");
    Ok(())
}

#[test]
fn unnamed_contexts_remain_usable_without_sections() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
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
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
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
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
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
    )
    .unwrap();
    let context = Context::new(Some("too long"), None);
    let error = context.with(&mut dumper, |_| Ok(())).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    let error = context
        .with(&mut dumper, |_| Err::<(), _>(Error::other("body failed")))
        .unwrap_err();
    assert_eq!(error.to_string(), "body failed");
}
