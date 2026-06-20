use crate::input::Command;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::process::{Command as ProcessCommand, Stdio};
pub const BUF_SIZE: usize = 64;
/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status = 1;

    while status != 0 {
        let dir = env::current_dir()
            .ok()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| ".".to_string());
        print!("[{}] $ ", dir);
        let _ = io::stdout().flush();

        let line = crate::input::bhshell_read_line();
        let mut cmd = crate::input::bhshell_parse(&line);
        if cmd.args.is_empty() {
            println!("Invalid Command");
            continue;
        }

        status = bhshell_execute(&mut cmd);
        crate::input::destroy_command(cmd);
    }
}
/// Executes the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_execute(cmd: &mut Command) -> i32 {
    let Some(program) = cmd.args.first().map(String::as_str) else {
        return 1;
    };

    match program {
        "cd" => bhshell_cd(&cmd.args),
        "help" => bhshell_help(&cmd.args),
        "exit" => bhshell_exit(&cmd.args),
        _ => bhshell_launch(cmd),
    }
}
/// Launches the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_launch(cmd: &mut Command) -> i32 {
    if cmd.args.is_empty() {
        return 1;
    }

    if !cmd.pipe_args.is_empty() {
        let mut first = ProcessCommand::new(&cmd.args[0]);
        first.args(&cmd.args[1..]).stdout(Stdio::piped());

        let mut first_child = match first.spawn() {
            Ok(child) => child,
            Err(err) => {
                eprintln!("bhshell: {err}");
                return 1;
            }
        };

        let Some(first_stdout) = first_child.stdout.take() else {
            let _ = first_child.wait();
            eprintln!("bhshell: Could not redirect stdout");
            return 1;
        };

        let mut second = ProcessCommand::new(&cmd.pipe_args[0]);
        second.args(&cmd.pipe_args[1..]).stdin(Stdio::from(first_stdout));

        if let Some(path) = &cmd.redirect_file_name {
            match File::create(path) {
                Ok(file) => {
                    second.stdout(Stdio::from(file));
                }
                Err(_) => {
                    let _ = first_child.wait();
                    eprintln!("Could not open file");
                    return 1;
                }
            }
        }

        let mut second_child = match second.spawn() {
            Ok(child) => child,
            Err(err) => {
                let _ = first_child.wait();
                eprintln!("bhshell: {err}");
                return 1;
            }
        };

        let _ = second_child.wait();
        let _ = first_child.wait();
        return 1;
    }

    let mut process = ProcessCommand::new(&cmd.args[0]);
    process.args(&cmd.args[1..]);

    if let Some(path) = &cmd.redirect_file_name {
        match File::create(path) {
            Ok(file) => {
                process.stdout(Stdio::from(file));
            }
            Err(_) => {
                eprintln!("Could not open file");
                return 1;
            }
        }
    }

    match process.spawn() {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(err) => eprintln!("bhshell: {err}"),
    }

    1
}
/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.get(1).is_none() {
        eprintln!("bhshell: expected argument to \"cd\" into");
        return 1;
    }

    if let Err(err) = env::set_current_dir(&args[1]) {
        eprintln!("bhshell: {err}");
    }

    1
}
/// Prints help information.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_help(_args: &[String]) -> i32 {
    println!("A simple shell built to understand how processes work.");
    println!("The following functions are builtin:");

    for (idx, builtin) in ["cd", "help", "exit"].iter().enumerate() {
        println!("\t {}. {}", idx + 1, builtin);
    }

    1
}
/// Handles exit request.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_exit(_args: &[String]) -> i32 {
    0
}
/// Returns the number of built-in commands.
pub fn bhshell_num_builtins() -> i32 {
    3
}
/// Writes to a redirected file descriptor array.
/// In C, this took an array 'int redirect_fd[2]' and a pointer to 'command'.
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], cmd: &mut Command) {
    if let Some(path) = &cmd.redirect_file_name {
        let _ = File::create(path);
    }
}
