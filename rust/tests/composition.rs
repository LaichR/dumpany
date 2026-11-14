mod common;

use common::memory_writer;
use dumpany::core::{Dump, Dumper, DumperConfig, NewLine, Text};
use std::{
    cell::RefCell,
    io::{Error, ErrorKind, Result, Write},
    rc::Rc,
};

use dumpany::dump_tuple::DumpHeadTail;
use tuple_list::tuple_list;

#[test]
fn iterator_dumps_empty_single_and_multiple_sequences() -> Result<()> {
    for (count, expected) in [(0, ""), (1, "1\n"), (3, "123\n")] {
        let writer = memory_writer();
        let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
        dumper.iterator((1..=count).map(Text))?;
        if count > 0 {
            dumper.new_line(None)?;
        }
        assert_eq!(dumper.get_ref(), expected.as_bytes());
    }
    Ok(())
}

#[test]
fn join_emits_delimiters_only_between_elements() -> Result<()> {
    for (count, expected) in [(0, ""), (1, "1\n"), (3, "1,2,3\n")] {
        let writer = memory_writer();
        let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
        dumper.join((1..=count).map(Text), Text(","))?;
        if count > 0 {
            dumper.new_line(None)?;
        }
        assert_eq!(dumper.get_ref(), expected.as_bytes());
    }
    Ok(())
}

#[test]
fn join_accepts_structured_delimiters() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    dumper.join(
        [Text("first"), Text("second"), Text("third")],
        NewLine(None),
    )?;
    dumper.new_line(None)?;
    assert_eq!(dumper.get_ref(), b"first\nsecond\nthird\n");
    Ok(())
}

struct DumpStep {
    name: &'static str,
    calls: Rc<RefCell<Vec<&'static str>>>,
    fail: bool,
}

impl Dump for DumpStep {
    fn dump<W: Write>(&self, _: &mut Dumper<W>) -> Result<()> {
        self.calls.borrow_mut().push(self.name);
        if self.fail {
            Err(Error::other(self.name))
        } else {
            Ok(())
        }
    }
}

#[test]
fn iterator_stops_consuming_after_an_item_error() {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut consumed = 0;
    let items = ["first", "second", "third"].into_iter().map(|name| {
        consumed += 1;
        DumpStep {
            name,
            calls: calls.clone(),
            fail: name == "second",
        }
    });
    let error = dumper.iterator(items).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Other);
    assert_eq!(error.to_string(), "second");
    assert_eq!(&*calls.borrow(), &["first", "second"]);
    assert_eq!(consumed, 2);
}

#[test]
fn join_stops_on_item_or_delimiter_errors() {
    for failure in ["first", "second", "delimiter"] {
        let writer = memory_writer();
        let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let step = |name| DumpStep {
            name,
            calls: calls.clone(),
            fail: name == failure,
        };
        let error = dumper
            .join(
                [step("first"), step("second"), step("third")],
                step("delimiter"),
            )
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Other);
        assert_eq!(error.to_string(), failure);
        let expected: &[&str] = match failure {
            "first" => &["first"],
            "second" => &["first", "delimiter", "second"],
            _ => &["first", "delimiter"],
        };
        assert_eq!(&*calls.borrow(), expected);
    }
}

#[test]
fn empty_tuple_emits_nothing() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    tuple_list!().dump_head_tail(&mut dumper)?;
    dumper.tuple(tuple_list!())?;
    assert!(dumper.get_ref().is_empty());
    Ok(())
}

#[test]
fn single_element_is_dumped() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    dumper.tuple(tuple_list!(Text("hello")))?;
    dumper.new_line(None)?;
    assert_eq!(dumper.get_ref(), b"hello\n");
    Ok(())
}

#[test]
fn heterogeneous_elements_are_dumped_in_order() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    dumper.tuple(tuple_list!(
        Text("answer: "),
        Text(42),
        NewLine(None),
        Text("done"),
    ))?;
    dumper.new_line(None)?;
    assert_eq!(dumper.get_ref(), b"answer: 42\ndone\n");
    Ok(())
}

#[test]
fn tuple_list_can_be_dumped_repeatedly() -> Result<()> {
    let writer = memory_writer();
    let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
    let items = tuple_list!(Text("hello"), NewLine(None));
    items.dump_head_tail(&mut dumper)?;
    items.dump_head_tail(&mut dumper)?;
    assert_eq!(dumper.get_ref(), b"hello\nhello\n");
    Ok(())
}

#[test]
fn errors_in_head_or_tail_stop_remaining_elements() {
    for failing_index in [0, 1] {
        let writer = memory_writer();
        let mut dumper = Dumper::new(writer, DumperConfig::default()).unwrap();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let step = |name, index| DumpStep {
            name,
            calls: calls.clone(),
            fail: index == failing_index,
        };
        let error = dumper
            .tuple(tuple_list!(
                step("first", 0),
                step("second", 1),
                step("third", 2),
            ))
            .unwrap_err();
        let expected = &["first", "second"][..=failing_index];
        assert_eq!(&*calls.borrow(), expected);
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert_eq!(error.to_string(), expected[failing_index]);
    }
}
