mod common;

use common::Writer;
use dumpany::core::{Dumper, DumperConfig, Target};
use std::io::{self, Write};

#[test]
fn finalizes_fragments_without_adding_a_newline() -> io::Result<()> {
    for ending in ["\n", "\r\n", "<br>"] {
        for (text, expected) in [
            (String::new(), String::new()),
            ("hello".into(), "hello".into()),
            (format!("hello{ending}"), format!("hello{ending}")),
        ] {
            let mut out = Dumper::new(
                Writer::default(),
                DumperConfig {
                    new_line: ending.into(),
                    ..DumperConfig::default()
                },
            )?;
            out.text(text)?;
            out.dump_all_with_new_line(false)?;
            out.dump_all()?;
            assert_eq!(out.get_ref().bytes, expected.as_bytes());
            assert_eq!(out.get_ref().flushes, 1);
            assert!(out.text("more").is_err());
            let mut writer = out.into_inner().unwrap();
            writer.write_all(b" suffix")?;
            assert_eq!(writer.bytes, format!("{expected} suffix").as_bytes());
        }
    }
    Ok(())
}

#[test]
fn fragment_finalization_closes_pending_contexts_without_extra_breaks() -> io::Result<()> {
    let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
    out.open_context(Some("("), Some("root"))?;
    out.open_context(Some("["), Some("child"))?;
    out.text("body")?;
    out.with_target(Target::Prologue("child"), |out| out.text("before:"))?;
    out.with_target(Target::Epilogue("child"), |out| out.text(":after]"))?;
    out.with_target(Target::Epilogue("root"), |out| out.text(")"))?;
    assert_eq!(
        out.into_inner_with_new_line(false).unwrap(),
        b"([before:body:after])"
    );
    Ok(())
}

#[test]
fn fragment_finalization_preserves_explicit_context_closing_breaks() -> io::Result<()> {
    let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
    out.open_context(None, None)?;
    out.text("body")?;
    out.close_context(None, true)?;
    assert_eq!(out.into_inner_with_new_line(false).unwrap(), b"body\n");
    Ok(())
}

#[test]
fn fragment_finalization_preserves_write_and_flush_errors() -> io::Result<()> {
    for fail_write in [true, false] {
        let writer = Writer {
            fail_write,
            fail_flush: !fail_write,
            ..Writer::default()
        };
        let mut out = Dumper::new(writer, DumperConfig::default())?;
        out.text("fragment")?;
        let error = out
            .into_inner_with_new_line(false)
            .err()
            .expect("writer must fail");
        assert_eq!(
            error.error().to_string(),
            if fail_write {
                "write failed"
            } else {
                "flush failed"
            }
        );
    }
    Ok(())
}

#[test]
fn finalizes_pending_lines_once_and_leaves_writer_usable() -> io::Result<()> {
    for (text, expected) in [("", ""), ("hello", "hello\n"), ("hello\n", "hello\n")] {
        let writer = Writer::default();
        let mut d = Dumper::new(writer, DumperConfig::default()).unwrap();
        if !text.is_empty() {
            d.text(text)?;
        }
        d.dump_all()?;
        d.dump_all()?;
        assert_eq!(d.get_ref().bytes, expected.as_bytes());
        assert_eq!(d.get_ref().flushes, 1);
        assert!(d.text("more").is_err());
        assert!(d.new_line(None).is_err());
        assert!(d.inc_indent().is_err());
        assert!(d.dec_indent().is_err());
        assert!(d.open_context(None, None).is_err());
        assert!(d.close_context(None, true).is_err());
        assert!(
            d.iterator(std::iter::empty::<dumpany::core::NewLine>())
                .is_err()
        );
        let mut writer = d.into_inner().unwrap();
        writer.write_all(b"still usable")?;
    }
    Ok(())
}

#[test]
fn finalizes_nested_contexts_and_sections() -> io::Result<()> {
    let writer = Writer::default();
    let mut d = Dumper::new(writer, DumperConfig::default()).unwrap();
    d.open_context(None, Some("root"))?;
    d.open_context(None, Some("child"))?;
    d.text("body")?;
    d.with_target(Target::Prologue("child"), |d| d.text("before:"))?;
    d.with_target(Target::Epilogue("child"), |d| d.text(":after"))?;
    d.dump_all()?;
    assert_eq!(d.get_ref().bytes, b"before:body:after\n\n");
    assert_eq!(d.get_ref().flushes, 1);
    Ok(())
}

#[test]
fn target_scope_rejection_does_not_finalize() -> io::Result<()> {
    let writer = Writer::default();
    let mut d = Dumper::new(writer, DumperConfig::default()).unwrap();
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
    assert_eq!(d.get_ref().bytes, b"before:body\n");
    Ok(())
}

#[test]
fn failures_are_terminal_and_preserve_original_error() -> io::Result<()> {
    for fail_write in [true, false] {
        let writer = Writer {
            fail_write,
            fail_flush: !fail_write,
            ..Writer::default()
        };
        let mut d = Dumper::new(writer, DumperConfig::default()).unwrap();
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
        let bytes = d.get_ref().bytes.clone();
        let flushes = d.get_ref().flushes;
        assert!(d.dump_all().is_err());
        assert!(d.text("more").is_err());
        assert_eq!(d.get_ref().bytes, bytes);
        assert_eq!(d.get_ref().flushes, flushes);
    }
    Ok(())
}

#[test]
fn ordinary_write_failure_also_prevents_finalization_retry() -> io::Result<()> {
    let writer = Writer {
        fail_write: true,
        ..Writer::default()
    };
    let mut d = Dumper::new(writer, DumperConfig::default()).unwrap();
    assert!(d.text("hello\n").is_err());
    assert!(d.dump_all().is_err());
    assert_eq!(d.get_ref().bytes, b"h");
    Ok(())
}

#[test]
fn into_inner_finalizes_pending_output_and_nested_contexts() -> io::Result<()> {
    let mut d = Dumper::new(Writer::default(), DumperConfig::default()).unwrap();
    d.open_context(None, Some("root"))?;
    d.open_context(None, Some("child"))?;
    d.text("body")?;
    d.with_target(Target::Prologue("child"), |d| d.text("before:"))?;
    d.with_target(Target::Epilogue("child"), |d| d.text(":after"))?;
    let writer = d.into_inner().unwrap();
    assert_eq!(writer.bytes, b"before:body:after\n\n");
    assert_eq!(writer.flushes, 1);
    Ok(())
}

#[test]
fn into_inner_flushes_empty_and_pending_dumps() -> io::Result<()> {
    for (text, expected) in [("", ""), ("hello", "hello\n")] {
        let mut d = Dumper::new(Writer::default(), DumperConfig::default()).unwrap();
        d.text(text)?;
        let writer = d.into_inner().unwrap();
        assert_eq!(writer.bytes, expected.as_bytes());
        assert_eq!(writer.flushes, 1);
    }
    Ok(())
}

#[test]
fn into_inner_preserves_writer_and_error_on_failure() -> io::Result<()> {
    for fail_write in [true, false] {
        let mut d = Dumper::new(
            Writer {
                fail_write,
                fail_flush: !fail_write,
                ..Writer::default()
            },
            DumperConfig::default(),
        )
        .unwrap();
        d.text("hello")?;
        let error = d.into_inner().err().unwrap();
        assert_eq!(
            error.error().kind(),
            if fail_write {
                io::ErrorKind::BrokenPipe
            } else {
                io::ErrorKind::Other
            }
        );
        assert_eq!(
            error.to_string(),
            if fail_write {
                "write failed"
            } else {
                "flush failed"
            }
        );
        assert!(std::error::Error::source(&error).is_some());
        let (error, mut writer) = error.into_parts();
        assert_eq!(
            error.to_string(),
            if fail_write {
                "write failed"
            } else {
                "flush failed"
            }
        );
        assert_eq!(
            writer.bytes,
            if fail_write {
                b"h".as_slice()
            } else {
                b"hello\n".as_slice()
            }
        );
        assert_eq!(writer.flushes, usize::from(!fail_write));
        writer.fail_write = false;
        writer.write_all(b"recovered")?;
    }
    Ok(())
}

#[test]
fn borrowed_writer_remains_available_after_finalization() -> io::Result<()> {
    let mut writer = Vec::new();
    let mut d = Dumper::new(&mut writer, DumperConfig::default()).unwrap();
    d.text("hello")?;
    d.into_inner().unwrap().write_all(b"after")?;
    assert_eq!(writer, b"hello\nafter");
    Ok(())
}

#[test]
fn finalization_error_into_inner_recovers_writer_without_retry() -> io::Result<()> {
    let mut d = Dumper::new(
        Writer {
            fail_flush: true,
            ..Writer::default()
        },
        DumperConfig::default(),
    )
    .unwrap();
    d.text("hello")?;
    let writer = d.into_inner().err().unwrap().into_inner();
    assert_eq!(writer.bytes, b"hello\n");
    assert_eq!(writer.flushes, 1);
    Ok(())
}

#[test]
fn owned_dumper_can_move_between_threads() {
    fn assert_send<T: Send>() {}
    assert_send::<Dumper<Vec<u8>>>();
    let mut d = Dumper::new(Vec::new(), DumperConfig::default()).unwrap();
    d.text("hello").unwrap();
    let writer = std::thread::spawn(move || d.into_inner().unwrap())
        .join()
        .unwrap();
    assert_eq!(writer, b"hello\n");
}
