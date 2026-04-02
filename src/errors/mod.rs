use std::fmt;
use std::path::PathBuf;

// ── Source Location ──────────────────────────────────────────────

/// A span representing a region of source code.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub file: PathBuf,
    pub line: u32,
    pub col: u32,
    pub len: u32,
}

impl Span {
    pub fn new(file: PathBuf, line: u32, col: u32, len: u32) -> Self {
        Self {
            file,
            line,
            col,
            len,
        }
    }

    /// A dummy span for compiler-generated nodes.
    pub fn dummy() -> Self {
        Self {
            file: PathBuf::from("<internal>"),
            line: 0,
            col: 0,
            len: 0,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file.display(), self.line, self.col)
    }
}

// ── Diagnostic Severity ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Level {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Error => write!(f, "error"),
            Level::Warning => write!(f, "warning"),
            Level::Note => write!(f, "note"),
        }
    }
}

// ── Diagnostic ───────────────────────────────────────────────────

/// A single compiler diagnostic message with optional notes.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub span: Span,
    pub level: Level,
    pub message: String,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            level: Level::Error,
            message: message.into(),
            notes: vec![],
        }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            level: Level::Warning,
            message: message.into(),
            notes: vec![],
        }
    }

    pub fn note(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            level: Level::Note,
            message: message.into(),
            notes: vec![],
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Render the diagnostic as a formatted string with ANSI colors.
    /// If `source` is provided, shows the offending source line with a caret.
    pub fn render(&self, source: Option<&str>) -> String {
        let color = match self.level {
            Level::Error => "\x1b[31m",   // red
            Level::Warning => "\x1b[33m", // yellow
            Level::Note => "\x1b[36m",    // cyan
        };
        let reset = "\x1b[0m";
        let bold = "\x1b[1m";

        let mut out = format!(
            "{bold}{color}{}{reset}: {bold}{}{reset}\n  {bold}-->{reset} {}\n",
            self.level, self.message, self.span
        );

        // Show source context if available
        if let Some(src) = source {
            if self.span.line > 0 {
                if let Some(line_str) = src.lines().nth((self.span.line - 1) as usize) {
                    let line_num = format!("{}", self.span.line);
                    let padding = " ".repeat(line_num.len());

                    out += &format!("{padding} |\n");
                    out += &format!("{line_num} | {line_str}\n");

                    if self.span.col > 0 {
                        let caret_pad = " ".repeat((self.span.col - 1) as usize);
                        let carets = "^".repeat(std::cmp::max(1, self.span.len as usize));
                        out += &format!("{padding} | {caret_pad}{color}{carets}{reset}\n");
                    }
                }
            }
        }

        // Append notes
        for note in &self.notes {
            out += &format!("  {bold}{color}={reset} {bold}note{reset}: {note}\n");
        }

        out
    }
}

// ── Diagnostic Collector ─────────────────────────────────────────

/// Collects diagnostics during compilation and reports them.
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
    source_cache: std::collections::HashMap<PathBuf, String>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            source_cache: std::collections::HashMap::new(),
        }
    }

    /// Register a source file's contents for error rendering.
    pub fn register_source(&mut self, path: PathBuf, source: String) {
        self.source_cache.insert(path, source);
    }

    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn error(&mut self, span: Span, message: impl Into<String>) {
        self.add(Diagnostic::error(span, message));
    }

    pub fn warning(&mut self, span: Span, message: impl Into<String>) {
        self.add(Diagnostic::warning(span, message));
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == Level::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.level == Level::Error)
            .count()
    }

    /// Print all diagnostics to stderr.
    pub fn emit_all(&self) {
        for diag in &self.diagnostics {
            let source = self.source_cache.get(&diag.span.file).map(|s| s.as_str());
            eprint!("{}", diag.render(source));
        }

        let errors = self.error_count();
        if errors > 0 {
            let warnings = self
                .diagnostics
                .iter()
                .filter(|d| d.level == Level::Warning)
                .count();
            eprintln!(
                "\x1b[1m\x1b[31merror\x1b[0m: compilation failed with {} error{} and {} warning{}",
                errors,
                if errors == 1 { "" } else { "s" },
                warnings,
                if warnings == 1 { "" } else { "s" },
            );
        }
    }
}
