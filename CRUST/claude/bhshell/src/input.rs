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
    let _ = stdin.lock().read_line(&mut line);
    if line.ends_with('\n') {
        line.pop();
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
    let mut s = dynamicarr::Str::default();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;
    let mut cmd = new_command();

    let flush_token = |s: &mut dynamicarr::Str| -> Option<String> {
        if s.position > 0 {
            let result = dynamicarr::get_string(s);
            // Reset s like C's get_string does (which freed the items
            // and reset position/bufsize).
            s.items.clear();
            s.position = 0;
            s.bufsize = 0;
            Some(result)
        } else {
            None
        }
    };

    let mut i = 0;
    while i < length {
        let c = bytes[i] as char;
        if c == '\n' || c == '\t' || c == ' ' {
            if let Some(string) = flush_token(&mut s) {
                match current {
                    ArgType::Arg => args.push(string),
                    ArgType::PipeArg => pipe_args.push(string),
                    ArgType::Redirect => redirect = Some(string),
                }
            }
        } else if c == '|' {
            if s.position > 0 {
                let string = flush_token(&mut s).unwrap();
                if current == ArgType::Arg {
                    args.push(string);
                } else {
                    return cmd; // empty / invalid -> empty Command
                }
            }
            // The original C check `line[i] == '>'` after seeing '|' is
            // dead code (line[i] is always '|' here), so we omit it.
            current = ArgType::PipeArg;
        } else if c == '>' {
            if let Some(string) = flush_token(&mut s) {
                match current {
                    ArgType::Arg => args.push(string),
                    ArgType::PipeArg => pipe_args.push(string),
                    ArgType::Redirect => {}
                }
            }
            current = ArgType::Redirect;
        } else {
            s.items.push(c);
            s.position += 1;
        }
        i += 1;
    }

    // After the loop, if there's nothing in the buffer at all, that's
    // invalid (matches the early `s.position == 0` check in C). But the
    // C code only returns NULL here if the buffer is empty *and* nothing
    // else got accumulated; in practice if we got tokens, we may still
    // have a non-empty args/pipe_args/redirect to honor. Replicate the
    // exact C order.
    if s.position == 0 {
        // Mirror the C behavior: this returns NULL.
        return cmd;
    }

    // s.position > 0 path
    if let Some(string) = flush_token(&mut s) {
        match current {
            ArgType::Arg => args.push(string),
            ArgType::PipeArg => pipe_args.push(string),
            ArgType::Redirect => redirect = Some(string),
        }
    }

    if args.is_empty() {
        return cmd;
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
    // In Rust, dropping the Command automatically frees its data.
}

/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command::default()
}
