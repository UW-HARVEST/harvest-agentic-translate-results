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
    let mut current_word = String::new();
    let mut current = ArgType::Arg;
    let mut cmd = new_command();

    for ch in line.chars() {
        if ch == '\n' || ch == '\t' || ch == ' ' {
            if !current_word.is_empty() {
                let word = std::mem::take(&mut current_word);
                match current {
                    ArgType::Arg => args.push(word),
                    ArgType::PipeArg => pipe_args.push(word),
                    ArgType::Redirect => redirect = Some(word),
                }
            }
            continue;
        } else if ch == '|' {
            if !current_word.is_empty() {
                let word = std::mem::take(&mut current_word);
                if current == ArgType::Arg {
                    args.push(word);
                } else {
                    return Command::default();
                }
            }
            current = ArgType::PipeArg;
        } else if ch == '>' {
            if !current_word.is_empty() {
                let word = std::mem::take(&mut current_word);
                match current {
                    ArgType::Arg => args.push(word),
                    ArgType::PipeArg => pipe_args.push(word),
                    ArgType::Redirect => {}
                }
            }
            current = ArgType::Redirect;
        } else {
            current_word.push(ch);
        }
    }

    // End of line: if no word accumulated, invalid
    if current_word.is_empty() {
        return Command::default();
    }

    let word = current_word;
    match current {
        ArgType::Arg => args.push(word),
        ArgType::PipeArg => pipe_args.push(word),
        ArgType::Redirect => redirect = Some(word),
    }

    if args.is_empty() {
        return Command::default();
    }

    cmd.args = args;
    cmd.pipe_args = pipe_args;
    cmd.redirect_file_name = redirect;
    cmd
}
/// Cleans up and destroys a Command structure.
/// In C, this took 'command* cmd' and freed its data.
pub fn destroy_command(_cmd: Command) {
    // Rust automatically drops the Command and its owned data.
}
/// Creates and returns a new Command structure.
/// In C, this was returning a 'command*'.
pub fn new_command() -> Command {
    Command::default()
}
