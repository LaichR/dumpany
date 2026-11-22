mod common;

use common::memory_writer;
use dumpany::context::Context;
use dumpany::core::{Dumper, DumperConfig, Target};
use std::io::{Error, ErrorKind, Result};

#[test]
fn explicit_selection_appends_and_restores_main() -> Result<()> {
    let mut d = Dumper::new(Vec::new(), DumperConfig::default()).unwrap();
    d.select_main()?;
    d.open_context(None, Some("root"))?;
    d.text("body")?;
    let name = String::from("root");
    d.select_target(Target::Prologue(&name))?;
    drop(name);
    d.text("before:")?;
    assert_eq!(d.dump_all().unwrap_err().kind(), ErrorKind::InvalidInput);
    assert!(d.close_context(None, true).is_err());
    assert!(d.select_target(Target::Epilogue("root")).is_err());
    d.select_main()?;
    d.select_main()?;
    d.select_target(Target::Prologue("root"))?;
    d.text("again:")?;
    d.select_main()?;
    d.select_target(Target::Epilogue("root"))?;
    d.text(":after")?;
    d.select_main()?;
    assert!(d.get_ref().is_empty());
    d.text("tail")?;
    assert_eq!(d.into_inner().unwrap(), b"before:again:bodytail:after\n");
    Ok(())
}

#[test]
fn invalid_selection_leaves_main_unchanged() -> Result<()> {
    let mut d = Dumper::new(Vec::new(), DumperConfig::default()).unwrap();
    assert_eq!(
        d.select_target(Target::Prologue("missing"))
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
    d.open_context(None, Some("root"))?;
    assert_eq!(
        d.select_target(Target::Prologue("missing"))
            .unwrap_err()
            .kind(),
        ErrorKind::NotFound
    );
    d.text("body")?;
    assert_eq!(d.into_inner().unwrap(), b"body\n");
    Ok(())
}

#[test]
fn target_contexts_resume_across_selections() -> Result<()> {
    let mut d = Dumper::new(Vec::new(), DumperConfig::default()).unwrap();
    d.open_context(None, Some("root"))?;
    d.select_target(Target::Prologue("root"))?;
    d.open_context(Some("- "), Some("continued"))?;
    d.text("first")?;
    d.select_main()?;
    d.text("body")?;
    assert_eq!(
        d.open_context(None, Some("continued")).unwrap_err().kind(),
        ErrorKind::AlreadyExists
    );
    assert_eq!(
        d.close_context(None, true).unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert!(d.get_ref().is_empty());
    d.select_target(Target::Epilogue("root"))?;
    d.open_context(Some("+ "), None)?;
    d.text("after")?;
    d.select_main()?;
    d.select_target(Target::Prologue("root"))?;
    d.text(" second")?;
    d.close_context(None, true)?;
    d.select_main()?;
    d.select_target(Target::Epilogue("root"))?;
    d.text(" continued")?;
    d.close_context(None, true)?;
    d.select_main()?;
    assert_eq!(
        d.into_inner().unwrap(),
        b"- first second\nbody+ after continued\n\n"
    );
    Ok(())
}

#[test]
fn with_target_preserves_open_contexts_on_success_and_error() -> Result<()> {
    let mut d = Dumper::new(Vec::new(), DumperConfig::default()).unwrap();
    d.open_context(None, Some("root"))?;
    d.with_target(Target::Prologue("root"), |d| {
        d.open_context(None, None)?;
        d.text("before")
    })?;
    let error = d
        .with_target(Target::Epilogue("root"), |d| {
            d.open_context(None, None)?;
            Err::<(), _>(Error::other("body failed"))
        })
        .unwrap_err();
    assert_eq!(error.to_string(), "body failed");
    d.text("body")?;
    d.with_target(Target::Prologue("root"), |d| d.close_context(None, true))?;
    d.with_target(Target::Epilogue("root"), |d| {
        d.text("after")?;
        d.close_context(None, true)
    })?;
    assert_eq!(d.into_inner().unwrap(), b"before\nbodyafter\n\n");
    Ok(())
}

#[test]
fn selection_is_rejected_after_finalization() -> Result<()> {
    let mut d = Dumper::new(Vec::new(), DumperConfig::default()).unwrap();
    d.dump_all()?;
    assert!(d.select_main().is_err());
    assert!(d.select_target(Target::Prologue("root")).is_err());
    Ok(())
}

#[test]
fn runtime_names_and_prefixes_outlive_their_sources() -> Result<()> {
    let mut dumper = Dumper::new(Vec::new(), DumperConfig::default()).unwrap();
    dumper.open_context(None, None)?;
    {
        let name = String::from("runtime");
        let prefix = String::from("- ");
        dumper.open_context(Some(&prefix), Some(&name))?;
        dumper.text("body")?;
        dumper.close_context(None, true)?;
    }
    let name = String::from("runtime");
    assert_eq!(
        dumper.open_context(None, Some(&name)).unwrap_err().kind(),
        ErrorKind::AlreadyExists
    );
    dumper.with_target(Target::Prologue(&name), |d| d.text("before:"))?;
    dumper.with_target(Target::Epilogue(&name), |d| d.text(":after"))?;
    drop(name);
    dumper.close_context(None, true)?;
    assert_eq!(dumper.get_ref(), b"- before:body:after\n\n");

    // Root flushing releases owned names and allows an equal runtime name again.
    let name = String::from("runtime");
    dumper.open_context(None, Some(&name))?;
    drop(name);
    dumper.text("reused")?;
    assert_eq!(
        dumper.into_inner().unwrap(),
        b"- before:body:after\n\nreused\n"
    );
    Ok(())
}

#[test]
fn closed_children_remain_targetable_until_flush() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    Context::new(None, Some("root")).with(&mut dumper, |d| {
        Context::new(None, Some("child")).with(d, |d| d.text("body"))?;
        assert!(d.get_ref().is_empty());
        assert_eq!(
            d.open_context(None, Some("child")).unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );
        d.with_target(Target::Prologue("child"), |d| d.text("before:"))?;
        d.with_target(Target::Prologue("child"), |d| d.text("again:"))?;
        d.with_target(Target::Epilogue("child"), |d| d.text(":after"))?;
        d.text("tail")
    })?;
    assert_eq!(dumper.get_ref(), b"before:again:body:after\ntail\n");
    Context::new(None, Some("child")).with(&mut dumper, |_| Ok(()))?;
    assert_eq!(dumper.get_ref(), b"before:again:body:after\ntail\n\n");
    Ok(())
}

#[test]
fn flush_failure_clears_names_and_section_recorders() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(
        writer,
        DumperConfig {
            line_width: 1,
            ..DumperConfig::default()
        },
    )
    .unwrap();
    let error = Context::new(Some("too long"), Some("root"))
        .with(&mut dumper, |d| {
            d.with_target(Target::Prologue("root"), |d| d.text("before"))?;
            d.with_target(Target::Epilogue("root"), |d| d.text("after"))
        })
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(
        dumper
            .with_target(Target::Prologue("root"), |_| Ok(()))
            .is_err()
    );
    Context::new(None, Some("root")).with(&mut dumper, |_| Ok(()))?;
    assert_eq!(dumper.get_ref(), b"too long\n\n");
    Ok(())
}

#[test]
fn section_selection_is_not_reentrant_and_sections_are_independent() -> Result<()> {
    for outer in [Target::Prologue("root"), Target::Epilogue("root")] {
        let writer = memory_writer();
        let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
        Context::new(None, Some("root")).with(&mut dumper, |d| {
            d.with_target(outer, |d| {
                for inner in [Target::Prologue("root"), Target::Epilogue("root")] {
                    let mut called = false;
                    let error = d
                        .with_target(inner, |_| {
                            called = true;
                            Ok(())
                        })
                        .unwrap_err();
                    assert_eq!(error.kind(), ErrorKind::InvalidInput);
                    assert!(!called);
                }
                d.text("section")
            })?;
            d.text("body")
        })?;
        let expected: &[u8] = match outer {
            Target::Prologue(_) => b"sectionbody\n",
            Target::Epilogue(_) => b"bodysection\n",
        };
        assert_eq!(dumper.get_ref(), expected);
    }
    Ok(())
}

#[test]
fn section_contexts_cannot_select_another_target() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    Context::new(None, Some("root")).with(&mut dumper, |d| {
        d.with_target(Target::Prologue("root"), |d| {
            d.text("a")?;
            Context::new(None, Some("child")).with(d, |d| {
                d.text("b")?;
                let error = d
                    .with_target(Target::Prologue("child"), |d| d.text("c"))
                    .unwrap_err();
                assert_eq!(error.kind(), ErrorKind::InvalidInput);
                Ok(())
            })?;
            d.text("d")
        })?;
        d.text("body")
    })?;
    assert_eq!(dumper.get_ref(), b"ab\ndbody\n");
    Ok(())
}

#[test]
fn sections_buffer_newlines_and_allow_nested_contexts() -> Result<()> {
    use dumpany::core::Target;
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    Context::new(None, Some("root")).with(&mut dumper, |d| {
        d.text("body")?;
        d.with_target(Target::Prologue("root"), |d| {
            assert!(d.close_context(None, true).is_err());
            Context::new(None, Some("header")).with(d, |d| d.text("header"))?;
            d.text("before")?;
            d.new_line(None)
        })?;
        assert!(d.get_ref().is_empty());
        d.with_target(Target::Epilogue("root"), |d| d.text("after"))
    })?;
    assert_eq!(dumper.get_ref(), b"header\nbefore\nbodyafter\n");
    Ok(())
}

#[test]
fn scoped_targets_restore_output_and_render_sections_in_order() -> Result<()> {
    use dumpany::core::Target;
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    assert!(
        dumper
            .with_target(Target::Prologue("root"), |_| Ok(()))
            .is_err()
    );
    Context::new(None, Some("root")).with(&mut dumper, |d| {
        d.text("body")?;
        d.with_target(Target::Epilogue("root"), |d| d.text("after"))?;
        d.with_target(Target::Prologue("root"), |d| {
            d.text("before")?;
            let error = d
                .with_target(Target::Epilogue("root"), |d| {
                    d.text("!")?;
                    Err::<(), _>(Error::other("failed"))
                })
                .unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidInput);
            d.text(":")
        })?;
        let error = d
            .with_target(Target::Epilogue("root"), |d| {
                d.text("!")?;
                Err::<(), _>(Error::other("failed"))
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "failed");
        assert!(
            d.with_target(Target::Prologue("missing"), |_| Ok(()))
                .is_err()
        );
        d.text("tail")
    })?;
    assert_eq!(dumper.get_ref(), b"before:bodytailafter!\n");
    // The flushed namespace no longer prevents reuse of the name.
    Context::new(None, Some("root")).with(&mut dumper, |_| Ok(()))?;
    Ok(())
}
