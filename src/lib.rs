pub mod context;
pub mod core;
pub mod operations;
pub mod recorder;

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        fs,
        io::{ErrorKind, Result},
        path::Path,
        rc::Rc,
    };

    use crate::context::Context;
    use crate::core::{Dump, Dumper, DumperConfig, Indentation, NewLine, Target, Text};

    #[test]
    fn closed_children_remain_targetable_until_flush() -> Result<()> {
        let writer = Rc::new(RefCell::new(Vec::new()));
        let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
        Context::new(None, Some("root")).with(&mut dumper, |d| {
            Context::new(None, Some("child")).with(d, |d| d.text("body"))?;
            assert!(writer.borrow().is_empty());
            assert_eq!(
                d.open_context(None, Some("child")).unwrap_err().kind(),
                ErrorKind::AlreadyExists
            );
            d.with_target(Target::Prologue("child"), |d| d.text("before:"))?;
            d.with_target(Target::Prologue("child"), |d| d.text("again:"))?;
            d.with_target(Target::Epilogue("child"), |d| d.text(":after"))?;
            d.text("tail")
        })?;
        assert_eq!(&*writer.borrow(), b"before:again:body:after\ntail\n");
        Context::new(None, Some("child")).with(&mut dumper, |_| Ok(()))?;
        assert_eq!(&*writer.borrow(), b"before:again:body:after\ntail\n\n");
        Ok(())
    }

    #[test]
    fn flush_failure_clears_names_and_section_recorders() -> Result<()> {
        let writer = Rc::new(RefCell::new(Vec::new()));
        let mut dumper = Dumper::new(
            writer.clone(),
            DumperConfig {
                line_width: 1,
                ..DumperConfig::default()
            },
        );
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
        assert_eq!(&*writer.borrow(), b"too long\n");
        Ok(())
    }

    #[test]
    fn section_selection_is_not_reentrant_and_sections_are_independent() -> Result<()> {
        for outer in [Target::Prologue("root"), Target::Epilogue("root")] {
            let writer = Rc::new(RefCell::new(Vec::new()));
            let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
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
            assert_eq!(&*writer.borrow(), expected);
        }
        Ok(())
    }

    #[test]
    fn unnamed_contexts_remain_usable_without_sections() -> Result<()> {
        let writer = Rc::new(RefCell::new(Vec::new()));
        let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
        Context::new(None, None).with(&mut dumper, |d| {
            for text in ["first", "second"] {
                Context::new(None, None).with(d, |d| d.text(text))?;
            }
            Ok(())
        })?;
        assert_eq!(&*writer.borrow(), b"first\nsecond\n\n");
        Ok(())
    }

    #[test]
    fn section_contexts_cannot_select_another_target() -> Result<()> {
        let writer = Rc::new(RefCell::new(Vec::new()));
        let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
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
        assert_eq!(&*writer.borrow(), b"ab\ndbody\n");
        Ok(())
    }

    fn result_writer(test_name: &str) -> Result<Rc<RefCell<fs::File>>> {
        let test_data = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");
        let result_dir = test_data.join("result");
        fs::create_dir_all(&result_dir)?;
        let file = fs::File::create(result_dir.join(format!("{}.txt", test_name)))?;
        Ok(Rc::new(RefCell::new(file)))
    }

    fn check_result(test_name: &str) {
        let test_data = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");
        let filename = format!("{}.txt", test_name);
        let result_path = test_data.join("result").join(&filename);

        let check_path = test_data.join("check").join(&filename);
        match fs::read(&check_path) {
            Ok(reference) => {
                let result = fs::read(&result_path).expect("failed to read test result");
                assert_eq!(
                    result,
                    reference,
                    "test result {} differs from reference {}",
                    result_path.display(),
                    check_path.display()
                );
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => panic!(
                "failed to read reference {}: {}",
                check_path.display(),
                error
            ),
        }
    }

    #[test]
    fn test_simple_text_dump() {
        fn compose_file() -> Result<()> {
            let mut d = Dumper::new(
                result_writer("test_simple_text_dump")?,
                DumperConfig {
                    new_line: "\n",
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
                    new_line: "\n",
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
}
