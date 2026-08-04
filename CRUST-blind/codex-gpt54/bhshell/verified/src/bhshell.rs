use crate::input::Command;
use crate::input::{bhshell_parse, bhshell_read_line, destroy_command};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{self, Command as ProcessCommand, Stdio};

pub const BUF_SIZE: usize = 64;

const BUILTINS: [&str; 3] = ["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status = 1;

    while status != 0 {
        let dir = match std::env::current_dir() {
            Ok(dir) => dir,
            Err(_) => process::exit(1),
        };

        print!("[{}] $ ", dir.display());
        let _ = io::stdout().flush();

        let line = bhshell_read_line();
        let mut cmd = bhshell_parse(&line);
        if cmd.args.is_empty() {
            println!("Invalid Command");
            continue;
        }

        status = bhshell_execute(&mut cmd);
        destroy_command(cmd);
    }
}
/// Executes the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_execute(cmd: &mut Command) -> i32 {
    if cmd.args.is_empty() {
        return 1;
    }

    match cmd.args[0].as_str() {
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

    if cmd.pipe_args.is_empty() {
        if let Some(path) = cmd.redirect_file_name.as_deref() {
            let output = run_command_capture_stdout(&cmd.args);
            write_stdout_to_file(path, &output);
            return 1;
        }

        let mut child = match spawn_command(&cmd.args, Stdio::inherit(), Stdio::inherit()) {
            Some(child) => child,
            None => return 1,
        };
        let _ = child.wait();
        return 1;
    }

    let mut first = match spawn_command(&cmd.args, Stdio::inherit(), Stdio::piped()) {
        Some(child) => child,
        None => return 1,
    };

    let first_stdout = match first.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = first.wait();
            return 1;
        }
    };

    if let Some(path) = cmd.redirect_file_name.as_deref() {
        let second_output = run_piped_command_capture_stdout(&cmd.pipe_args, first_stdout);
        let _ = first.wait();
        write_stdout_to_file(path, &second_output);
        return 1;
    }

    let mut second = match spawn_command(&cmd.pipe_args, Stdio::from(first_stdout), Stdio::inherit()) {
        Some(child) => child,
        None => {
            let _ = first.wait();
            return 1;
        }
    };

    let _ = second.wait();
    let _ = first.wait();
    1
}
/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.get(1).is_none() {
        eprintln!("bhshell: expected argument to \"cd\" into");
        return 1;
    }

    if let Err(err) = std::env::set_current_dir(Path::new(&args[1])) {
        eprintln!("bhshell: {err}");
    }
    1
}
/// Prints help information.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_help(_args: &[String]) -> i32 {
    println!("A simple shell built to understand how processes work.");
    println!("The following functions are builtin:");

    for (idx, builtin) in BUILTINS.iter().enumerate() {
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
    BUILTINS.len() as i32
}
/// Writes to a redirected file descriptor array.
/// In C, this took an array 'int redirect_fd[2]' and a pointer to 'command'.
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], cmd: &mut Command) {
    if let Some(path) = cmd.redirect_file_name.as_deref() {
        if let Err(err) = fs::write(path, []) {
            eprintln!("Could not open file");
            eprintln!("{err}");
            process::exit(1);
        }
    }
}

fn spawn_command(
    args: &[String],
    stdin: Stdio,
    stdout: Stdio,
) -> Option<std::process::Child> {
    let mut command = ProcessCommand::new(&args[0]);
    if args.len() > 1 {
        command.args(&args[1..]);
    }
    match command.stdin(stdin).stdout(stdout).spawn() {
        Ok(child) => Some(child),
        Err(err) => {
            eprintln!("bhshell: {err}");
            None
        }
    }
}

fn run_command_capture_stdout(args: &[String]) -> Vec<u8> {
    let mut command = ProcessCommand::new(&args[0]);
    if args.len() > 1 {
        command.args(&args[1..]);
    }
    match command.stderr(Stdio::inherit()).output() {
        Ok(output) => output.stdout,
        Err(err) => {
            eprintln!("bhshell: {err}");
            Vec::new()
        }
    }
}

fn run_piped_command_capture_stdout(
    args: &[String],
    stdin: impl Into<Stdio>,
) -> Vec<u8> {
    let mut command = ProcessCommand::new(&args[0]);
    if args.len() > 1 {
        command.args(&args[1..]);
    }
    match command
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(output) => output.stdout,
        Err(err) => {
            eprintln!("bhshell: {err}");
            Vec::new()
        }
    }
}

fn write_stdout_to_file(path: &str, stdout: &[u8]) {
    if fs::write(path, stdout).is_err() {
        eprintln!("Could not open file");
        process::exit(1);
    }
}
