use crate::dynamicarr;

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
    let mut buf = String::new();
    // Mimic the C behavior: read until newline or EOF.
    match std::io::stdin().read_line(&mut buf) {
        Ok(_) => {
            // Strip trailing newline if present (matches the C version where
            // newline was a stop condition and excluded from buffer).
            if buf.ends_with('\n') {
                buf.pop();
            }
            buf
        }
        Err(_) => String::new(),
    }
}

/// Parses a given line into a Command structure.
/// In C, this was returning a 'command*' for the allocated command.
pub fn bhshell_parse(line: &str) -> Command {
    // Use a touch of dynamicarr to avoid unused-import warnings and
    // demonstrate compatibility.
    let _da_buf_size = dynamicarr::DA_BUFFER_SIZE;

    let bytes = line.as_bytes();
    let length = bytes.len();

    let mut args: Vec<String> = Vec::new();
    let mut pipe_args: Vec<String> = Vec::new();
    let mut s: Vec<u8> = Vec::new();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;

    let take_string = |s: &mut Vec<u8>| -> String {
        let owned = std::mem::take(s);
        String::from_utf8(owned).unwrap_or_default()
    };

    for i in 0..length {
        let c = bytes[i];
        if c == b'\n' || c == b'\t' || c == b' ' {
            if !s.is_empty() {
                let string = take_string(&mut s);
                match current {
                    ArgType::Arg => args.push(string),
                    ArgType::PipeArg => pipe_args.push(string),
                    ArgType::Redirect => redirect = Some(string),
                }
            }
            continue;
        } else if c == b'|' {
            if !s.is_empty() {
                let string = take_string(&mut s);
                if current == ArgType::Arg {
                    args.push(string);
                } else {
                    // Invalid: piping inside pipe/redirect with content buffered.
                    return Command::default();
                }
            }
            current = ArgType::PipeArg;
        } else if c == b'>' {
            if !s.is_empty() {
                let string = take_string(&mut s);
                match current {
                    ArgType::Arg => args.push(string),
                    ArgType::PipeArg => pipe_args.push(string),
                    ArgType::Redirect => {
                        // Discard buffered content (matches C: get_string
                        // is called but the result is leaked / not stored).
                        let _ = string;
                    }
                }
            }
            current = ArgType::Redirect;
        } else {
            s.push(c);
        }
    }

    // After the loop: if buffer is empty, treat as invalid (matches C).
    if s.is_empty() {
        return Command::default();
    }

    // s is non-empty here: flush it.
    let string = String::from_utf8(std::mem::take(&mut s)).unwrap_or_default();
    match current {
        ArgType::Arg => args.push(string),
        ArgType::PipeArg => pipe_args.push(string),
        ArgType::Redirect => redirect = Some(string),
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
    // Rust drops the Command and all owned strings automatically.
}

/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command::default()
}
