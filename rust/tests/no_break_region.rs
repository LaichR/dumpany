use dumpany::context::Context;
use dumpany::core::{Dump, Dumper, DumperConfig, NoBreakRegion, Target, Text};
use std::io::{Error, ErrorKind, Result};

fn dumper(width: usize) -> Result<Dumper<Vec<u8>>> {
    Dumper::new(
        Vec::new(),
        DumperConfig {
            line_width: width,
            ..DumperConfig::default()
        },
    )
}

#[test]
fn composed_iterator_wraps_as_one_unit_including_when_recorded() -> Result<()> {
    for recorded in [false, true] {
        for (prefix, expected) in [("x ", "x a + b\n"), ("long ", "long\na + b\n")] {
            let mut out = dumper(8)?;
            if recorded {
                out.open_context(None, None)?;
            }
            out.text(prefix)?;
            let mut calls = 0;
            out.no_break_region(|out| {
                calls += 1;
                out.iterator([Text("a"), Text(" + "), Text("b")])
            })?;
            assert_eq!(calls, 1);
            assert_eq!(out.into_inner().unwrap(), expected.as_bytes());
        }
    }
    Ok(())
}

#[test]
fn oversized_and_nested_regions_never_wrap_automatically() -> Result<()> {
    let mut out = dumper(4)?;
    out.no_break_region(|out| {
        out.text("first ")?;
        NoBreakRegion(Text("second ")).dump(out)?;
        out.no_break("third")
    })?;
    assert_eq!(out.into_inner().unwrap(), b"first second third\n");
    Ok(())
}

#[test]
fn explicit_newlines_and_context_alignment_are_preserved() -> Result<()> {
    let mut out = dumper(8)?;
    Context::new(Some("- "), None).with(&mut out, |out| {
        out.text("head ")?;
        out.no_break_region(|out| {
            out.text("a + b")?;
            out.new_line(None)?;
            let mut context = Context::new(Some("call("), None);
            context.new_line = false;
            context.with(out, |out| out.text("a, b, c, d"))?;
            out.text(")")
        })
    })?;
    assert_eq!(
        out.into_inner().unwrap(),
        b"- head\n  a + b\n  call(a, b, c, d)\n"
    );
    Ok(())
}

#[test]
fn unicode_and_first_explicit_line_determine_fit() -> Result<()> {
    let mut out = dumper(6)?;
    out.text("é ")?;
    out.no_break_region(|out| out.text("界 + x\nsecond line is long"))?;
    assert_eq!(
        out.into_inner().unwrap(),
        "é\n界 + x\nsecond line is long\n".as_bytes()
    );
    Ok(())
}

#[test]
fn callback_error_is_preserved_and_permanently_fails_the_dumper() -> Result<()> {
    let mut out = dumper(30)?;
    out.open_context(None, Some("outer"))?;
    let error = out
        .no_break_region::<()>(|out| {
            out.text("partial ")?;
            Err(Error::other("body failed"))
        })
        .unwrap_err();
    assert_eq!(error.to_string(), "body failed");
    assert!(out.get_ref().is_empty());
    assert!(out.text("tail").is_err());
    assert!(out.end_no_break_region().is_err());
    assert!(out.close_context(None, true).is_err());
    assert!(out.dump_all().is_err());
    assert!(out.into_inner().unwrap_err().into_inner().is_empty());
    Ok(())
}

#[test]
fn unmatched_region_boundaries_permanently_fail_the_dumper() -> Result<()> {
    let mut out = dumper(30)?;
    assert_eq!(
        out.end_no_break_region().unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert!(out.text("tail").is_err());

    let mut out = dumper(30)?;
    assert_eq!(
        out.no_break_region(|out| out.open_context(None, Some("unclosed")))
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
    assert!(out.text("tail").is_err());
    assert!(out.dump_all().is_err());
    assert!(out.get_ref().is_empty());
    Ok(())
}

#[test]
fn a_region_cannot_close_an_enclosing_context() -> Result<()> {
    let mut out = dumper(30)?;
    out.open_context(None, Some("outer"))?;
    let error = out
        .no_break_region(|out| out.close_context(None, true))
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(out.dump_all().is_err());
    Ok(())
}

#[test]
fn region_in_target_uses_the_same_wrapping_rules() -> Result<()> {
    let mut out = dumper(5)?;
    out.open_context(None, Some("root"))?;
    out.text("body")?;
    out.with_target(Target::Prologue("root"), |out| {
        out.no_break_region(|out| out.text("a b c d e"))?;
        out.new_line(None)
    })?;
    out.close_context(None, true)?;
    assert_eq!(out.into_inner().unwrap(), b"a b c d e\nbody\n");
    Ok(())
}

#[test]
fn retained_width_includes_sections_supplied_after_capture() -> Result<()> {
    for section_newline in [false, true] {
        let mut out = dumper(10)?;
        out.open_context(None, Some("root"))?;
        out.text("head ")?;
        out.no_break_region(|out| {
            let mut context = Context::new(None, Some("item"));
            context.new_line = false;
            context.with(out, |out| out.text("value"))
        })?;
        out.with_target(Target::Prologue("item"), |out| {
            out.text("pre ")?;
            if section_newline {
                out.new_line(None)?;
            }
            Ok(())
        })?;
        out.with_target(Target::Epilogue("item"), |out| out.text(" end"))?;
        out.close_context(None, true)?;
        let expected = if section_newline {
            b"head pre\n     value end\n".as_slice()
        } else {
            b"head\npre value end\n".as_slice()
        };
        assert_eq!(out.into_inner().unwrap(), expected);
    }
    Ok(())
}

#[test]
fn captured_width_uses_configured_line_endings_and_survives_nested_capture() -> Result<()> {
    let mut out = Dumper::new(
        Vec::new(),
        DumperConfig {
            line_width: 8,
            new_line: "<br>".into(),
            ..DumperConfig::default()
        },
    )?;
    out.open_context(None, None)?;
    out.text("head ")?;
    out.no_break_region(|out| {
        out.no_break_region(|out| out.text("é + 界<br>long continuation"))?;
        out.text(" tail")
    })?;
    out.close_context(None, true)?;
    assert_eq!(
        out.into_inner().unwrap(),
        "head<br>é + 界<br>long continuation tail<br>".as_bytes()
    );
    Ok(())
}
