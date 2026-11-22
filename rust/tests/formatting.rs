mod common;

use common::memory_writer;
use dumpany::context::Context;
use dumpany::core::{Dump, Dumper, DumperConfig, Indentation, NewLine, Text};
use std::io::Result;

use common::{check_result, result_writer};

#[test]
fn rejects_empty_line_ending() {
    let result = Dumper::new(
        Vec::new(),
        DumperConfig {
            new_line: String::new(),
            ..DumperConfig::default()
        },
    );
    let error = result.err().expect("empty line ending must be rejected");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn text_splits_on_configured_line_ending() -> Result<()> {
    for delimiter in ["\n", "\r\n", "<br>", "§"] {
        for in_context in [false, true] {
            let mut dumper = Dumper::new(
                Vec::new(),
                DumperConfig {
                    new_line: delimiter.into(),
                    ..DumperConfig::default()
                },
            )?;
            dumper.inc_indent()?;
            if in_context {
                dumper.open_context(None, None)?;
            }
            dumper.text(format!("first{delimiter}{delimiter}last"))?;
            let output = dumper.into_inner().map_err(|error| error.into_parts().0)?;
            assert_eq!(
                output,
                format!("    first{delimiter}{delimiter}    last{delimiter}").as_bytes()
            );
        }
    }
    Ok(())
}

#[test]
fn text_preserves_lf_when_it_is_not_the_configured_delimiter() -> Result<()> {
    for delimiter in ["\r\n", "<br>"] {
        let mut dumper = Dumper::new(
            Vec::new(),
            DumperConfig {
                new_line: delimiter.into(),
                ..DumperConfig::default()
            },
        )?;
        dumper.text("first\nlast")?;
        let output = dumper.into_inner().map_err(|error| error.into_parts().0)?;
        assert_eq!(output, format!("first\nlast{delimiter}").as_bytes());
    }
    Ok(())
}

#[test]
fn trailing_configured_delimiter_does_not_add_an_extra_line() -> Result<()> {
    let mut dumper = Dumper::new(
        Vec::new(),
        DumperConfig {
            new_line: "\r\n".into(),
            ..DumperConfig::default()
        },
    )?;
    dumper.text("first\r\n")?;
    let output = dumper.into_inner().map_err(|error| error.into_parts().0)?;
    assert_eq!(output, b"first\r\n");
    Ok(())
}

#[test]
fn test_simple_text_dump() {
    fn compose_file() -> Result<()> {
        let mut d = Dumper::new(
            result_writer("test_simple_text_dump")?,
            DumperConfig {
                new_line: "\n".into(),
                indent_width: 2,
                line_width: 10,
            },
        )?;
        d.text("hello world")?;
        NewLine(Some(Indentation::Increment(1))).dump(&mut d)?;
        d.text("hello world indented")?;
        NewLine(Some(Indentation::Decrement(1))).dump(&mut d)?;
        d.text("hello world not indented")?;
        NewLine(None).dump(&mut d)?;
        Ok(())
    }

    compose_file().expect("failed to compose test file");
    check_result("test_simple_text_dump");
}

#[test]
fn test_context_text_dump() {
    fn compose_file() -> Result<()> {
        let mut d = Dumper::new(
            result_writer("test_context_text_dump")?,
            DumperConfig {
                new_line: "\n".into(),
                indent_width: 4,
                line_width: 40,
            },
        )?;
        Text("# This is a little markup\n").dump(&mut d)?;
        NewLine(Some(Indentation::Increment(1))).dump(&mut d)?;
        Text("Can i format numbers? ").dump(&mut d)?;
        Text(3.1888).dump(&mut d)?;
        NewLine(None).dump(&mut d)?;
        NewLine(None).dump(&mut d)?;
        d.open_context(Some("- "), None)?;
        Text("This is the first enumeration item and it might be very long so that it does not fit a single line\n")
            .dump(&mut d)?;
        d.close_context(None, true)?;
        d.open_context(Some("- "), None)?;
        Text("This is the second enueration itme\n").dump(&mut d)?;
        d.close_context(None, true)?;

        NewLine(Some(Indentation::Decrement(1))).dump(&mut d)?;
        Ok(())
    }

    compose_file().expect("failed to compose test file");
    check_result("test_context_text_dump");
}
#[test]
fn text_accepts_display_directly_and_inside_contexts() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default())?;
    dumper.text(42)?;
    dumper.text(format_args!(" = {}", "answer"))?;
    dumper.new_line(None)?;
    Context::new(None, None).with(&mut dumper, |d| {
        d.text(42)?;
        d.text(format_args!(" = {}", "answer"))
    })?;
    assert_eq!(dumper.get_ref(), b"42 = answer\n42 = answer\n");
    Ok(())
}
