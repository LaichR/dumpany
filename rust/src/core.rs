//! Format structured text with wrapping, indentation, and aligned contexts.
//!
//! # Examples
//!
//! ```
//! use dumpany::core::{Dumper, DumperConfig, Text};
//!
//! let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
//! out.join([Text("one"), Text("two")], Text(", "))?;
//! let bytes = out.into_inner().map_err(|error| error.into_parts().0)?;
//! assert_eq!(bytes, b"one, two\n");
//! # Ok::<(), std::io::Error>(())
//! ```

use crate::core::Indentation::{Decrement, Increment};
use crate::dump_tuple::DumpHeadTail;
use crate::operations::{FromBytes, Operation};
use crate::recorder::Recorder;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::{Error, ErrorKind, Result, Write};

const DEFAULT_WIDTH: usize = 120_usize;

/// Select the prologue or epilogue of a named context.
///
/// Use with [`Dumper::select_target`] or [`Dumper::with_target`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target<'a> {
    /// Content before the named context's body, after its prefix.
    Prologue(&'a str),
    /// Content after the named context's body.
    Epilogue(&'a str),
}

/// A change to the indentation level.
///
/// Each level corresponds to [`DumperConfig::indent_width`] spaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indentation {
    /// Increase indentation by the given number of levels.
    Increment(u8),
    /// Decrease indentation by the given number of levels, stopping at zero.
    Decrement(u8),
}

/// Select whether a join inserts explicit breaks between complete items.
///
/// Explicit policies suppress ordinary wrapping within items and delimiters,
/// but preserve their explicit newlines. Delimiters stay on the preceding line,
/// even if attaching them exceeds the configured width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BreakPolicy {
    /// Insert no breaks of its own; items and delimiters follow surrounding wrapping rules.
    #[default]
    Inherit,
    /// End the line after each delimiter, unless it already ends on an empty line.
    Always,
    /// Break after a delimiter when the next item's first line will not fit.
    /// Items and delimiters are emitted intact between their own explicit breaks.
    AsNeeded,
}

/// Enables a value to emit formatted output through a [`Dumper`].
pub trait Dump {
    /// Emit this value's formatted output.
    ///
    /// # Arguments
    ///
    /// * `out` — Dumper receiving the output.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if formatting or writing fails.
    fn dump<T: Write>(&self, out: &mut Dumper<T>) -> Result<()>;
}

/// Configure line endings, indentation, and wrapping for a [`Dumper`].
///
/// Defaults to LF line endings, four spaces per indentation level, and a line
/// width of 120 Unicode characters.
pub struct DumperConfig {
    /// Nonempty sequence used to separate lines.
    pub new_line: String,
    /// Number of spaces per indentation level.
    pub indent_width: usize,
    /// Preferred line width in Unicode characters, including indentation.
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

/// Produce formatted text with wrapping, indentation, and aligned contexts.
///
/// Content can be supplied out of order: a named context's prologue and epilogue
/// appear before and after its body regardless of when they are supplied.
/// Finish output explicitly with [`Self::dump_all`] or [`Self::into_inner`];
/// dropping the dumper does not finish output.
///
/// Formatting operations return an error after finalization or a writer failure.
///
/// # Examples
///
/// ```
/// use dumpany::core::{Dumper, DumperConfig, Target};
///
/// let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
/// out.open_context(None, Some("item"))?;
/// out.text("body")?;
/// out.with_target(Target::Prologue("item"), |out| out.text("before "))?;
/// out.with_target(Target::Epilogue("item"), |out| out.text(" after"))?;
/// out.close_context(None, true)?;
/// let bytes = out.into_inner().map_err(|error| error.into_parts().0)?;
/// assert_eq!(bytes, b"before body after\n");
/// # Ok::<(), std::io::Error>(())
/// ```
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
    // Recording state: one saved parent for the outer region, never a recorder stack.
    region_capture: Option<RegionCapture>,
    // Only routing state: whether each join created the root recorder.
    join_captures: Vec<bool>,
    // Includes late sections replayed inside an item.
    join_replay_depth: usize,
    // Suppress automatic line wrapping while an eventually nested no_break section is emitted.
    no_break_depth: usize,
}

struct SelectedTarget {
    main: Recorder,
    name: String,
    prologue: bool,
}

/// Save the enclosing destination while the outer region records independently.
struct RegionCapture {
    /// Parent body or section recorder; None means replay directly when finished.
    main: Option<Recorder>,
    /// Nested regions share the active recorder and only change this counter.
    depth: usize,
}

/// Recover the writer and error after [`Dumper::into_inner`] fails.
///
/// The writer may contain incomplete output. Pending output is discarded and
/// failed output is not retried.
pub struct IntoInnerError<T: Write> {
    error: Error,
    writer: T,
}

impl<T: Write> IntoInnerError<T> {
    /// Inspect the error that prevented finalization.
    ///
    /// Returns a reference to the original error.
    pub fn error(&self) -> &Error {
        &self.error
    }

    /// Return the writer as-is without retrying failed output.
    ///
    /// Returns the recovered writer, which may contain incomplete output.
    pub fn into_inner(self) -> T {
        self.writer
    }

    /// Return the original error and writer without retrying output.
    ///
    /// Returns an `(error, writer)` pair. Pending output is discarded.
    pub fn into_parts(self) -> (Error, T) {
        (self.error, self.writer)
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

/// Make a [`Display`] value usable as a [`Dump`] operation.
#[derive(Debug, PartialEq, Eq)]
pub struct Text<T>(
    /// Value to format as text.
    pub T,
);

/// Append string text without inserting line breaks within it.
///
/// Accepts borrowed or owned strings, not arbitrary displayable values.
///
/// ```compile_fail
/// use dumpany::core::{Dump, Dumper, DumperConfig, NoBreak};
/// let mut out = Dumper::new(Vec::new(), DumperConfig::default()).unwrap();
/// NoBreak(42).dump(&mut out).unwrap();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct NoBreak<T>(
    /// Indivisible string text; existing line endings are preserved.
    pub T,
);

impl<T: AsRef<str>> Dump for NoBreak<T> {
    fn dump<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()> {
        dumper.no_break(&self.0)
    }
}

/// Dump composed content without automatic line breaks inside it.
///
/// Regions nest and honor explicit newlines, including context closing breaks.
/// Use an iterator through [`Dumper::no_break_region`] or supply a [`Dump`] value
/// here. Unlike [`NoBreak`], regions have no 65,535-character limit.
pub struct NoBreakRegion<T>(
    /// Composed content to keep together between explicit line breaks.
    pub T,
);

impl<T: Dump> Dump for NoBreakRegion<T> {
    fn dump<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()> {
        dumper.no_break_region(|out| self.0.dump(out))
    }
}

pub(crate) fn no_break_length(text: &str) -> Result<u16> {
    u16::try_from(text.chars().count()).map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            "no-break text exceeds 65535 Unicode characters",
        )
    })
}

/// Insert a line break with an optional indentation change for following lines.
///
/// The default leaves indentation unchanged.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct NewLine(
    /// Change applied after the break, or `None` to leave indentation unchanged.
    pub Option<Indentation>,
);

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
    /// # Arguments
    ///
    /// * `writer` — Destination for formatted output.
    /// * `config` — Line ending, indentation, and wrapping settings.
    ///
    /// # Returns
    ///
    /// Returns a dumper with zero indentation, or [`ErrorKind::InvalidInput`]
    /// if `config.new_line` is empty.
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
            region_capture: None,
            join_captures: Vec::new(),
            join_replay_depth: 0,
            no_break_depth: 0,
        })
    }

    /// Inspect the output writer.
    ///
    /// Returns a shared reference to the writer without flushing pending output.
    pub fn get_ref(&self) -> &T {
        &self.writer
    }

    /// Finish output and recover the writer.
    ///
    /// Closes all contexts, emits pending output, and flushes the writer.
    ///
    /// # Returns
    ///
    /// Returns the writer on success. On failure, returns an [`IntoInnerError`]
    /// providing the original error and writer; pending output is discarded.
    pub fn into_inner(self) -> std::result::Result<T, IntoInnerError<T>> {
        self.into_inner_with_new_line(true)
    }

    /// Finish output and recover the writer, optionally ending the final line.
    ///
    /// # Arguments
    ///
    /// * `new_line` — Add a final line ending and end automatically closed contexts'
    ///   lines. Defaults to true when using [`Self::into_inner`]. Explicit line
    ///   breaks already requested by the caller are never removed.
    ///
    /// # Returns
    ///
    /// Returns the writer on success, or an [`IntoInnerError`] containing the
    /// error and writer on failure. Already-finalized output is unchanged.
    pub fn into_inner_with_new_line(
        mut self,
        new_line: bool,
    ) -> std::result::Result<T, IntoInnerError<T>> {
        match self.dump_all_with_new_line(new_line) {
            Ok(()) => Ok(self.writer),
            Err(error) => Err(IntoInnerError {
                error,
                writer: self.writer,
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
    ///
    /// Leaves the writer open. After success, repeated calls do nothing.
    /// Select main before calling this method.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if a target is selected, the
    /// dumper has failed, or finishing output fails. A failure while finishing
    /// output prevents further use of the dumper.
    pub fn dump_all(&mut self) -> Result<()> {
        self.dump_all_with_new_line(true)
    }

    /// Finish output, optionally ending the final line, and flush the writer.
    ///
    /// Select main first. Explicit line breaks are preserved. Repeated calls
    /// after successful finalization do nothing, regardless of `new_line`.
    ///
    /// # Arguments
    ///
    /// * `new_line` — End pending text and automatically closed contexts' lines.
    ///   Set false for a fragment that continues on the same line. This cannot
    ///   remove a newline explicitly emitted earlier, including by context closing.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())`, or an error if a target or no-break region is active,
    /// the dumper has failed, or output fails. Output failures are terminal.
    pub fn dump_all_with_new_line(&mut self, new_line: bool) -> Result<()> {
        if self.state == DumperState::Finalized {
            return Ok(());
        }
        self.ensure_active()?;
        if !self.join_captures.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot finalize inside a join",
            ));
        }
        if self.in_no_break_region() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot finalize inside a no-break region",
            ));
        }
        if self.selected_target.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot finalize inside a target scope",
            ));
        }
        let result = (|| {
            while self.recorder.is_some() {
                self.close_context(None, new_line)?;
            }
            if !self.current_line.is_empty() {
                self.flush_line_with_new_line(new_line)?;
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

    /// Direct subsequent output to a named context's prologue or epilogue.
    ///
    /// The named context must have been opened beforehand. It may already be
    /// closed while the outermost context remains open. Call [`Self::select_main`]
    /// before selecting another target.
    ///
    /// # Arguments
    ///
    /// * `target` — Section and context name to receive output.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, [`ErrorKind::NotFound`] for an unknown name,
    /// or [`ErrorKind::InvalidInput`] if no context is open or a target is already
    /// selected. Also returns an error if the dumper is finalized or has failed.
    pub fn select_target(&mut self, target: Target<'_>) -> Result<()> {
        self.ensure_active()?;
        if !self.join_captures.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot switch targets inside a join",
            ));
        }
        if self.in_no_break_region() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot switch targets inside a no-break region",
            ));
        }
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

    /// Direct subsequent output back to the main context body.
    ///
    /// Does nothing if main is already selected. Contexts opened within a target
    /// can be continued by selecting that target again; close them before closing
    /// the outermost main context.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())`, or an error if the dumper is finalized or has failed.
    pub fn select_main(&mut self) -> Result<()> {
        self.ensure_active()?;
        if !self.join_captures.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot switch targets inside a join",
            ));
        }
        if self.in_no_break_region() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "cannot switch targets inside a no-break region",
            ));
        }
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

    /// Produce a prologue or epilogue through a closure.
    ///
    /// Selects main again after the closure returns, including on error, but not
    /// if the closure panics. See [`Dumper`] for an example.
    ///
    /// # Arguments
    ///
    /// * `target` — Section and context name to receive output.
    /// * `body` — Closure that produces the section's content.
    ///
    /// # Returns
    ///
    /// Returns the closure's value on success, or an error from selection, the
    /// closure, or restoring main. The closure's error takes precedence.
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
    ///
    /// # Arguments
    ///
    /// * `text` — Value whose [`Display`] representation is appended.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if formatting or writing fails,
    /// or the dumper is finalized or has failed.
    pub fn text(&mut self, text: impl Display) -> Result<()> {
        self.ensure_active()?;
        if let Some(recorder) = &mut self.recorder {
            recorder.text(text)?;
        } else {
            self.dump_text(&text.to_string())?;
        }
        Ok(())
    }

    /// Append text without inserting line breaks within it.
    ///
    /// Wraps before the text when it does not fit on a nonempty line. Text that
    /// exceeds a fresh line's width is emitted intact. Empty text does nothing.
    /// Existing line endings are preserved verbatim, not escaped or interpreted.
    ///
    /// # Arguments
    ///
    /// * `text` — String text appended as one unit, accepting `&str` or `String`.
    ///
    /// # Returns
    ///
    /// Returns [`ErrorKind::InvalidInput`] without accepting the text if
    /// it exceeds 65,535 Unicode characters.
    /// Returns `Ok(())`, or an error if writing fails, or the
    /// dumper is finalized or has failed.
    pub fn no_break(&mut self, text: impl AsRef<str>) -> Result<()> {
        self.ensure_active()?;
        if let Some(recorder) = &mut self.recorder {
            recorder.no_break(text)
        } else {
            let text = text.as_ref();
            self.dump_no_break(text, no_break_length(text)?)
        }
    }

    /// Produce composed output without automatic line breaks inside it.
    ///
    /// The region starts at the current position, even if the line is already
    /// full. Automatic wrapping is suppressed; oversized lines overflow intact.
    /// Regions nest, and explicit newlines and context closing breaks are honored.
    /// Contexts opened inside must close inside. Target selection and finalization
    /// are not allowed inside a region. An error permanently fails the dumper;
    /// pending output is discarded. Bytes already written cannot be undone.
    ///
    /// # Arguments
    ///
    /// * `body` — Closure producing text, iterators, or other dump operations.
    ///
    /// # Returns
    ///
    /// Returns the closure's value, or an error from the closure or output.
    /// The closure's original error is preserved. The dumper cannot be reused
    /// after a region error. Panics are not caught.
    ///
    /// # Examples
    ///
    /// ```
    /// use dumpany::core::{Dumper, DumperConfig, Text};
    /// let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
    /// out.no_break_region(|out| {
    ///     out.iterator([Text("return "), Text("a + b"), Text(";")])
    /// })?;
    /// assert_eq!(out.into_inner().unwrap(), b"return a + b;\n");
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn no_break_region<R>(&mut self, body: impl FnOnce(&mut Self) -> Result<R>) -> Result<R> {
        self.begin_no_break_region()?;
        match body(self) {
            Ok(value) => self.end_no_break_region().map(|()| value),
            Err(error) => {
                self.abort_no_break_region();
                Err(error)
            }
        }
    }

    /// Begin a no-break region; prefer [`Self::no_break_region`].
    ///
    /// Returns `Ok(())`, or an error if the dumper is finalized or has failed.
    /// Regions may nest. Pair each call with [`Self::end_no_break_region`].
    pub fn begin_no_break_region(&mut self) -> Result<()> {
        self.ensure_active()?;
        if let Some(capture) = &mut self.region_capture {
            capture.depth += 1;
        } else {
            self.region_capture = Some(RegionCapture {
                main: self.recorder.take(),
                depth: 1,
            });
            self.recorder = Some(Recorder::default());
        }
        Ok(())
    }

    /// Finish the innermost no-break region and resume the enclosing wrapping rules.
    ///
    /// Returns `Ok(())`, an output error, or [`ErrorKind::InvalidInput`] if no
    /// region is open or its contexts remain unclosed. Any error permanently fails
    /// the dumper; output already written cannot be undone.
    pub fn end_no_break_region(&mut self) -> Result<()> {
        self.ensure_active()?;
        let result = (|| {
            let capture = self
                .region_capture
                .as_mut()
                .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "no no-break region is open"))?;
            assert!(
                capture.depth > 0,
                "active region capture must have positive depth"
            );
            capture.depth -= 1;
            if capture.depth != 0 {
                return Ok(());
            }
            let region = self.recorder.take().expect("region recorder exists");
            if region.context_depth() != 0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "unclosed context in no-break region",
                ));
            }
            self.recorder = self.region_capture.take().expect("region is open").main;
            if let Some(parent) = &mut self.recorder {
                parent.no_break_region(region);
            } else {
                let prologues = std::mem::take(&mut self.prologues);
                let epilogues = std::mem::take(&mut self.epilogues);
                self.names.clear();
                self.dump_no_break_region(&prologues, &epilogues, &region)?;
            }
            Ok(())
        })();
        if result.is_err() {
            self.abort_no_break_region();
        }
        result
    }

    /// Permanently fail the dumper after an error while producing a region.
    ///
    /// Pending output is discarded and subsequent operations return an
    /// error. This does not undo bytes already written. Returns nothing.
    pub fn abort_no_break_region(&mut self) {
        self.state = DumperState::Failed;
    }

    fn in_no_break_region(&self) -> bool {
        self.region_capture.is_some()
    }

    /// Dump a sequence with a delimiter between adjacent items.
    ///
    /// An empty sequence emits nothing. See the [module example](self).
    /// Uses [`BreakPolicy::Inherit`]: the join inserts no breaks of its own;
    /// items and delimiters follow surrounding wrapping rules. Iterators and
    /// callbacks run once when the join is dumped.
    ///
    /// # Arguments
    ///
    /// * `items` — Values to dump in iteration order.
    /// * `delimiter` — Value to dump between adjacent items.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success. Stops at the first error from an item or
    /// delimiter. Also returns an error if the dumper is finalized or has failed.
    /// An item or delimiter error permanently fails the dumper.
    pub fn join<I, D>(&mut self, items: I, delimiter: D) -> Result<()>
    where
        I: IntoIterator,
        I::Item: Dump,
        D: Dump,
    {
        self.join_with_breaks(items, delimiter, BreakPolicy::Inherit)
    }

    /// Dump a sequence with configurable breaks between adjacent items.
    ///
    /// # Arguments
    /// * `items` — Items consumed once, in order.
    /// * `delimiter` — Dumped once for each gap, retaining its own explicit breaks.
    /// * `breaks` — Inherit surrounding wrapping, always break, or break as needed.
    ///
    /// Explicit policies keep delimiters attached to preceding items, even if
    /// they overflow. AsNeeded breaks when the next item's first line will not
    /// fit, including its prologue and epilogue content. The first item starts here.
    /// Contexts and regions must balance within each item or delimiter; target
    /// switching and indentation changes are not allowed inside a join.
    /// Violations permanently fail the dumper; output already written cannot be undone.
    ///
    /// Returns `Ok(())`, or an item, delimiter, or output error. Errors are
    /// terminal, and the original error is preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// use dumpany::core::{BreakPolicy, Dumper, DumperConfig, Text};
    /// let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
    /// out.join_with_breaks([Text("one"), Text("two")], Text(", "), BreakPolicy::Always)?;
    /// assert_eq!(out.into_inner().unwrap(), b"one,\ntwo\n");
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn join_with_breaks<I, D>(
        &mut self,
        items: I,
        delimiter: D,
        breaks: BreakPolicy,
    ) -> Result<()>
    where
        I: IntoIterator,
        I::Item: Dump,
        D: Dump,
    {
        self.begin_join(breaks)?;
        let result = (|| {
            let mut iterator = items.into_iter();
            if let Some(first) = iterator.next() {
                first.dump(self)?;
                for item in iterator {
                    self.next_join_part()?;
                    delimiter.dump(self)?;
                    self.next_join_part()?;
                    item.dump(self)?;
                }
            }
            self.end_join()
        })();
        if result.is_err() {
            self.abort_join();
        }
        result
    }

    /// Begin a join with the `breaks` policy; prefer [`Self::join_with_breaks`].
    /// Returns an error if finalized or failed. Pair with [`Self::end_join`].
    pub fn begin_join(&mut self, breaks: BreakPolicy) -> Result<()> {
        self.ensure_active()?;
        let root = self.recorder.is_none();
        let recorder = self.recorder.get_or_insert_with(Recorder::default);
        self.join_captures.push(root);
        recorder.begin_join(breaks);
        Ok(())
    }

    /// Finish the current item or delimiter and start the next alternating part.
    /// Returns `Ok(())`, or an error if the dumper is finalized or has failed.
    pub fn next_join_part(&mut self) -> Result<()> {
        self.ensure_active()?;
        self.recorder
            .get_or_insert_with(Recorder::default)
            .join_part();
        Ok(())
    }

    /// Finish a join and continue with the enclosing output.
    /// Returns an error if parts are unbalanced or end with a delimiter, or output
    /// fails. Any error permanently fails the dumper.
    pub fn end_join(&mut self) -> Result<()> {
        self.ensure_active()?;
        let result = (|| {
            let root = self.join_captures.pop().unwrap_or(self.recorder.is_none());
            self.recorder
                .get_or_insert_with(Recorder::default)
                .end_join();
            if root {
                let recording = self.recorder.take().expect("join recorder exists");
                let prologues = std::mem::take(&mut self.prologues);
                let epilogues = std::mem::take(&mut self.epilogues);
                self.names.clear();
                self.dump_recorder(&prologues, &epilogues, &recording)?;
            }
            Ok(())
        })();
        if result.is_err() {
            self.abort_join();
        }
        result
    }

    /// Permanently fail the dumper after a join callback error and discard pending output.
    pub fn abort_join(&mut self) {
        self.state = DumperState::Failed;
    }

    /// Dump a sequence in order without adding separators.
    ///
    /// # Arguments
    ///
    /// * `items` — Values to dump in iteration order.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or the first error from dumping an item.
    /// Also returns an error if the dumper is finalized or has failed.
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

    /// Dump a nested tuple list without adding separators.
    ///
    /// # Arguments
    ///
    /// * `tuple` — Values arranged as `(head, tail)` pairs ending in `()`.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or the first error from dumping an element.
    /// Also returns an error if the dumper is finalized or has failed.
    ///
    /// # Examples
    ///
    /// ```
    /// use dumpany::core::{Dumper, DumperConfig, Text};
    /// let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
    /// out.tuple((Text("answer: "), (Text(42), ())))?;
    /// let bytes = out.into_inner().map_err(|error| error.into_parts().0)?;
    /// assert_eq!(bytes, b"answer: 42\n");
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn tuple<Tuple: DumpHeadTail>(&mut self, tuple: Tuple) -> Result<()> {
        self.ensure_active()?;
        tuple.dump_head_tail(self)
    }

    /// Increase indentation by one level without inserting a line break.
    ///
    /// Returns `Ok(())`, or an error if the dumper is finalized or has failed.
    pub fn inc_indent(&mut self) -> Result<()> {
        self.ensure_active()?;
        if let Some(recorder) = &mut self.recorder {
            recorder.inc_indent(1)?;
        } else {
            self.indent += 1;
        }
        Ok(())
    }

    /// Decrease indentation by one level without inserting a line break.
    ///
    /// Stops at zero. Returns `Ok(())`, or an error if the dumper is finalized
    /// or has failed.
    pub fn dec_indent(&mut self) -> Result<()> {
        self.ensure_active()?;
        if let Some(recorder) = &mut self.recorder {
            recorder.dec_indent(1)?;
        } else {
            self.indent = self.indent.saturating_sub(1);
        }
        Ok(())
    }

    /// End the current line and optionally adjust indentation for following lines.
    ///
    /// An empty current line still produces a line break.
    ///
    /// # Arguments
    ///
    /// * `change_indentation` — Change applied after the break, or `None` to
    ///   leave indentation unchanged.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if writing fails or the dumper
    /// is finalized or has failed.
    pub fn new_line(&mut self, change_indentation: Option<Indentation>) -> Result<()> {
        self.ensure_active()?;
        if change_indentation.is_some() && self.join_replay_depth > 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "indentation changes not supported inside joins",
            ));
        }
        if let Some(recorder) = &mut self.recorder {
            return recorder.new_line(change_indentation);
        }
        self.flush_line()?;
        self.apply_indentation_change(change_indentation);
        Ok(())
    }

    /// Open an aligned context with an optional prefix and name.
    ///
    /// Continuation lines align with the content after the prefix. Contexts may
    /// nest; their output is emitted when the outermost context closes.
    ///
    /// # Arguments
    ///
    /// * `prefix` — Optional text before the content; must fit on the current line.
    /// * `name` — Optional identifier for selecting a prologue or epilogue. Must
    ///   be unique until the outermost context closes.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, [`ErrorKind::AlreadyExists`] for a duplicate
    /// name, or an error if the dumper is finalized or has failed.
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

    /// Close the current context and optionally adjust indentation for following output.
    ///
    /// Closing the last open main context emits the complete content of all
    /// contexts, including their prologues and epilogues, to the writer.
    /// A context ends its final line when `new_line` is true or an indentation
    /// change is supplied. Otherwise output continues using the enclosing
    /// alignment. Closing without an open context returns an error without
    /// changing indentation or output. Decrements stop at zero.
    ///
    /// # Arguments
    ///
    /// * `indentation` — Change applied after closing, or `None` to leave
    ///   indentation unchanged.
    /// * `new_line` — End the final line. An indentation change forces this,
    ///   including a zero increment or decrement.
    ///
    /// # Returns
    ///
    /// Returns [`ErrorKind::InvalidInput`] if no context is open.
    /// Returns `Ok(())` on success, or an error if output fails, target contexts
    /// remain open when closing the last open main context, a target tries to
    /// close a main context, or the dumper is finalized or has failed.
    pub fn close_context(
        &mut self,
        indentation: Option<Indentation>,
        new_line: bool,
    ) -> Result<()> {
        self.close_current_context(new_line || indentation.is_some())?;
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

    fn close_current_context(&mut self, new_line: bool) -> Result<()> {
        self.ensure_active()?;
        if self
            .recorder
            .as_ref()
            .is_none_or(|recorder| !recorder.can_close_context())
        {
            if self.in_no_break_region() {
                self.abort_no_break_region();
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "cannot close a context outside the no-break region",
                ));
            }
            return Err(Error::new(ErrorKind::InvalidInput, "no context is open"));
        }
        if self.selected_target.is_none() && !self.in_no_break_region() && self.join_captures.is_empty()
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
                recorder.close_context(new_line)?;
                self.selected_target.is_none()
                    && self.region_capture.is_none()
                    && self.join_captures.is_empty()
                    && recorder.context_depth() == 0
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

    fn dump_context(
        &mut self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        context: crate::context::Context<'_>,
        bytes: &mut &[u8],
        recording: &Recorder,
    ) -> Result<()> {
        self.ensure_line_space(1)?;
        self.set_left_margin();
        let mut has_prefix = false;
        if let Some(prefix) = context.prefix {
            // we assume that the prefix is allways

            self.current_line.push_str(prefix);
            if self.no_break_depth == 0  // we must not break so there is no check of line length!
                && self.current_line.chars().count() > self.config.line_width
            {
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
        if let Some(prologue) = context.name.and_then(|name| prologues.get(name)) {
            self.dump_recorder(prologues, epilogues, prologue)?;
        }
        let new_line = self.dump_content(prologues, epilogues, bytes, true, recording)?;
        if let Some(epilogue) = context.name.and_then(|name| epilogues.get(name)) {
            self.dump_recorder(prologues, epilogues, epilogue)?;
        }
        if has_prefix {
            self.release_left_margin();
        }
        self.release_left_margin();
        if new_line {
            self.flush_line()?;
        }
        Ok(())
    }

    fn dump_recorder(
        &mut self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        recorder: &Recorder,
    ) -> Result<()> {
        let mut bytes = recorder.encoded_content();
        self.dump_content(prologues, epilogues, &mut bytes, false, recorder)
            .map(|_| ())
    }

    fn dump_content(
        &mut self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        bytes: &mut &[u8],
        in_context: bool,
        recording: &Recorder,
    ) -> Result<bool> {
        while !bytes.is_empty() {
            match Operation::from_bytes(bytes)? {
                Operation::Join(policy) => {
                    self.join_replay_depth += 1;
                    let result = self.dump_join(prologues, epilogues, policy, bytes, recording);
                    self.join_replay_depth -= 1;
                    if result.is_err() {
                        self.state = DumperState::Failed;
                    }
                    result?;
                }
                Operation::JoinPart | Operation::JoinEnd => {
                    self.state = DumperState::Failed;
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "unexpected join boundary",
                    ));
                }
                Operation::NoBreakRegion(index) => {
                    let region = recording.regions().get(index).ok_or_else(|| {
                        Error::new(ErrorKind::InvalidData, "unknown no-break region")
                    })?;
                    self.dump_no_break_region(prologues, epilogues, region)?
                }
                Operation::Text(text) => self.dump_text(text.0)?,
                Operation::NoBreak { text, length } => self.dump_no_break(text, length)?,
                Operation::NewLine(line) => self.new_line(line.0)?,
                Operation::Indentation(_) if self.join_replay_depth > 0 => {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "indentation changes are forbidden inside joins",
                    ));
                }
                Operation::Indentation(Increment(amount)) => self.indent += amount as usize,
                Operation::Indentation(Decrement(amount)) => {
                    self.indent = self.indent.saturating_sub(amount as usize)
                }
                Operation::OpenContext(context) => {
                    self.dump_context(prologues, epilogues, context, bytes, recording)?;
                }
                Operation::CloseContext(close) if in_context => {
                    return Ok(close.0);
                }
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
            Ok(false)
        }
    }

    // Explicit join policies suppress normal wrapping while emitting each part;
    // only dump_join inserts their boundary breaks, even inside a no-break region.
    fn dump_join(
        &mut self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        policy: BreakPolicy,
        bytes: &mut &[u8],
        recording: &Recorder,
    ) -> Result<()> {
        let mut index = 0;
        loop {
            let (part, end) = crate::operations::join_part(bytes)?;
            if end && index % 2 == 1 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "join ends with a delimiter",
                ));
            }
            if policy == BreakPolicy::Inherit {
                self.dump_content(prologues, epilogues, &mut &part[..], false, recording)?;
            } else {
                if index > 0
                    && index % 2 == 0
                    && policy == BreakPolicy::Always
                    && !self.current_line.trim().is_empty()
                {
                    self.flush_line()?;
                }
                let mut text = self.render_join_part(prologues, epilogues, part, recording)?;
                let first = text
                    .split(&self.config.new_line)
                    .next()
                    .unwrap_or("")
                    .chars()
                    .count();
                if index > 0
                    && index % 2 == 0
                    && policy == BreakPolicy::AsNeeded
                    && first > 0
                    && !self.current_line.trim().is_empty()
                    && self.current_line.chars().count() + first > self.config.line_width
                {
                    self.flush_line()?;
                    // Only recorded operations are replayed; callbacks are never invoked here.
                    text = self.render_join_part(prologues, epilogues, part, recording)?;
                }
                self.no_break_depth += 1;
                let result = self.dump_text(&text);
                self.no_break_depth -= 1;
                result?;
            }
            if end {
                return Ok(());
            }
            index += 1;
        }
    }

    // A virtual prefix gives nested joins and local contexts the actual starting
    // column. Continuations use the enclosing margin, not the prefix length.
    fn render_join_part(
        &self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        part: &[u8],
        recording: &Recorder,
    ) -> Result<String> {
        let margin = self
            .left_margin
            .last()
            .copied()
            .unwrap_or(self.indent * self.config.indent_width);
        let offset = self.current_line.chars().count().saturating_sub(margin);
        let mut temp = Dumper::new(
            Vec::new(),
            DumperConfig {
                new_line: self.config.new_line.clone(),
                indent_width: self.config.indent_width,
                line_width: self.config.line_width.saturating_sub(margin).max(1),
            },
        )?;
        temp.no_break_depth = 1;
        temp.join_replay_depth = 1;
        temp.current_line = "x".repeat(offset);
        temp.dump_content(prologues, epilogues, &mut &part[..], false, recording)?;
        let mut text = String::from_utf8(temp.writer).expect("dumper emits UTF-8");
        text.push_str(&temp.current_line);
        Ok(text[offset..].to_owned())
    }

    // Continue at the current position and replay the region once, without measuring it.
    // Explicit breaks still work; no_break_depth disables only automatic wrapping.
    fn dump_no_break_region(
        &mut self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        region: &Recorder,
    ) -> Result<()> {
        self.no_break_depth += 1;
        let result = self.dump_recorder(prologues, epilogues, region);
        self.no_break_depth -= 1;
        result
    }

    fn dump_no_break(&mut self, text: &str, length: u16) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        self.ensure_line_space(usize::from(length))?;
        self.current_line.push_str(text);
        Ok(())
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
        self.left_margin.push(self.current_line.chars().count());
    }

    fn release_left_margin(&mut self) {
        self.left_margin.pop();
    }

    fn get_indentation_string(&self) -> String {
        " ".repeat(self.indent * self.config.indent_width)
    }
    // Reserve space when possible, allowing overflow on a fresh line.
    fn ensure_line_space(&mut self, required_width: usize) -> Result<()> {
        if self.no_break_depth == 0
            && !self.current_line.trim().is_empty()
            && self
                .current_line
                .chars()
                .count()
                .saturating_add(required_width)
                > self.config.line_width
        {
            self.flush_line()?;
        }
        self.prepare_line_indentation();
        Ok(())
    }

    // Start an empty line at the active context margin or normal indentation.
    // This never checks the width or inserts a line break.
    fn prepare_line_indentation(&mut self) {
        if self.current_line.is_empty() {
            if let Some(left_margin) = self.left_margin.last() {
                self.current_line.push_str(&" ".repeat(*left_margin))
            } else {
                let indentation = self.get_indentation_string();
                self.current_line.push_str(&indentation);
            }
        }
    }

    /// add a line and return false if the delimiter needs to go on the next line
    /// since it did not fit anymore
    fn add_line(&mut self, line: &str, delimiter: char) -> bool {
        self.current_line.push_str(line);
        if self.current_line.chars().count() < self.config.line_width && !delimiter.is_whitespace()
        {
            self.current_line.push(delimiter);
            return true;
        }
        delimiter.is_whitespace() // the delimiter doesn't need to be processed again!
    }

    fn flush_line(&mut self) -> Result<()> {
        self.flush_line_with_new_line(true)
    }

    fn flush_line_with_new_line(&mut self, new_line: bool) -> Result<()> {
        if self.current_line.ends_with(' ') {
            self.current_line = self.current_line.trim_end().into();
        }
        if new_line {
            self.current_line.push_str(&self.config.new_line);
        }
        let result = self.writer.write_all(self.current_line.as_bytes());
        if let Err(error) = result {
            self.state = DumperState::Failed;
            return Err(error);
        }
        self.current_line.clear();
        Ok(())
    }

    fn process_line(&mut self, line: &str, add_new_line: bool) -> Result<()> {
        if self.no_break_depth > 0 {
            self.process_line_unwrapped(line);
        } else {
            self.process_line_wrapped(line)?;
        }
        if add_new_line {
            self.flush_line()?;
        }
        Ok(())
    }

    // Append verbatim: only explicit breaks handled by process_line may flush it.
    fn process_line_unwrapped(&mut self, line: &str) {
        self.prepare_line_indentation();
        self.current_line.push_str(line);
    }

    fn process_line_wrapped(&mut self, line: &str) -> Result<()> {
        let f = |x: char| matches!(x, ' ' | '\t' | ',' | ';');
        let mut end = line.len();
        let mut head = line;
        self.ensure_line_space(1)?;
        while head.chars().count() + self.current_line.chars().count() > self.config.line_width {
            if let Some(mut pos) = head[..end].rfind(&f) {
                if head[..pos].chars().count() + self.current_line.chars().count()
                    <= self.config.line_width
                {
                    if self.add_line(&head[..pos], head[pos..].chars().next().unwrap()) {
                        pos += 1;
                    }
                    self.flush_line()?;
                    self.ensure_line_space(1)?;
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
        Ok(())
    }
}
