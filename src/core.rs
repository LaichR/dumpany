use crate::context::{Context, Cursor};
use crate::core::Indentation::{Decrement, Increment};
use crate::operations::Operation;
use crate::recorder::Recorder;
use std::cell::RefCell;
use std::fmt::Display;
use std::io::{Error, ErrorKind, Result, Write};
use std::rc::Rc;
use std::unreachable;

const DEFAULT_WIDTH: usize = 120_usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indentation {
    Increment(u8),
    Decrement(u8),
}

pub trait Dump {
    fn dump<T: Write>(&self, out: &mut Dumper<T>) -> Result<()>;
}

pub struct DumperConfig {
    pub new_line: &'static str,
    pub indent_width: usize,
    pub line_width: usize,
}

impl Default for DumperConfig {
    fn default() -> Self {
        Self {
            new_line: "\n",
            indent_width: 4,
            line_width: DEFAULT_WIDTH,
        }
    }
}

pub struct Dumper<T: Write> {
    writer: Rc<RefCell<T>>,
    indent: usize,
    config: DumperConfig,
    recorder: Option<Recorder>,
    current_line: String,
    left_margin: Vec<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Text<T>(pub T);

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct NewLine(pub Option<Indentation>);

impl Dump for NewLine {
    fn dump<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()> {
        dumper.new_line(self.0)
    }
}

impl<T: Display> Dump for Text<T> {
    fn dump<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()> {
        dumper.text(&self.0)
    }
}

impl<T: Write> Dumper<T> {
    pub fn new(writer: Rc<RefCell<T>>, config: DumperConfig) -> Self {
        Self {
            writer,
            config,
            recorder: None,
            indent: 0,
            current_line: String::new(),
            left_margin: Vec::new(),
        }
    }

    pub fn text(&mut self, text: impl Display) -> Result<()> {
        if let Some(recorder) = &mut self.recorder {
            recorder.text(text)?
        } else {
            self.dump_text(&text.to_string())?
        }
        Ok(())
    }

    pub fn inc_indent(&mut self) -> Result<()> {
        if let Some(recorder) = &mut self.recorder {
            recorder.inc_indent()?
        } else {
            self.indent += 1;
        }
        Ok(())
    }

    pub fn dec_indent(&mut self) -> Result<()> {
        if let Some(recorder) = &mut self.recorder {
            recorder.dec_indent()?
        } else {
            self.indent -= 1;
        }
        Ok(())
    }

    pub fn new_line(&mut self, change_indentation: Option<Indentation>) -> Result<()> {
        if let Some(recorder) = &mut self.recorder {
            recorder.new_line(change_indentation)?
        }
        self.flush_line()?;
        if let Some(indentation) = change_indentation {
            match indentation {
                Increment(how_mutch) => self.indent += how_mutch as usize,
                Decrement(how_mutch) => {
                    self.indent -= if how_mutch as usize <= self.indent {
                        how_mutch as usize
                    } else {
                        self.indent
                    };
                }
            };
        }
        Ok(())
    }

    pub fn open_context(
        &mut self,
        prefix: Option<&'static str>,
        name: Option<&'static str>,
    ) -> Result<()> {
        match &mut self.recorder {
            None => {
                let recorder = Recorder::new(prefix, name)?;
                self.recorder = Some(recorder);
                Ok(())
            }
            Some(recorder) => recorder.open_context(prefix, name),
        }
    }

    pub fn close_context(&mut self) -> Result<()> {
        let dump_context = match &mut self.recorder {
            Some(recorder) => {
                recorder.close_context()?;
                recorder.context_depth() == 0
            }
            None => false,
        };
        if dump_context {
            let recorder = match self.recorder.take() {
                Some(recorder) => recorder,
                None => unreachable!(),
            };
            let mut cursor = Cursor::new(recorder.encoded_content());
            let root = Context::from_encoding(&mut cursor)?;
            self.dump_section(root)?;
        }
        Ok(())
    }

    fn dump_section(&mut self, mut context: Context<'_, '_>) -> Result<()> {
        self.set_left_margin();
        let mut has_prefix = false;
        if let Some(prefix) = context.prefix {
            // we assume that the prefix is allways

            self.current_line.push_str(prefix);
            if self.current_line.len() > self.config.line_width {
                // we assume that the prefix is short and fits onto the
                // line within the line width!
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Write context prefix cannot be placed properly",
                ));
            }
            self.set_left_margin();
            has_prefix = true;
        }
        while let Ok(f) = context.next_event() {
            match f {
                Some(Operation::Text(txt)) => self.dump_text(txt.0)?,
                Some(Operation::NewLine(nl)) => self.new_line(nl.0)?,
                Some(Operation::OpenContext(context)) => self.dump_section(context)?,
                Some(Operation::CloseContext(_)) => {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "close context should be consumed by parent",
                    ));
                }
                Some(Operation::Indentation(_)) => {}
                None => break, // this is the end of a context
            }
        }
        if has_prefix {
            self.release_left_margin();
        }
        self.release_left_margin();
        self.flush_line()?;
        Ok(())
    }

    fn dump_text(&mut self, text: &str) -> Result<()> {
        let mut iter = text.split("\n");

        if let Some(mut current) = iter.next() {
            for next in iter {
                // current is definitely NOT the last
                self.process_line(current, true)?;
                current = next;
            }
            // current is the last
            self.process_line(current, false)?;
        }
        Ok(())
    }

    fn set_left_margin(&mut self) {
        // observer the indentation
        if self.current_line.is_empty() {
            self.current_line.push_str(&self.get_indentation_string());
        }
        self.left_margin.push(self.current_line.len());
    }

    fn release_left_margin(&mut self) {
        self.left_margin.pop();
    }

    fn get_indentation_string(&self) -> String {
        " ".repeat(self.indent * self.config.indent_width)
    }
    fn prepare_current_line(&mut self) -> Result<()> {
        if self.current_line.len() >= self.config.line_width {
            self.flush_line()?;
        }
        if self.current_line.is_empty() {
            if let Some(left_margin) = self.left_margin.last() {
                self.current_line.push_str(&" ".repeat(*left_margin))
            } else {
                let indentation = self.get_indentation_string();
                self.current_line.push_str(&indentation);
            }
        }
        Ok(())
    }

    /// add a line and return false if the delimiter needs to go on the next line
    /// since it did not fit anymore
    fn add_line(&mut self, line: &str, delimiter: char) -> bool {
        self.current_line.push_str(line);
        if self.current_line.len() < self.config.line_width && !delimiter.is_whitespace() {
            self.current_line.push(delimiter);
            return true;
        }
        delimiter.is_whitespace() // the delimiter doesn't need to be processed again!
    }

    fn flush_line(&mut self) -> Result<()> {
        if self.current_line.ends_with(' ') {
            self.current_line = self.current_line.trim_end().into();
        }
        self.current_line.push_str(self.config.new_line);
        self.writer
            .borrow_mut()
            .write_all(self.current_line.as_bytes())?;
        self.current_line.clear();
        Ok(())
    }

    fn process_line(&mut self, line: &str, add_new_line: bool) -> Result<()> {
        let f = |x: char| matches!(x, ' ' | '\t' | ',' | ';');
        let mut end = line.len();
        let mut head = line;
        self.prepare_current_line()?;
        while head.len() + self.current_line.len() > self.config.line_width {
            if let Some(mut pos) = head[..end].rfind(&f) {
                if pos + self.current_line.len() <= self.config.line_width {
                    if self.add_line(&head[..pos], head[pos..].chars().next().unwrap()) {
                        pos += 1;
                    }
                    self.flush_line()?;
                    self.prepare_current_line()?;
                    head = &head[pos..];
                    end = head.len()
                } else {
                    end = pos;
                }
            } else {
                // we cannot find another delimiter
                break;
            }
        }
        self.add_line(head, ' ');
        if add_new_line {
            self.flush_line()?;
        }
        Ok(())
    }
}
