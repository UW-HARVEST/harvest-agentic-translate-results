use crate::dynamicarr::{self, ArgList, Str};
use std::io::{self, Read};

/// Represents a command, including its arguments, pipe-arguments,
/// and redirection information, in a safe Rust form.
#[derive(Debug, Default)]
pub struct Command {
    pub args: Vec<String>,
    pub pipe_args: Vec<String>,
    pub redirect_file_name: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum ArgType {
    Arg,
    PipeArg,
    Redirect,
}

/// Reads a line of input from the user.
/// In C, this was returning a 'char*'.
pub fn bhshell_read_line() -> String {
    let mut s = Str::default();
    let mut buf = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    loop {
        match handle.read(&mut buf) {
            Ok(0) => {
                // EOF
                return dynamicarr::take_string(&mut s);
            }
            Ok(_) => {
                let c = buf[0] as char;
                if c == '\n' {
                    return dynamicarr::take_string(&mut s);
                } else {
                    dynamicarr::da_append_str(&mut s, c);
                }
            }
            Err(_) => {
                return dynamicarr::take_string(&mut s);
            }
        }
    }
}

/// Parses a given line into a Command structure.
/// In C, this was returning a 'command*' for the allocated command.
pub fn bhshell_parse(line: &str) -> Command {
    let bytes = line.as_bytes();
    let length = bytes.len();
    let mut args = ArgList::default();
    let mut pipe_args = ArgList::default();
    let mut s = Str::default();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;
    let mut cmd = new_command();

    let mut i = 0;
    while i < length {
        let c = bytes[i] as char;
        if c == '\n' || c == '\t' || c == ' ' {
            if s.position > 0 {
                let string = dynamicarr::take_string(&mut s);
                match current {
                    ArgType::Arg => dynamicarr::da_append_arglist(&mut args, string),
                    ArgType::PipeArg => dynamicarr::da_append_arglist(&mut pipe_args, string),
                    ArgType::Redirect => redirect = Some(string),
                }
            }
            i += 1;
            continue;
        } else if c == '|' {
            if s.position > 0 {
                let string = dynamicarr::take_string(&mut s);
                if current == ArgType::Arg {
                    dynamicarr::da_append_arglist(&mut args, string);
                } else {
                    return Command::default();
                }
            }
            // The C code has a buggy check: `line[i] == '>'`, but we replicate it
            // faithfully — it can never trigger at the same position.
            current = ArgType::PipeArg;
        } else if c == '>' {
            if s.position > 0 {
                let string = dynamicarr::take_string(&mut s);
                match current {
                    ArgType::Arg => dynamicarr::da_append_arglist(&mut args, string),
                    ArgType::PipeArg => dynamicarr::da_append_arglist(&mut pipe_args, string),
                    ArgType::Redirect => {}
                }
            }
            current = ArgType::Redirect;
        } else {
            dynamicarr::da_append_str(&mut s, c);
        }
        i += 1;
    }

    // If we never saw a non-whitespace token, we mirror the C check that
    // returned NULL on `s.position == 0` — but only when we also never
    // collected any args.  The C behaviour is: if the last segment is empty
    // AND no args were collected, return NULL.  We translate that to "if
    // there are no args produced at all, return an empty command".
    if s.position == 0 {
        // The C code unconditionally returns NULL here.  We do the same: if
        // the input ends without a final non-whitespace token, the parse is
        // considered invalid.
        return Command::default();
    }

    // Final non-empty token.
    let string = dynamicarr::take_string(&mut s);
    match current {
        ArgType::Arg => dynamicarr::da_append_arglist(&mut args, string),
        ArgType::PipeArg => dynamicarr::da_append_arglist(&mut pipe_args, string),
        ArgType::Redirect => redirect = Some(string),
    }

    if args.position == 0 {
        return Command::default();
    }

    cmd.args = args.items.into_iter().take(args.position).collect();
    if pipe_args.position > 0 {
        cmd.pipe_args = pipe_args.items.into_iter().take(pipe_args.position).collect();
    }
    cmd.redirect_file_name = redirect;
    cmd
}

/// Cleans up and destroys a Command structure.
/// In C, this took 'command* cmd' and freed its data.
pub fn destroy_command(_cmd: Command) {
    // Drop semantics in Rust handle this automatically.
}

/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command::default()
}
