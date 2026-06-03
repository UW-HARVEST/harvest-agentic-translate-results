use std::io::{self, BufRead};

/// Represents a command, including its arguments, pipe-arguments,
/// and redirection information, in a safe Rust form.
#[derive(Debug, Default)]
pub struct Command {
    pub args: Vec<String>,
    pub pipe_args: Vec<String>,
    pub redirect_file_name: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ArgType {
    Arg,
    PipeArg,
    Redirect,
}

/// Reads a line of input from the user.
/// In C, this was returning a 'char*'.
pub fn bhshell_read_line() -> String {
    let stdin = io::stdin();
    let mut line = String::new();
    // read until newline or EOF; trim the trailing newline if present
    let _ = stdin.lock().read_line(&mut line);
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    line
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

    let mut i = 0;
    while i < length {
        let c = bytes[i] as char;
        if c == '\n' || c == '\t' || c == ' ' {
            if !buf.is_empty() {
                let s = std::mem::take(&mut buf);
                match current {
                    ArgType::Arg => args.push(s),
                    ArgType::PipeArg => pipe_args.push(s),
                    ArgType::Redirect => redirect = Some(s),
                }
            }
        } else if c == '|' {
            if !buf.is_empty() {
                let s = std::mem::take(&mut buf);
                if current == ArgType::Arg {
                    args.push(s);
                } else {
                    return Command::default();
                }
            }
            // Mirror the C check: if the next character is '>'
            if i + 1 < length && bytes[i] as char == '>' {
                // C bug: this checks line[i] which is '|', so never triggers,
                // but kept for fidelity to the C code.
                return Command::default();
            }
            current = ArgType::PipeArg;
        } else if c == '>' {
            if !buf.is_empty() {
                let s = std::mem::take(&mut buf);
                match current {
                    ArgType::Arg => args.push(s),
                    ArgType::PipeArg => pipe_args.push(s),
                    ArgType::Redirect => {}
                }
            }
            current = ArgType::Redirect;
        } else {
            buf.push(c);
        }
        i += 1;
    }

    if buf.is_empty() {
        // matches "if (s.position == 0) return NULL"
        return Command::default();
    }

    // Flush remaining buffer
    let s = std::mem::take(&mut buf);
    match current {
        ArgType::Arg => args.push(s),
        ArgType::PipeArg => pipe_args.push(s),
        ArgType::Redirect => redirect = Some(s),
    }

    if args.is_empty() {
        return Command::default();
    }

    let mut cmd = new_command();
    cmd.args = args;
    cmd.pipe_args = pipe_args;
    cmd.redirect_file_name = redirect;
    cmd
}

/// Cleans up and destroys a Command structure.
/// In C, this took 'command* cmd' and freed its data.
pub fn destroy_command(_cmd: Command) {
    // In Rust, memory is automatically freed when the Command is dropped.
}

/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command::default()
}
