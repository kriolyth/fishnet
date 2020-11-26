use std::sync::{Arc, Mutex};
use std::fmt;
use std::io;
use std::io::Write as _;
use std::cmp::{min, max};
use atty::Stream;
use url::Url;
use crate::ipc::{BatchId, PositionId, PositionResponse};
use crate::configure::Verbose;

#[derive(Clone)]
pub struct Logger {
    verbose: Verbose,
    stderr: bool,
    atty: bool,
    state: Arc<Mutex<LoggerState>>,
}

impl Logger {
    pub fn new(verbose: Verbose, stderr: bool) -> Logger {
        Logger {
            verbose,
            stderr,
            atty: atty::is(Stream::Stdout),
            state: Arc::new(Mutex::new(LoggerState {
                progress_line: 0,
            })),
        }
    }

    fn println(&self, line: &str) {
        let mut state = self.state.lock().expect("logger state");
        state.line_feed();

        if self.stderr {
            eprintln!("{}", line);
        } else {
            println!("{}", line);
        }
    }

    pub fn clear_echo(&self) {
        let mut state = self.state.lock().expect("logger state");
        state.line_feed();
    }

    pub fn headline(&self, title: &str) {
        self.println(&format!("\n### {}\n", title));
    }

    pub fn debug(&self, line: &str) {
        if self.verbose.level > 0 {
            self.println(&format!("D: {}", line));
        }
    }

    pub fn info(&self, line: &str) {
        self.println(line);
    }

    pub fn fishnet_info(&self, line: &str) {
        self.println(&format!("><> {}", line));
    }

    pub fn warn(&self, line: &str) {
        self.println(&format!("W: {}", line));
    }

    pub fn error(&self, line: &str) {
        self.println(&format!("E: {}", line));
    }

    pub fn progress<P>(&self, queue: QueueStatusBar, progress: P)
        where P: Into<ProgressAt>,
    {
        let line = format!("{}, latest: {}", queue, progress.into());
        if self.atty {
            let mut state = self.state.lock().expect("logger state");
            print!("\r{}{}", line, " ".repeat(state.progress_line.saturating_sub(line.len())));
            io::stdout().flush().expect("flush stdout");
            state.progress_line = line.len();
        } else if self.verbose.level > 0 {
            println!("{}", line);
        }
    }
}

pub struct ProgressAt {
    pub batch_id: BatchId,
    pub batch_url: Option<Url>,
    pub position_id: Option<PositionId>,
}

impl fmt::Display for ProgressAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref batch_url) = self.batch_url {
            let mut url = batch_url.clone();
            if let Some(PositionId(positon_id)) = self.position_id {
                url.set_fragment(Some(&positon_id.to_string()));
            }
            fmt::Display::fmt(&url, f)
        } else {
            write!(f, "{}", self.batch_id)?;
            if let Some(PositionId(positon_id)) = self.position_id {
                write!(f, "#{}", positon_id)?;
            }
            Ok(())
        }
    }
}

impl From<&PositionResponse> for ProgressAt {
    fn from(pos: &PositionResponse) -> ProgressAt {
        ProgressAt {
            batch_id: pos.batch_id,
            batch_url: pos.url.clone(),
            position_id: Some(pos.position_id),
        }
    }
}

struct LoggerState {
    pub progress_line: usize,
}

impl LoggerState {
    fn line_feed(&mut self) {
        if self.progress_line > 0 {
            self.progress_line = 0;
            println!();
        }
    }
}

pub struct QueueStatusBar {
    pub pending: usize,
    pub cores: usize,
}

impl fmt::Display for QueueStatusBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let width = 16;
        let virtual_width = max(self.cores * 2, 16);
        let cores_width = self.cores * width / virtual_width;
        let pending_width = min(width, self.pending * width / virtual_width);
        let overhang_width = pending_width.saturating_sub(cores_width);

        f.write_str("[")?;
        f.write_str(&"=".repeat(min(pending_width, cores_width)))?;
        f.write_str(&" ".repeat(cores_width.saturating_sub(pending_width)))?;
        f.write_str("|")?;
        f.write_str(&"=".repeat(overhang_width))?;
        f.write_str(&" ".repeat(width.saturating_sub(cores_width).saturating_sub(overhang_width)))?;
        write!(f, "] {} cores / {} queued", self.cores, self.pending)
    }
}
