use crate::core::Indentation::{Decrement, Increment};
use crate::dump_tuple::DumpHeadTail;
use crate::operations::{FromBytes, Operation};
use crate::recorder::Recorder;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::{Error, ErrorKind, Result, Write};

const DEFAULT_WIDTH: usize = 120_usize;

/// Select a lazily created context prologue or epilogue.
/// Requires an explicit context name, which remains valid until root flushing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target<'a> {
    Prologue(&'a str),
    Epilogue(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indentation {
    Increment(u8),
    Decrement(u8),
}

pub trait Dump {
    fn dump<T: Write>(&self, out: &mut Dumper<T>) -> Result<()>;
}

pub struct DumperConfig {
    pub new_line: String,
    pub indent_width: usize,
    pub line_width: usize,
}

impl Default for DumperConfig {
    fn default() -> Self {
        Self {
            new_line: String::from("\n"),
            indent_width: 4,
            line_width: DEFAULT_WIDTH,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DumperState {
    Active,
    Finalized,
    Failed,
}

pub struct Dumper<T: Write> {
    state: DumperState,
    writer: T,
    indent: usize,
    config: DumperConfig,
    recorder: Option<Recorder>,
    names: Vec<String>,
    prologues: HashMap<String, Recorder>,
    epilogues: HashMap<String, Recorder>,
    current_line: String,
    left_margin: Vec<usize>,
    selected_target: Option<SelectedTarget>,
}

struct SelectedTarget {
    main: Recorder,
    name: String,
    prologue: bool,
}

/// A finalization error that retains the writer, discarding pending output.
/// The writer may contain a partial write, not necessarily a complete line.
pub struct IntoInnerError<T: Write> {
    error: Error,
    writer: Box<T>,
}

impl<T: Write> IntoInnerError<T> {
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Return the writer as-is without retrying failed output.
    pub fn into_inner(self) -> T {
        *self.writer
    }

    /// Return the original error and writer without retrying output.
    /// Any remaining buffered output is discarded.
    pub fn into_parts(self) -> (Error, T) {
        (self.error, *self.writer)
    }
}

impl<T: Write> std::fmt::Debug for IntoInnerError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntoInnerError")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<T: Write> std::fmt::Display for IntoInnerError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl<T: Write> std::error::Error for IntoInnerError<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
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
    /// Create a dumper with the supplied writer and configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] if `config.new_line` is empty.
    pub fn new(writer: T, config: DumperConfig) -> Result<Self> {
        if config.new_line.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "line ending must not be empty",
            ));
        }
        Ok(Self {
            state: DumperState::Active,
            writer,
            config,
            recorder: None,
            names: Vec::new(),
            prologues: HashMap::new(),
            epilogues: HashMap::new(),
            indent: 0,
            current_line: String::new(),
            left_margin: Vec::new(),
            selected_target: None,
        })
    }

    /// Borrow the writer without flushing buffered output.
    pub fn get_ref(&self) -> &T {
        &self.writer
    }

    /// Finalize the dump and return its writer. On failure, the error retains
    /// only the writer and original error; remaining buffered output is discarded.
    /// Failed output is never retried and may leave a partial write in the writer.
    pub fn into_inner(mut self) -> std::result::Result<T, IntoInnerError<T>> {
        match self.dump_all() {
            Ok(()) => Ok(self.writer),
            Err(error) => Err(IntoInnerError {
                error,
                writer: Box::new(self.writer),
            }),
        }
    }

    fn ensure_active(&self) -> Result<()> {
        match self.state {
            DumperState::Active => Ok(()),
            DumperState::Finalized => Err(Error::other("dumper is finalized")),
            DumperState::Failed => Err(Error::other("dumper has failed")),
        }
    }

    /// Close all contexts, emit a pending line, and flush the writer.
    /// Never closes the writer. Successful repeated calls do nothing; failures are
    /// terminal because output may already have been partially written.
    pub fn dump_all(&mut self) -> Result<()> {
        if self.state == DumperState::Finalized {
            return Ok(());
        }
        self.ensure_active()?;
        if self.selected_target.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot finalize inside a target scope",
            ));
        }
        let result = (|| {
            while self.recorder.is_some() {
                self.close_context()?;
            }
            if !self.current_line.is_empty() {
                self.flush_line()?;
            }
            self.writer.flush()
        })();
        self.state = if result.is_ok() {
            DumperState::Finalized
        } else {
            DumperState::Failed
        };
        result
    }

    /// Select where subsequent operations are recorded, without opening a context.
    /// Requires an open context and a registered name. Selecting another target
    /// before selecting main again is forbidden. Does not write or flush output.
    pub fn select_target(&mut self, target: Target<'_>) -> Result<()> {
        self.ensure_active()?;
        if self.selected_target.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "target selection is only allowed from the main body",
            ));
        }
        let recorder = self.recorder.as_mut().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "target selection requires an open context",
            )
        })?;
        let requested = match target {
            Target::Prologue(name) | Target::Epilogue(name) => name,
        };
        let name = self
            .names
            .iter()
            .find(|name| name.as_str() == requested)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "unknown context name"))?;
        let section = match target {
            Target::Prologue(_) => self.prologues.remove(&name),
            Target::Epilogue(_) => self.epilogues.remove(&name),
        }
        .unwrap_or_default();
        let main = std::mem::replace(recorder, section);
        self.selected_target = Some(SelectedTarget {
            main,
            name,
            prologue: matches!(target, Target::Prologue(_)),
        });
        Ok(())
    }

    /// Save the selected target and restore main recording. Already on main
    /// is a no-op. Does not close the enclosing context or flush output.
    /// Open target contexts are retained and can be continued by selecting the
    /// same target again. They must be closed before the root is rendered.
    pub fn select_main(&mut self) -> Result<()> {
        self.ensure_active()?;
        let Some(selected) = self.selected_target.take() else {
            return Ok(());
        };
        let section = self
            .recorder
            .replace(selected.main)
            .expect("selected target always has a recorder");
        if selected.prologue {
            self.prologues.insert(selected.name, section);
        } else {
            self.epilogues.insert(selected.name, section);
        }
        Ok(())
    }

    /// Record a prologue or epilogue, then select main again, including on error.
    /// Open target contexts may be continued in later selections. The closure's
    /// error takes precedence over a restoration error. Panics are not handled.
    pub fn with_target<R>(
        &mut self,
        target: Target<'_>,
        body: impl FnOnce(&mut Self) -> Result<R>,
    ) -> Result<R> {
        self.select_target(target)?;
        let result = body(self);
        let restored = self.select_main();
        match result {
            Ok(value) => restored.map(|()| value),
            Err(error) => Err(error),
        }
    }

    /// Append text, treating occurrences of the configured line ending as line breaks.
    pub fn text(&mut self, text: impl Display) -> Result<()> {
        self.ensure_active()?;
        if let Some(recorder) = &mut self.recorder {
            recorder.text(text)?;
        } else {
            self.dump_text(&text.to_string())?;
        }
        Ok(())
    }

    pub fn join<I, D>(&mut self, items: I, delimiter: D) -> Result<()>
    where
        I: IntoIterator,
        I::Item: Dump,
        D: Dump,
    {
        self.ensure_active()?;
        let mut iterator = items.into_iter();
        if let Some(first) = iterator.next() {
            first.dump(self)?;
            for i in iterator {
                delimiter.dump(self)?;
                i.dump(self)?;
            }
        }
        Ok(())
    }

    pub fn iterator<I>(&mut self, items: I) -> Result<()>
    where
        I: IntoIterator,
        I::Item: Dump,
    {
        self.ensure_active()?;
        let iterator = items.into_iter();
        for i in iterator {
            i.dump(self)?;
        }
        Ok(())
    }

    pub fn tuple<Tuple: DumpHeadTail>(&mut self, tuple: Tuple) -> Result<()> {
        self.ensure_active()?;
        tuple.dump_head_tail(self)
    }

    pub fn inc_indent(&mut self) -> Result<()> {
        self.ensure_active()?;
        if let Some(recorder) = &mut self.recorder {
            recorder.inc_indent(1)?;
        } else {
            self.indent += 1;
        }
        Ok(())
    }

    pub fn dec_indent(&mut self) -> Result<()> {
        self.ensure_active()?;
        if let Some(recorder) = &mut self.recorder {
            recorder.dec_indent(1)?;
        } else {
            self.indent = self.indent.saturating_sub(1);
        }
        Ok(())
    }

    pub fn new_line(&mut self, change_indentation: Option<Indentation>) -> Result<()> {
        self.ensure_active()?;
        if let Some(recorder) = &mut self.recorder {
            return recorder.new_line(change_indentation);
        }
        self.flush_line()?;
        self.apply_indentation_change(change_indentation);
        Ok(())
    }

    pub fn open_context(&mut self, prefix: Option<&str>, name: Option<&str>) -> Result<()> {
        self.ensure_active()?;
        if name.is_some_and(|name| self.names.iter().any(|registered| registered == name)) {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                "context name is already registered",
            ));
        }
        self.recorder
            .get_or_insert_with(Recorder::default)
            .open_context(prefix, name)?;
        if let Some(name) = name {
            self.names.push(name.to_owned());
        }
        Ok(())
    }

    pub fn close_context(&mut self) -> Result<()> {
        self.ensure_active()?;
        if self.selected_target.is_none()
            // the root context is beeing closed
            && self
                .recorder
                .as_ref()
                .is_some_and(|recorder| recorder.context_depth() == 1)
            // there are open prolgues or epilogues
            && self
                .prologues
                .values()
                .chain(self.epilogues.values())
                .any(|recorder| recorder.context_depth() != 0)
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot render root with unclosed target contexts",
            ));
        }
        if self
            .recorder
            .as_ref()
            .is_some_and(|recorder| self.selected_target.is_some() && recorder.context_depth() == 0)
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot close an enclosing context inside a target scope",
            ));
        }
        let dump_to_writer = match &mut self.recorder {
            Some(recorder) => {
                recorder.close_context()?;
                self.selected_target.is_none() && recorder.context_depth() == 0
            }
            None => false,
        };
        if dump_to_writer {
            let recorder = self.recorder.take().expect("root recorder exists");
            let prologues = std::mem::take(&mut self.prologues);
            let epilogues = std::mem::take(&mut self.epilogues);
            self.names.clear();
            self.dump_recorder(&prologues, &epilogues, &recorder)?;
        }
        Ok(())
    }

    /// Close an aligned block, then explicitly adjust normal indentation.
    /// No additional line break is emitted. Decrements stop at zero.
    pub fn close_context_with_indentation(
        &mut self,
        indentation: Option<Indentation>,
    ) -> Result<()> {
        self.close_context()?;
        if let Some(indentation) = indentation {
            if let Some(recorder) = &mut self.recorder {
                match indentation {
                    Increment(count) => recorder.inc_indent(count)?,
                    Decrement(count) => recorder.dec_indent(count)?,
                }
            } else {
                self.apply_indentation_change(Some(indentation));
            }
        }
        Ok(())
    }

    fn dump_context(
        &mut self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        name: Option<&str>,
        prefix: Option<&str>,
        bytes: &mut &[u8],
    ) -> Result<()> {
        self.set_left_margin();
        let mut has_prefix = false;
        if let Some(prefix) = prefix {
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
        if let Some(prologue) = name.and_then(|name| prologues.get(name)) {
            self.dump_recorder(prologues, epilogues, prologue)?;
        }
        self.dump_content(prologues, epilogues, bytes, true)?;
        if let Some(epilogue) = name.and_then(|name| epilogues.get(name)) {
            self.dump_recorder(prologues, epilogues, epilogue)?;
        }
        if has_prefix {
            self.release_left_margin();
        }
        self.release_left_margin();
        self.flush_line()?;
        Ok(())
    }

    fn dump_recorder(
        &mut self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        recorder: &Recorder,
    ) -> Result<()> {
        let mut bytes = recorder.encoded_content();
        self.dump_content(prologues, epilogues, &mut bytes, false)
    }

    fn dump_content(
        &mut self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        bytes: &mut &[u8],
        in_context: bool,
    ) -> Result<()> {
        while !bytes.is_empty() {
            match Operation::from_bytes(bytes)? {
                Operation::Text(text) => self.dump_text(text.0)?,
                Operation::NewLine(line) => self.new_line(line.0)?,
                Operation::Indentation(Increment(amount)) => self.indent += amount as usize,
                Operation::Indentation(Decrement(amount)) => {
                    self.indent = self.indent.saturating_sub(amount as usize)
                }
                Operation::OpenContext(context) => {
                    self.dump_context(prologues, epilogues, context.name, context.prefix, bytes)?;
                }
                Operation::CloseContext(_) if in_context => return Ok(()),
                Operation::CloseContext(_) => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "unexpected context close",
                    ));
                }
            }
        }
        if in_context {
            Err(Error::new(ErrorKind::UnexpectedEof, "unclosed context"))
        } else {
            Ok(())
        }
    }

    fn dump_text(&mut self, text: &str) -> Result<()> {
        let new_line = self.config.new_line.clone();
        let mut iter = text.split(new_line.as_str());

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

    fn apply_indentation_change(&mut self, indentation: Option<Indentation>) {
        match indentation {
            Some(Indentation::Increment(num)) => self.indent += usize::from(num),
            Some(Indentation::Decrement(num)) => {
                self.indent = self.indent.saturating_sub(usize::from(num))
            }
            None => {}
        }
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
        self.current_line.push_str(&self.config.new_line);
        let result = self.writer.write_all(self.current_line.as_bytes());
        if let Err(error) = result {
            self.state = DumperState::Failed;
            return Err(error);
        }
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
