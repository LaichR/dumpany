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

    use crate::core::{Dump, Dumper, DumperConfig, Indentation, NewLine, Text};

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
