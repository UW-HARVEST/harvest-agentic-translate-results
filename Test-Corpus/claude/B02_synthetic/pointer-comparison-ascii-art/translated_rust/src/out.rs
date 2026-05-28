// out.rs - C-like buffered stdout/stderr.
//
// C's printf() uses fully-buffered stdout when redirected to a non-terminal
// (e.g. a file or a pipe), and line-buffered when stdout is a terminal.
// To match byte-identical output and ordering when both stdout and stderr
// are captured together, we mimic C's behavior:
//   - stdout: block-buffered when not a terminal, line-buffered when terminal
//   - stderr: unbuffered (printed immediately)
//   - flushed on exit and before reading stdin

use std::io::{self, IsTerminal, Write};
use std::sync::{Mutex, OnceLock};

// BUFSIZ is typically 8192 on glibc.
const BUFSIZ: usize = 8192;

struct StdoutState {
    buf: Vec<u8>,
    is_tty: bool,
}

fn state() -> &'static Mutex<StdoutState> {
    static S: OnceLock<Mutex<StdoutState>> = OnceLock::new();
    S.get_or_init(|| {
        let is_tty = io::stdout().is_terminal();
        Mutex::new(StdoutState {
            buf: Vec::with_capacity(BUFSIZ),
            is_tty,
        })
    })
}

pub fn cout_print(data: &[u8]) {
    let m = state();
    let mut g = m.lock().unwrap();
    g.buf.extend_from_slice(data);
    if g.is_tty {
        // Line-buffered: flush at each newline
        if let Some(pos) = g.buf.iter().rposition(|&b| b == b'\n') {
            let to_write = g.buf[..=pos].to_vec();
            let stdout = io::stdout();
            let mut h = stdout.lock();
            let _ = h.write_all(&to_write);
            let _ = h.flush();
            g.buf.drain(..=pos);
        }
    } else {
        // Block-buffered: flush when buffer reaches BUFSIZ
        while g.buf.len() >= BUFSIZ {
            let to_write: Vec<u8> = g.buf.drain(..BUFSIZ).collect();
            let stdout = io::stdout();
            let mut h = stdout.lock();
            let _ = h.write_all(&to_write);
            let _ = h.flush();
        }
    }
}

pub fn cout_flush() {
    let m = state();
    let mut g = m.lock().unwrap();
    if !g.buf.is_empty() {
        let to_write = std::mem::take(&mut g.buf);
        let stdout = io::stdout();
        let mut h = stdout.lock();
        let _ = h.write_all(&to_write);
        let _ = h.flush();
    }
}

/// Flush only when stdout is a terminal (line-buffered mode). This matches
/// C's behavior: glibc's _IO_flush_all_lockfp_helper is called before
/// reading stdin from a TTY, but only line-buffered streams are flushed.
pub fn cout_flush_if_tty() {
    let m = state();
    let mut g = m.lock().unwrap();
    if g.is_tty && !g.buf.is_empty() {
        let to_write = std::mem::take(&mut g.buf);
        let stdout = io::stdout();
        let mut h = stdout.lock();
        let _ = h.write_all(&to_write);
        let _ = h.flush();
    }
}

// stderr: unbuffered. Write immediately.
pub fn cerr_print(data: &[u8]) {
    let stderr = io::stderr();
    let mut h = stderr.lock();
    let _ = h.write_all(data);
}

// Helper macros for printing
#[macro_export]
macro_rules! cprint {
    ($($arg:tt)*) => {{
        let s = format!($($arg)*);
        $crate::out::cout_print(s.as_bytes());
    }};
}

#[macro_export]
macro_rules! cprintln {
    ($($arg:tt)*) => {{
        let s = format!($($arg)*);
        $crate::out::cout_print(s.as_bytes());
        $crate::out::cout_print(b"\n");
    }};
}

#[macro_export]
macro_rules! ceprint {
    ($($arg:tt)*) => {{
        let s = format!($($arg)*);
        $crate::out::cerr_print(s.as_bytes());
    }};
}

#[macro_export]
macro_rules! ceprintln {
    ($($arg:tt)*) => {{
        let s = format!($($arg)*);
        $crate::out::cerr_print(s.as_bytes());
        $crate::out::cerr_print(b"\n");
    }};
}
