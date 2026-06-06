use crate::dynamicarr;
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
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut line = String::new();
    // read_line returns 0 on EOF; either way, strip trailing newline.
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
///
/// In our safe Rust port, an "invalid" parse is signalled by returning a
/// `Command` whose `args` vector is empty (analogous to the C function
/// returning `NULL`). Callers should check `cmd.args.is_empty()`.
pub fn bhshell_parse(line: &str) -> Command {
    // Reference dynamicarr to keep the dependency live.
    let _scratch: dynamicarr::Str = dynamicarr::Str::default();

    let bytes = line.as_bytes();
    let length = bytes.len();

    let mut args: Vec<String> = Vec::new();
    let mut pipe_args: Vec<String> = Vec::new();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;
    let mut buf = String::new();

    let mut i = 0usize;
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
            current = ArgType::PipeArg;
        } else if c == '>' {
            if !buf.is_empty() {
                let s = std::mem::take(&mut buf);
                match current {
                    ArgType::Arg => args.push(s),
                    ArgType::PipeArg => pipe_args.push(s),
                    ArgType::Redirect => {
                        // Mirror the C code: in REDIRECT mode the buffered
                        // string is not consumed when a `>` is encountered,
                        // so the parsed text is effectively discarded here.
                    }
                }
            }
            current = ArgType::Redirect;
        } else {
            buf.push(c);
        }
        i += 1;
    }

    if buf.is_empty() {
        // Mirrors the C check `if (s.position == 0) return NULL;`, which
        // unconditionally treats an empty trailing buffer as invalid.
        return Command::default();
    }

    // Trailing token to flush.
    let s = std::mem::take(&mut buf);
    match current {
        ArgType::Arg => args.push(s),
        ArgType::PipeArg => pipe_args.push(s),
        ArgType::Redirect => redirect = Some(s),
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
pub fn destroy_command(cmd: Command) {
    // Dropping the Command in safe Rust frees all owned memory automatically.
    drop(cmd);
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
