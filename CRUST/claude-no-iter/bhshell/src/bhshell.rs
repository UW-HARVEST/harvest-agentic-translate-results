use crate::input::{bhshell_parse, bhshell_read_line, destroy_command, Command};
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::process::{Command as ProcCommand, Stdio};

pub const BUF_SIZE: usize = 64;

const BUILTIN_NAMES: &[&str] = &["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status: i32 = 1;

    while status != 0 {
        let dir = match env::current_dir() {
            Ok(p) => p.display().to_string(),
            Err(_) => std::process::exit(1),
        };
        print!("[{}] $ ", dir);
        let _ = io::stdout().flush();

        let line = bhshell_read_line();
        let mut cmd = bhshell_parse(&line);
        if cmd.args.is_empty() && cmd.pipe_args.is_empty() && cmd.redirect_file_name.is_none() {
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
    for (i, name) in BUILTIN_NAMES.iter().enumerate() {
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
    if cmd.args.is_empty() {
        return 1;
    }

    let prog = cmd.args[0].clone();
    let args = cmd.args[1..].to_vec();

    // Build the primary process.
    let mut first = ProcCommand::new(&prog);
    first.args(&args);

    // If we have pipe_args, we'll pipe stdout to a second process.
    if !cmd.pipe_args.is_empty() {
        let pipe_prog = cmd.pipe_args[0].clone();
        let pipe_extra = cmd.pipe_args[1..].to_vec();

        first.stdout(Stdio::piped());

        let mut first_child = match first.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };

        let mut second = ProcCommand::new(&pipe_prog);
        second.args(&pipe_extra);
        if let Some(out) = first_child.stdout.take() {
            second.stdin(Stdio::from(out));
        }

        // Redirect output of the second command if requested.
        if let Some(ref fname) = cmd.redirect_file_name {
            match File::create(fname) {
                Ok(f) => {
                    second.stdout(Stdio::from(f));
                }
                Err(_) => {
                    eprintln!("Could not open file");
                    let _ = first_child.wait();
                    return 1;
                }
            }
        }

        let mut second_child = match second.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                let _ = first_child.wait();
                return 1;
            }
        };

        let _ = first_child.wait();
        let _ = second_child.wait();
        return 1;
    }

    // No pipe; possibly redirect stdout to a file.
    if let Some(ref fname) = cmd.redirect_file_name {
        match File::create(fname) {
            Ok(f) => {
                first.stdout(Stdio::from(f));
            }
            Err(_) => {
                eprintln!("Could not open file");
                return 1;
            }
        }
    }

    match first.spawn() {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => {
            eprintln!("bhshell: {}", e);
        }
    }
    1
}

/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else if let Err(e) = env::set_current_dir(&args[1]) {
        eprintln!("bhshell: {}", e);
    }
    1
}

/// Prints help information.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_help(_args: &[String]) -> i32 {
    println!("A simple shell built to understand how processes work.");
    println!("The following functions are builtin:");
    let count = bhshell_num_builtins();
    for i in 0..count {
        println!("\t {}. {}", i + 1, BUILTIN_NAMES[i as usize]);
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
    BUILTIN_NAMES.len() as i32
}

/// Writes to a redirected file descriptor array.
/// In C, this took an array 'int redirect_fd[2]' and a pointer to 'command'.
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], cmd: &mut Command) {
    // In the safe-Rust port, redirection is handled by setting up the child's
    // stdout to a file directly in `bhshell_launch`. This function is provided
    // for API parity. If a redirect file name is set, ensure the file exists.
    if let Some(ref fname) = cmd.redirect_file_name {
        if let Ok(mut f) = File::create(fname) {
            // Nothing to write here in this safe form; preserve file creation.
            let _ = f.flush();
        } else {
            eprintln!("Could not open file");
        }
    }
}
