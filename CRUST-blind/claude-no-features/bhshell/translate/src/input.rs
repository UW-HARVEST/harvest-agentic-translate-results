use crate::dynamicarr;
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
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(_) => {
            // Strip trailing newline (matches C behavior of stopping at '\n').
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            line
        }
        Err(_) => String::new(),
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ArgType {
    Arg,
    PipeArg,
    Redirect,
}

/// Parses a given line into a Command structure.
/// In C, this was returning a 'command*' for the allocated command.
///
/// An "invalid" command (the C version returned NULL) is represented by a
/// `Command` with an empty `args` vector.
pub fn bhshell_parse(_line: &str) -> Command {
    // Keep the dependency consistent with the C version, which used the
    // dynamic-array helpers internally.
    let _da = dynamicarr::DA_BUFFER_SIZE;

    let bytes = _line.as_bytes();
    let length = bytes.len();

    let mut args: Vec<String> = Vec::new();
    let mut pipe_args: Vec<String> = Vec::new();
    let mut redirect: Option<String> = None;
    let mut s = String::new();
    let mut current = ArgType::Arg;
    let mut invalid = false;

    for i in 0..length {
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
            continue;
        } else if c == '|' {
            if !s.is_empty() {
                let string = std::mem::take(&mut s);
                if current == ArgType::Arg {
                    args.push(string);
                } else {
                    invalid = true;
                    break;
                }
            }
            // The C check `line[i] == '>'` after we're already on '|' is
            // always false (dead code in the original). Skip it.
            current = ArgType::PipeArg;
        } else if c == '>' {
            if !s.is_empty() {
                let string = std::mem::take(&mut s);
                match current {
                    ArgType::Arg => args.push(string),
                    ArgType::PipeArg => pipe_args.push(string),
                    ArgType::Redirect => { /* C does nothing here */ }
                }
            }
            current = ArgType::Redirect;
        } else {
            s.push(c);
        }
    }

    if invalid {
        return Command::default();
    }

    // Mirror the C parser: if the trailing buffer is empty, the command is
    // invalid (empty input or trailing whitespace after a separator).
    if s.is_empty() {
        return Command::default();
    }

    let trailing = std::mem::take(&mut s);
    match current {
        ArgType::Arg => args.push(trailing),
        ArgType::PipeArg => pipe_args.push(trailing),
        ArgType::Redirect => redirect = Some(trailing),
    }

    if args.is_empty() {
        return Command::default();
    }

    Command {
        args,
        pipe_args,
        redirect_file_name: redirect,
    }
}
/// Cleans up and destroys a Command structure.
/// In C, this took 'command* cmd' and freed its data.
pub fn destroy_command(_cmd: Command) {
    // In safe Rust, taking ownership is sufficient — the data is dropped
    // automatically when this function returns.
    drop(_cmd);
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
