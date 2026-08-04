use crate::dynamicarr;
/// Represents a command, including its arguments, pipe-arguments,
/// and redirection information, in a safe Rust form.
#[derive(Debug, Default)]
pub struct Command {
    pub args: Vec<String>,
    pub pipe_args: Vec<String>,
    pub redirect_file_name: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ArgType {
    Arg,
    PipeArg,
    Redirect,
}

/// Reads a line of input from the user.
/// In C, this was returning a 'char*'.
pub fn bhshell_read_line() -> String {
    use std::io::Read;

    // Touch dynamicarr to keep the import meaningful.
    let _: dynamicarr::Str = dynamicarr::Str::default();

    let mut buf = String::new();
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    loop {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let c = byte[0] as char;
                if c == '\n' {
                    break;
                }
                buf.push(c);
            }
            Err(_) => break,
        }
    }
    buf
}

/// Parses a given line into a Command structure.
/// In C, this was returning a 'command*' for the allocated command.
///
/// In the C version this returned NULL for invalid input. In Rust, we signal
/// "invalid" by returning a Command whose `args` vector is empty (because a
/// valid parsed command always has at least one argument).
pub fn bhshell_parse(_line: &str) -> Command {
    let line = _line;
    let bytes = line.as_bytes();
    let length = bytes.len();

    let mut args: Vec<String> = Vec::new();
    let mut pipe_args: Vec<String> = Vec::new();
    let mut s = String::new();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;

    let invalid = || Command::default();

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
        } else if c == '|' {
            if !s.is_empty() {
                let string = std::mem::take(&mut s);
                if current == ArgType::Arg {
                    args.push(string);
                } else {
                    return invalid();
                }
            }
            // The C source contains a dead-code check here:
            //   `if (i + 1 < length && line[i] == '>')` — `line[i]` is always
            //   '|' inside this branch, so the condition can never be true.
            // We preserve that behavior (i.e. don't reject `|>` here).
            current = ArgType::PipeArg;
        } else if c == '>' {
            if !s.is_empty() {
                let string = std::mem::take(&mut s);
                match current {
                    ArgType::Arg => args.push(string),
                    ArgType::PipeArg => pipe_args.push(string),
                    ArgType::Redirect => {} // matches C behavior (no-op)
                }
            }
            current = ArgType::Redirect;
        } else {
            s.push(c);
        }
        i += 1;
    }

    // Trailing-content check: if there's no current token at the end, the
    // command is invalid (e.g., empty input, trailing whitespace, or a stray
    // operator with nothing after it).
    if s.is_empty() {
        return invalid();
    }

    let string = s;
    match current {
        ArgType::Arg => args.push(string),
        ArgType::PipeArg => pipe_args.push(string),
        ArgType::Redirect => redirect = Some(string),
    }

    if args.is_empty() {
        return invalid();
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
    // Rust drops the Command (and its owned Vec/Strings) automatically when
    // the value goes out of scope. We accept it by value to match the C
    // semantics of taking ownership and freeing it.
    drop(_cmd);
}

/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command::default()
}
