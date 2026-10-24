mod common;

use common::memory_writer;
use dumpany::context::Context;
use dumpany::core::{Dump, Dumper, DumperConfig, Indentation, NewLine, Text};
use std::io::Result;

use common::{check_result, result_writer};

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
        );
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
        );
        Text("# This is a little markup\n").dump(&mut d)?;
        NewLine(Some(Indentation::Increment(1))).dump(&mut d)?;
        Text("Can i format numbers? ").dump(&mut d)?;
        Text(3.1888).dump(&mut d)?;
        NewLine(None).dump(&mut d)?;
        NewLine(None).dump(&mut d)?;
        d.open_context(Some("- "), None)?;
        Text("This is the first enumeration item and it might be very long so that it does not fit a single line\n")
            .dump(&mut d)?;
        d.close_context()?;
        d.open_context(Some("- "), None)?;
        Text("This is the second enueration itme\n").dump(&mut d)?;
        d.close_context()?;

        NewLine(Some(Indentation::Decrement(1))).dump(&mut d)?;
        Ok(())
    }

    compose_file().expect("failed to compose test file");
    check_result("test_context_text_dump");
}
#[test]
fn text_accepts_display_directly_and_inside_contexts() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default());
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
