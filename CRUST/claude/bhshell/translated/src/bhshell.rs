use crate::input::{bhshell_parse, bhshell_read_line, destroy_command, Command};
use std::io::{self, Write};

pub const BUF_SIZE: usize = 64;

const BUILTINS: &[&str] = &["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status: i32 = 1;
    while status != 0 {
        let dir = match std::env::current_dir() {
            Ok(d) => d.display().to_string(),
            Err(_) => std::process::exit(1),
        };
        print!("[{}] $ ", dir);
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
    let first = cmd.args[0].clone();
    for (i, name) in BUILTINS.iter().enumerate() {
        if first == *name {
            return match i {
                0 => bhshell_cd(&cmd.args),
                1 => bhshell_help(&cmd.args),
                2 => bhshell_exit(&cmd.args),
                _ => 1,
            };
        }
    }
    bhshell_launch(cmd)
}

/// Launches the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_launch(cmd: &mut Command) -> i32 {
    use std::process::{Command as ProcCommand, Stdio};

    if cmd.args.is_empty() {
        return 1;
    }

    let program = cmd.args[0].clone();
    let prog_args: Vec<String> = cmd.args.iter().skip(1).cloned().collect();

    if !cmd.pipe_args.is_empty() {
        // Pipe scenario: run program | pipe_program, optionally redirect.
        let pipe_program = cmd.pipe_args[0].clone();
        let pipe_prog_args: Vec<String> = cmd.pipe_args.iter().skip(1).cloned().collect();

        let mut first_cmd = ProcCommand::new(&program);
        first_cmd.args(&prog_args);
        first_cmd.stdout(Stdio::piped());
        let first_child = match first_cmd.spawn() {
            Ok(c) => c,
            Err(_) => {
                eprintln!("bhshell: Could not start command");
                return 1;
            }
        };

        let stdin_for_pipe = first_child.stdout;
        let mut second_cmd = ProcCommand::new(&pipe_program);
        second_cmd.args(&pipe_prog_args);
        if let Some(out) = stdin_for_pipe {
            second_cmd.stdin(Stdio::from(out));
        }
        if cmd.redirect_file_name.is_some() {
            second_cmd.stdout(Stdio::piped());
        }

        let mut second_child = match second_cmd.spawn() {
            Ok(c) => c,
            Err(_) => {
                eprintln!("bhshell: Could not start piped command");
                return 1;
            }
        };

        if let Some(ref file_name) = cmd.redirect_file_name {
            if let Some(out) = second_child.stdout.take() {
                use std::io::Read;
                let mut output = Vec::new();
                let mut reader = out;
                let _ = reader.read_to_end(&mut output);
                if let Ok(mut f) = std::fs::File::create(file_name) {
                    let _ = f.write_all(&output);
                }
            }
        }

        let _ = second_child.wait();
        return 1;
    }

    // No pipe
    let mut child_cmd = ProcCommand::new(&program);
    child_cmd.args(&prog_args);

    if cmd.redirect_file_name.is_some() {
        child_cmd.stdout(Stdio::piped());
    }

    let mut child = match child_cmd.spawn() {
        Ok(c) => c,
        Err(_) => {
            eprintln!("bhshell: Could not start command");
            return 1;
        }
    };

    if let Some(ref file_name) = cmd.redirect_file_name {
        if let Some(out) = child.stdout.take() {
            use std::io::Read;
            let mut output = Vec::new();
            let mut reader = out;
            let _ = reader.read_to_end(&mut output);
            if let Ok(mut f) = std::fs::File::create(file_name) {
                let _ = f.write_all(&output);
            }
        }
    }

    let _ = child.wait();
    1
}

/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else if std::env::set_current_dir(&args[1]).is_err() {
        eprintln!("bhshell: chdir failed");
    }
    1
}

/// Prints help information.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_help(_args: &[String]) -> i32 {
    println!("A simple shell built to understand how processes work.");
    println!("The following functions are builtin:");
    let count = bhshell_num_builtins();
    for i in 0..count as usize {
        println!("\t {}. {}", i + 1, BUILTINS[i]);
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
    // In safe Rust we cannot manipulate raw fds; this implementation is
    // a no-op when no redirect file is set, and otherwise creates an
    // empty file at the redirect path so the function has observable
    // behavior consistent with "open the file for writing".
    if let Some(ref file_name) = cmd.redirect_file_name {
        let _ = std::fs::File::create(file_name);
    }
}
