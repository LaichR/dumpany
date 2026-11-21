use dumpany::core::{Dump, Dumper, DumperConfig, NoBreak};
use std::io::Result;

#[test]
fn character_limit_is_enforced_before_output_or_recording() -> Result<()> {
    for recorded in [false, true] {
        let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
        if recorded {
            out.open_context(None, None)?;
        }
        let oversized = "界".repeat(usize::from(u16::MAX) + 1);
        assert_eq!(
            out.no_break(oversized).unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
        let maximum = "界".repeat(usize::from(u16::MAX));
        out.no_break(&maximum)?;
        assert_eq!(
            out.into_inner().map_err(|e| e.into_parts().0)?,
            format!("{maximum}\n").as_bytes()
        );
    }
    Ok(())
}

#[test]
fn indivisible_text_wraps_before_but_never_within() -> Result<()> {
    for recorded in [false, true] {
        for (prefix, text, expected) in [
            ("abc ", "de fg", "abc de fg\n"),
            ("abcd ", "ef gh", "abcd\nef gh\n"),
            ("abc", "very long literal", "abc\nvery long literal\n"),
            ("", "very long literal", "very long literal\n"),
            ("abc", "", "abc\n"),
            ("ééé ", "界界 界界", "ééé 界界 界界\n"),
        ] {
            let mut out = Dumper::new(
                Vec::new(),
                DumperConfig {
                    line_width: 9,
                    ..DumperConfig::default()
                },
            )?;
            if recorded {
                out.open_context(None, None)?;
            }
            out.text(prefix)?;
            NoBreak(text).dump(&mut out)?;
            let bytes = out.into_inner().map_err(|e| e.into_parts().0)?;
            assert_eq!(String::from_utf8(bytes).unwrap(), expected);
        }
    }
    Ok(())
}

#[test]
fn indentation_does_not_produce_an_empty_line_for_oversized_text() -> Result<()> {
    let mut out = Dumper::new(
        Vec::new(),
        DumperConfig {
            line_width: 2,
            ..DumperConfig::default()
        },
    )?;
    out.inc_indent()?;
    out.no_break("a b c")?;
    assert_eq!(
        out.into_inner().map_err(|e| e.into_parts().0)?,
        b"    a b c\n"
    );
    Ok(())
}

#[test]
fn no_break_wraps_after_context_prefix_and_preserves_alignment() -> Result<()> {
    let mut out = Dumper::new(
        Vec::new(),
        DumperConfig {
            line_width: 8,
            ..DumperConfig::default()
        },
    )?;
    out.open_context(Some("é: "), None)?;
    out.no_break("a long literal")?;
    assert_eq!(
        out.into_inner().map_err(|e| e.into_parts().0)?,
        "é:\n   a long literal\n".as_bytes()
    );
    Ok(())
}
