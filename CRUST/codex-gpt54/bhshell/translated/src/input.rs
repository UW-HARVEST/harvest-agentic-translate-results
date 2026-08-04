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
    if std::io::stdin().read_line(&mut line).is_err() {
        return String::new();
    }

    while matches!(line.chars().last(), Some('\n' | '\r')) {
        line.pop();
    }

    line
}
/// Parses a given line into a Command structure.
/// In C, this was returning a 'command*' for the allocated command.
pub fn bhshell_parse(line: &str) -> Command {
    enum ArgType {
        Arg,
        PipeArg,
        Redirect,
    }

    let mut args: Vec<String> = Vec::new();
    let mut pipe_args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut redirect_file_name: Option<String> = None;
    let mut target = ArgType::Arg;

    let finish_current = |token: &mut String,
                          target: &ArgType,
                          args: &mut Vec<String>,
                          pipe_args: &mut Vec<String>,
                          redirect: &mut Option<String>| {
        if token.is_empty() {
            return;
        }

        let value = std::mem::take(token);
        match target {
            ArgType::Arg => args.push(value),
            ArgType::PipeArg => pipe_args.push(value),
            ArgType::Redirect => *redirect = Some(value),
        }
    };

    for ch in line.chars() {
        match ch {
            '\n' | '\t' | ' ' => {
                finish_current(
                    &mut current,
                    &target,
                    &mut args,
                    &mut pipe_args,
                    &mut redirect_file_name,
                );
            }
            '|' => {
                if !current.is_empty() {
                    if matches!(target, ArgType::Arg) {
                        args.push(std::mem::take(&mut current));
                    } else {
                        return Command::default();
                    }
                }
                target = ArgType::PipeArg;
            }
            '>' => {
                finish_current(
                    &mut current,
                    &target,
                    &mut args,
                    &mut pipe_args,
                    &mut redirect_file_name,
                );
                target = ArgType::Redirect;
            }
            _ => current.push(ch),
        }
    }

    if current.is_empty() {
        return Command::default();
    }

    finish_current(
        &mut current,
        &target,
        &mut args,
        &mut pipe_args,
        &mut redirect_file_name,
    );

    if args.is_empty() {
        return Command::default();
    }

    Command {
        args,
        pipe_args,
        redirect_file_name,
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
