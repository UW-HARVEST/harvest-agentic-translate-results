use crate::dynamicarr::{ArgList, Str};
use std::io::{self, Read};

/// Represents a command, including its arguments, pipe-arguments,
/// and redirection information, in a safe Rust form.
#[derive(Debug, Default)]
pub struct Command {
    pub args: Vec<String>,
    pub pipe_args: Vec<String>,
    pub redirect_file_name: Option<String>,
}
/// Reads a line of input from the user.
/// In C, this was returning a 'char*'.
pub fn bhshell_read_line() -> String {
    let mut s = String::new();
    let stdin = io::stdin();
    let mut buf = [0u8; 1];
    loop {
        match stdin.lock().read(&mut buf) {
            Ok(0) => return s, // EOF
            Ok(_) => {
                let c = buf[0] as char;
                if c == '\n' {
                    return s;
                }
                s.push(c);
            }
            Err(_) => return s,
        }
    }
}
/// Parses a given line into a Command structure.
/// In C, this was returning a 'command*' for the allocated command.
///
/// In this Rust port we represent a parse failure (NULL in C) by returning a
/// Command with an empty `args` vector. The shell loop checks for this and
/// prints "Invalid Command" before continuing.
pub fn bhshell_parse(line: &str) -> Command {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ArgType {
        Arg,
        PipeArg,
        Redirect,
    }

    let mut args = ArgList::default();
    let mut pipe_args = ArgList::default();
    let mut s = Str::default();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;

    let chars: Vec<char> = line.chars().collect();
    let length = chars.len();

    for i in 0..length {
        let ch = chars[i];
        if ch == '\n' || ch == '\t' || ch == ' ' {
            if s.position > 0 {
                let string = take_string(&mut s);
                match current {
                    ArgType::Arg => args.append(string),
                    ArgType::PipeArg => pipe_args.append(string),
                    ArgType::Redirect => redirect = Some(string),
                }
            }
            continue;
        } else if ch == '|' {
            if s.position > 0 {
                let string = take_string(&mut s);
                if current == ArgType::Arg {
                    args.append(string);
                    // mirror the trailing-NULL append in C: in Rust we
                    // simply don't push a NULL marker since Vec<String>
                    // has explicit length information.
                } else {
                    return Command::default();
                }
            }
            // C check: `if (i + 1 < length && line[i] == '>')` — note that
            // the comparison is against `line[i]`, not `line[i+1]`, so the
            // condition is effectively impossible (we already know
            // line[i] == '|'). We preserve the (no-op) behaviour here.
            current = ArgType::PipeArg;
        } else if ch == '>' {
            if s.position > 0 {
                let string = take_string(&mut s);
                match current {
                    ArgType::Arg => args.append(string),
                    ArgType::PipeArg => pipe_args.append(string),
                    ArgType::Redirect => {
                        // C: only branches handled were ARG and PIPE_ARG;
                        // for REDIRECT it does nothing here.
                    }
                }
            }
            current = ArgType::Redirect;
        } else {
            s.append(ch);
        }
    }

    if s.position == 0 {
        // Trailing whitespace / nothing in the buffer at end.
        return Command::default();
    }

    // s.position > 0 here.
    let string = take_string(&mut s);
    match current {
        ArgType::Arg => args.append(string),
        ArgType::PipeArg => pipe_args.append(string),
        ArgType::Redirect => redirect = Some(string),
    }

    if args.position == 0 {
        return Command::default();
    }

    let mut cmd = new_command();
    cmd.args = args.items;
    if pipe_args.position > 0 {
        cmd.pipe_args = pipe_args.items;
    }
    if redirect.is_some() {
        cmd.redirect_file_name = redirect;
    }
    cmd
}

/// Helper: extracts the accumulated string from a `Str`, resetting it.
fn take_string(s: &mut Str) -> String {
    let out = std::mem::take(&mut s.items);
    s.position = 0;
    s.bufsize = 0;
    out
}

/// Cleans up and destroys a Command structure.
/// In C, this took 'command* cmd' and freed its data.
pub fn destroy_command(_cmd: Command) {
    // Rust automatically frees the Command and all owned strings on drop.
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
