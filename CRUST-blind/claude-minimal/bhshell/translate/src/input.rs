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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgType {
    Arg,
    PipeArg,
    Redirect,
}

/// Reads a line of input from the user.
/// In C, this was returning a 'char*'.
pub fn bhshell_read_line() -> String {
    let mut s = dynamicarr::Str::default();
    let mut buf = [0u8; 1];
    loop {
        match io::stdin().read(&mut buf) {
            Ok(0) => {
                // EOF
                return dynamicarr::get_string(&s);
            }
            Ok(_) => {
                let c = buf[0] as char;
                if c == '\n' {
                    return dynamicarr::get_string(&s);
                } else {
                    s.items.push(c);
                    s.position += 1;
                }
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
    let mut s = String::new();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;
    let mut cmd = new_command();

    let mut i = 0;
    while i < length {
        let c = bytes[i] as char;
        if c == '\n' || c == '\t' || c == ' ' {
            if !s.is_empty() {
                let string = std::mem::take(&mut s);
                match current {
                    ArgType::Arg => args.push(string),
                    ArgType::PipeArg => pipe_args.push(string),
                    ArgType::Redirect => redirect = Some(string),
                }
            }
            i += 1;
            continue;
        } else if c == '|' {
            if !s.is_empty() {
                let string = std::mem::take(&mut s);
                if current == ArgType::Arg {
                    args.push(string);
                } else {
                    // Invalid: pipe encountered while not in ARG state.
                    return Command::default();
                }
            }
            // Mirrors the C check (which is logically suspect but kept here):
            // if (i + 1 < length && line[i] == '>') { invalid }
            // The C check compares line[i] to '>', which is always '|' here, so
            // it never triggers. We preserve that behavior.
            current = ArgType::PipeArg;
        } else if c == '>' {
            if !s.is_empty() {
                let string = std::mem::take(&mut s);
                match current {
                    ArgType::Arg => args.push(string),
                    ArgType::PipeArg => pipe_args.push(string),
                    ArgType::Redirect => {}
                }
            }
            current = ArgType::Redirect;
        } else {
            s.push(c);
        }
        i += 1;
    }

    if s.is_empty() {
        // Mirror the C behavior: if the trailing buffer is empty here, the
        // command is considered invalid.
        return Command::default();
    }

    // s is not empty: flush it according to the current state.
    let string = std::mem::take(&mut s);
    match current {
        ArgType::Arg => args.push(string),
        ArgType::PipeArg => pipe_args.push(string),
        ArgType::Redirect => redirect = Some(string),
    }

    if args.is_empty() {
        return Command::default();
    }

    cmd.args = args;
    if !pipe_args.is_empty() {
        cmd.pipe_args = pipe_args;
    }
    if redirect.is_some() {
        cmd.redirect_file_name = redirect;
    }
    cmd
}

/// Cleans up and destroys a Command structure.
/// In C, this took 'command* cmd' and freed its data.
pub fn destroy_command(_cmd: Command) {
    // In Rust, dropping the Command frees the contained Vecs and Strings.
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
