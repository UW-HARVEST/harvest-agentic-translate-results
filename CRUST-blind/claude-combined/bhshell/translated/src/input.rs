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
    use std::io::Read;
    let mut s = String::new();
    let mut byte = [0u8; 1];
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    loop {
        match handle.read(&mut byte) {
            Ok(0) => return s, // EOF
            Ok(_) => {
                let c = byte[0] as char;
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
/// Returns an empty Command (default) to represent the C NULL return.
pub fn bhshell_parse(line: &str) -> Command {
    let bytes = line.as_bytes();
    let length = bytes.len();
    let mut args = dynamicarr::ArgList::default();
    let mut pipe_args = dynamicarr::ArgList::default();
    let mut s = dynamicarr::Str::default();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;
    let mut cmd = new_command();

    let mut i = 0;
    while i < length {
        let c = bytes[i] as char;
        if c == '\n' || c == '\t' || c == ' ' {
            if s.position > 0 {
                let string = dynamicarr::get_string(&s);
                s = dynamicarr::Str::default();
                match current {
                    ArgType::Arg => dynamicarr::da_append_arg(&mut args, string),
                    ArgType::PipeArg => dynamicarr::da_append_arg(&mut pipe_args, string),
                    ArgType::Redirect => {
                        redirect = Some(string);
                    }
                }
            }
            i += 1;
            continue;
        } else if c == '|' {
            if s.position > 0 {
                let string = dynamicarr::get_string(&s);
                s = dynamicarr::Str::default();
                if current == ArgType::Arg {
                    dynamicarr::da_append_arg(&mut args, string);
                } else {
                    return Command::default();
                }
            }
            // Mirror the (buggy) C check: it checks line[i] which is always '|',
            // so this branch never triggers. Leaving it semantically equivalent.
            current = ArgType::PipeArg;
        } else if c == '>' {
            if s.position > 0 {
                let string = dynamicarr::get_string(&s);
                s = dynamicarr::Str::default();
                match current {
                    ArgType::Arg => dynamicarr::da_append_arg(&mut args, string),
                    ArgType::PipeArg => dynamicarr::da_append_arg(&mut pipe_args, string),
                    ArgType::Redirect => {}
                }
            }
            current = ArgType::Redirect;
        } else {
            dynamicarr::da_append_char(&mut s, c);
        }
        i += 1;
    }
    if s.position == 0 {
        return Command::default();
    }
    if s.position > 0 {
        let string = dynamicarr::get_string(&s);
        match current {
            ArgType::Arg => dynamicarr::da_append_arg(&mut args, string),
            ArgType::PipeArg => dynamicarr::da_append_arg(&mut pipe_args, string),
            ArgType::Redirect => {
                redirect = Some(string);
            }
        }
    }
    if args.position == 0 {
        return Command::default();
    }
    cmd.args = dynamicarr::get_args(&args);
    if pipe_args.position > 0 {
        cmd.pipe_args = dynamicarr::get_args(&pipe_args);
    }
    cmd.redirect_file_name = redirect;
    cmd
}
/// Cleans up and destroys a Command structure.
/// In C, this took 'command* cmd' and freed its data.
pub fn destroy_command(_cmd: Command) {
    // In Rust, dropping is automatic when cmd goes out of scope.
}
/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command::default()
}
