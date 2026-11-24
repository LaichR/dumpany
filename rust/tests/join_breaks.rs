use dumpany::context::Context;
use dumpany::core::{BreakPolicy, Dumper, DumperConfig, NewLine, Target, Text};
use std::io::Result;

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
fn policies_control_argument_breaks_inside_no_break_region() -> Result<()> {
    for (policy, expected) in [
        (BreakPolicy::Inherit, "call(aaa, bbb, ccc)\n"),
        (BreakPolicy::Always, "call(aaa,\nbbb,\nccc)\n"),
        (BreakPolicy::AsNeeded, "call(aaa,\nbbb, ccc)\n"),
    ] {
        let mut out = dumper(12)?;
        out.no_break_region(|out| {
            out.text("call(")?;
            out.join_with_breaks([Text("aaa"), Text("bbb"), Text("ccc")], Text(", "), policy)?;
            out.text(")")
        })?;
        assert_eq!(out.into_inner().unwrap(), expected.as_bytes());
    }
    Ok(())
}

#[test]
fn joins_preserve_context_alignment_and_late_sections() -> Result<()> {
    let mut out = dumper(12)?;
    out.open_context(None, Some("root"))?;
    out.no_break_region(|out| {
        let mut context = Context::new(Some("call("), None);
        context.new_line = false;
        context.with(out, |out| {
            out.begin_join(BreakPolicy::AsNeeded)?;
            out.text("a")?;
            out.next_join_part()?;
            out.text(", ")?;
            out.next_join_part()?;
            let mut item = Context::new(None, Some("item"));
            item.new_line = false;
            item.with(out, |out| out.text("b"))?;
            out.end_join()
        })?;
        out.text(")")
    })?;
    out.with_target(Target::Prologue("item"), |out| out.text("long"))?;
    out.close_context(None, true)?;
    assert_eq!(out.into_inner().unwrap(), b"call(a,\n     longb)\n");
    Ok(())
}

#[test]
fn surrounding_prologue_changes_the_actual_join_column() -> Result<()> {
    let mut out = dumper(10)?;
    out.open_context(None, Some("root"))?;
    out.no_break_region(|out| {
        let mut call = Context::new(None, Some("call"));
        call.new_line = false;
        call.with(out, |out| {
            out.join_with_breaks([Text("a"), Text("bbb")], Text(", "), BreakPolicy::AsNeeded)
        })?;
        out.text(")")
    })?;
    out.with_target(Target::Prologue("call"), |out| out.text("call("))?;
    out.close_context(None, true)?;
    assert_eq!(out.into_inner().unwrap(), b"call(a,\nbbb)\n");
    Ok(())
}

#[test]
fn empty_single_exact_fit_and_oversized_items() -> Result<()> {
    for policy in [
        BreakPolicy::Inherit,
        BreakPolicy::Always,
        BreakPolicy::AsNeeded,
    ] {
        let mut out = dumper(8)?;
        out.join_with_breaks(std::iter::empty::<Text<&str>>(), Text(", "), policy)?;
        assert!(out.into_inner().unwrap().is_empty());
        let mut out = dumper(8)?;
        out.join_with_breaks([Text("one")], Text(", "), policy)?;
        assert_eq!(out.into_inner().unwrap(), b"one\n");
    }
    let mut out = dumper(8)?;
    out.join_with_breaks(
        [Text("aaa"), Text("bbb")],
        Text(", "),
        BreakPolicy::AsNeeded,
    )?;
    assert_eq!(out.into_inner().unwrap(), b"aaa, bbb\n");
    let mut out = dumper(3)?;
    out.join_with_breaks(
        [Text("abc"), Text("long item")],
        Text(","),
        BreakPolicy::AsNeeded,
    )?;
    assert_eq!(out.into_inner().unwrap(), b"abc,\nlong item\n");
    Ok(())
}

#[test]
fn explicit_newline_delimiters_do_not_produce_extra_blank_lines() -> Result<()> {
    for policy in [BreakPolicy::Always, BreakPolicy::AsNeeded] {
        let mut out = dumper(2)?;
        out.join_with_breaks([Text("first"), Text("second")], NewLine(None), policy)?;
        assert_eq!(out.into_inner().unwrap(), b"first\nsecond\n");
    }
    Ok(())
}

#[test]
fn nested_joins_and_regions_are_recorded_without_repeating_callbacks() -> Result<()> {
    let mut out = dumper(8)?;
    let mut calls = 0;
    out.begin_join(BreakPolicy::Always)?;
    out.no_break_region(|out| out.text("first item"))?;
    out.next_join_part()?;
    out.text("; ")?;
    out.next_join_part()?;
    out.join_with_breaks(
        ["é", "界"].map(|s| {
            calls += 1;
            Text(s)
        }),
        Text(", "),
        BreakPolicy::AsNeeded,
    )?;
    out.end_join()?;
    assert_eq!(calls, 2);
    assert_eq!(out.into_inner().unwrap(), "first item;\né, 界\n".as_bytes());
    Ok(())
}

#[test]
fn first_explicit_line_not_total_item_length_determines_fit() -> Result<()> {
    let mut out = dumper(8)?;
    out.join_with_breaks(
        [Text("aaa"), Text("b\nlong second line")],
        Text(", "),
        BreakPolicy::AsNeeded,
    )?;
    assert_eq!(out.into_inner().unwrap(), b"aaa, b\nlong second line\n");
    let mut out = dumper(3)?;
    out.join_with_breaks(
        [Text("oversized"), Text("\nnext")],
        Text(", "),
        BreakPolicy::AsNeeded,
    )?;
    assert_eq!(out.into_inner().unwrap(), b"oversized,\nnext\n");
    Ok(())
}

#[test]
fn broken_part_boundaries_are_terminal() -> Result<()> {
    for policy in [
        BreakPolicy::Inherit,
        BreakPolicy::Always,
        BreakPolicy::AsNeeded,
    ] {
        let mut out = dumper(8)?;
        out.begin_join(policy)?;
        out.open_context(None, None)?;
        out.next_join_part()?;
        out.close_context(None, false)?;
        out.next_join_part()?;
        // Encoding accepts balanced overall content, but replay rejects a context
        // crossing an item boundary. No recording-time depth snapshots are needed.
        assert!(out.end_join().is_err());
        assert!(out.dump_all().is_err());
    }
    Ok(())
}

#[test]
fn malformed_join_markers_are_rejected_during_replay() -> Result<()> {
    let mut out = dumper(8)?;
    out.begin_join(BreakPolicy::Inherit)?;
    out.next_join_part()?;
    assert!(out.end_join().is_err());
    assert!(out.dump_all().is_err());

    let mut out = dumper(8)?;
    out.open_context(None, None)?;
    out.end_join()?;
    assert!(out.close_context(None, false).is_err());
    assert!(out.dump_all().is_err());
    Ok(())
}

#[test]
fn indentation_changes_and_enclosing_context_closes_are_terminal() -> Result<()> {
    for operation in 0..4 {
        let mut out = dumper(12)?;
        out.open_context(None, None)?;
        out.begin_join(BreakPolicy::Inherit)?;
        match operation {
            0 => out.inc_indent(),
            1 => out.dec_indent(),
            2 => out.new_line(Some(dumpany::core::Indentation::Increment(0))),
            _ => out.close_context(None, false),
        }?;
        out.end_join()?;
        assert!(out.dump_all().is_err());
        assert!(out.get_ref().is_empty());
    }
    Ok(())
}

#[test]
fn moved_items_align_local_contexts_at_the_new_column() -> Result<()> {
    let mut out = dumper(12)?;
    let mut outer = Context::new(Some("-> "), None);
    outer.new_line = false;
    outer.with(&mut out, |out| {
        out.begin_join(BreakPolicy::AsNeeded)?;
        out.text("first")?;
        out.next_join_part()?;
        out.text(", ")?;
        out.next_join_part()?;
        let mut local = Context::new(Some("["), None);
        local.new_line = false;
        local.with(out, |out| out.text("long\nnext"))?;
        out.text("]")?;
        out.end_join()
    })?;
    assert_eq!(
        out.into_inner().unwrap(),
        b"-> first,\n   [long\n    next]\n"
    );
    Ok(())
}
