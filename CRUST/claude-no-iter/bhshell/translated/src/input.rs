use crate::dynamicarr;
use std::io::{self, Read};

/// Represents a command, including its arguments, pipe-arguments,
/// and redirection information, in a safe Rust form.
#[derive(Debug, Default)]
pub struct Command {
    pub args: Vec<String>,
    pub pipe_args: Vec<String>,
    pub redirect_file_name: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ArgType {
    Arg,
    PipeArg,
    Redirect,
}

/// Reads a line of input from the user.
/// In C, this was returning a 'char*'.
pub fn bhshell_read_line() -> String {
    let mut s = dynamicarr::Str::default();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    loop {
        match handle.read(&mut byte) {
            Ok(0) => {
                // EOF
                return dynamicarr::get_string(&s);
            }
            Ok(_) => {
                let c = byte[0] as char;
                if c == '\n' {
                    return dynamicarr::get_string(&s);
                }
                s.items.push(c);
                s.position += 1;
            }
            Err(_) => {
                return dynamicarr::get_string(&s);
            }
        }
    }
}

/// Parses a given line into a Command structure.
/// In C, this was returning a 'command*' for the allocated command.
pub fn bhshell_parse(line: &str) -> Command {
    let bytes = line.as_bytes();
    let length = bytes.len();
    let mut args: Vec<String> = Vec::new();
    let mut pipe_args: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;
    let cmd_empty = Command::default();

    let mut i: usize = 0;
    while i < length {
        let ch = bytes[i] as char;
        if ch == '\n' || ch == '\t' || ch == ' ' {
            if !buf.is_empty() {
                let s = std::mem::take(&mut buf);
                match current {
                    ArgType::Arg => args.push(s),
                    ArgType::PipeArg => pipe_args.push(s),
                    ArgType::Redirect => redirect = Some(s),
                }
            }
            i += 1;
            continue;
        } else if ch == '|' {
            if !buf.is_empty() {
                let s = std::mem::take(&mut buf);
                if current == ArgType::Arg {
                    args.push(s);
                } else {
                    // Invalid: pipe encountered while not in ARG mode with content
                    return cmd_empty;
                }
            }
            // Mirror the C check: `if i+1 < length && line[i] == '>'` is
            // effectively dead in the C code (since line[i] is always '|' here),
            // so we don't do anything special.
            current = ArgType::PipeArg;
            i += 1;
            continue;
        } else if ch == '>' {
            if !buf.is_empty() {
                let s = std::mem::take(&mut buf);
                match current {
                    ArgType::Arg => args.push(s),
                    ArgType::PipeArg => pipe_args.push(s),
                    ArgType::Redirect => {
                        // C didn't handle this case explicitly; preserve behavior
                        // by treating as redirect overwrite.
                        redirect = Some(s);
                    }
                }
            }
            current = ArgType::Redirect;
            i += 1;
            continue;
        } else {
            buf.push(ch);
            i += 1;
        }
    }

    // If buf is empty at end (s.position == 0 in C), invalid.
    if buf.is_empty() {
        return cmd_empty;
    }

    // Otherwise, flush remaining buffer.
    let last = std::mem::take(&mut buf);
    match current {
        ArgType::Arg => args.push(last),
        ArgType::PipeArg => pipe_args.push(last),
        ArgType::Redirect => redirect = Some(last),
    }

    if args.is_empty() {
        return cmd_empty;
    }

    let mut cmd = new_command();
    cmd.args = args;
    if !pipe_args.is_empty() {
        cmd.pipe_args = pipe_args;
    }
    if let Some(r) = redirect {
        cmd.redirect_file_name = Some(r);
    }
    cmd
}

/// Cleans up and destroys a Command structure.
/// In C, this took 'command* cmd' and freed its data.
pub fn destroy_command(_cmd: Command) {
    // Rust drops the Command and its fields automatically.
}

/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command {
        args: Vec::new(),
        pipe_args: Vec::new(),
        redirect_file_name: None,
    }
}
