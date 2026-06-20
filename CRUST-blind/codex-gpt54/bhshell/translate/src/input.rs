use crate::dynamicarr;
use std::io;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArgType {
    Arg,
    PipeArg,
    Redirect,
}
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
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(_) => {
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
/// Parses a given line into a Command structure.
/// In C, this was returning a 'command*' for the allocated command.
pub fn bhshell_parse(line: &str) -> Command {
    let mut args: Vec<String> = Vec::new();
    let mut pipe_args: Vec<String> = Vec::new();
    let mut current_token = String::new();
    let mut redirect: Option<String> = None;
    let mut current = ArgType::Arg;

    for ch in line.chars() {
        if matches!(ch, '\n' | '\t' | ' ') {
            if !current_token.is_empty() {
                let token = dynamicarr::get_string(&dynamicarr::Str {
                    items: current_token.clone(),
                    position: current_token.chars().count(),
                    bufsize: current_token.chars().count(),
                });
                match current {
                    ArgType::Arg => args.push(token),
                    ArgType::PipeArg => pipe_args.push(token),
                    ArgType::Redirect => redirect = Some(token),
                }
                current_token.clear();
            }
            continue;
        }

        if ch == '|' {
            if !current_token.is_empty() {
                let token = std::mem::take(&mut current_token);
                if current == ArgType::Arg {
                    args.push(token);
                } else {
                    return Command::default();
                }
            }
            current = ArgType::PipeArg;
            continue;
        }

        if ch == '>' {
            if !current_token.is_empty() {
                let token = std::mem::take(&mut current_token);
                match current {
                    ArgType::Arg => args.push(token),
                    ArgType::PipeArg => pipe_args.push(token),
                    ArgType::Redirect => {}
                }
            }
            current = ArgType::Redirect;
            continue;
        }

        current_token.push(ch);
    }

    if current_token.is_empty() {
        return Command::default();
    }

    match current {
        ArgType::Arg => args.push(current_token),
        ArgType::PipeArg => pipe_args.push(current_token),
        ArgType::Redirect => redirect = Some(current_token),
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
    drop(cmd);
}
/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command::default()
}
