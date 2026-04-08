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
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap_or(0);
    if line.ends_with('\n') {
        line.pop();
    }
    line
}
/// Parses a given line into a Command structure.
/// In C, this was returning a 'command*' for the allocated command.
pub fn bhshell_parse(line: &str) -> Command {
    #[derive(PartialEq)]
    enum ArgType { Arg, PipeArg, Redirect }

    let mut args: Vec<String> = Vec::new();
    let mut pipe_args: Vec<String> = Vec::new();
    let mut redirect: Option<String> = None;
    let mut current_token = String::new();
    let mut current = ArgType::Arg;

    for ch in line.chars() {
        if ch == '\n' || ch == '\t' || ch == ' ' {
            if !current_token.is_empty() {
                let token = std::mem::take(&mut current_token);
                match current {
                    ArgType::Arg => args.push(token),
                    ArgType::PipeArg => pipe_args.push(token),
                    ArgType::Redirect => redirect = Some(token),
                }
            }
            continue;
        } else if ch == '|' {
            if !current_token.is_empty() {
                let token = std::mem::take(&mut current_token);
                if current == ArgType::Arg {
                    args.push(token);
                } else {
                    // invalid: pipe after pipe or redirect
                    return Command::default();
                }
            }
            current = ArgType::PipeArg;
        } else if ch == '>' {
            if !current_token.is_empty() {
                let token = std::mem::take(&mut current_token);
                match current {
                    ArgType::Arg => args.push(token),
                    ArgType::PipeArg => pipe_args.push(token),
                    ArgType::Redirect => {}
                }
            }
            current = ArgType::Redirect;
        } else {
            current_token.push(ch);
        }
    }

    // End of line: if no token accumulated, it's invalid
    if current_token.is_empty() {
        return Command::default();
    }

    let token = current_token;
    match current {
        ArgType::Arg => args.push(token),
        ArgType::PipeArg => pipe_args.push(token),
        ArgType::Redirect => redirect = Some(token),
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
    // Rust drops automatically
}
/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command::default()
}
