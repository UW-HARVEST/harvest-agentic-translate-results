use crate::input::{bhshell_parse, bhshell_read_line, destroy_command, Command};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};

pub const BUF_SIZE: usize = 64;

const BUILTIN_NAMES: &[&str] = &["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status: i32 = 1;
    while status != 0 {
        let dir = match env::current_dir() {
            Ok(d) => d,
            Err(_) => std::process::exit(1),
        };
        print!("[{}] $ ", dir.display());
        let _ = std::io::stdout().flush();

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
    for (i, name) in BUILTIN_NAMES.iter().enumerate() {
        if &first == name {
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

    // Build the primary process.
    let mut primary = StdCommand::new(&cmd.args[0]);
    if cmd.args.len() > 1 {
        primary.args(&cmd.args[1..]);
    }

    let has_pipe = !cmd.pipe_args.is_empty();
    let has_redirect = cmd.redirect_file_name.is_some();

    if has_pipe {
        primary.stdout(Stdio::piped());
    } else if has_redirect {
        primary.stdout(Stdio::piped());
    }

    let mut primary_child = match primary.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bhshell: {}", e);
            return 1;
        }
    };

    if has_pipe {
        let mut secondary = StdCommand::new(&cmd.pipe_args[0]);
        if cmd.pipe_args.len() > 1 {
            secondary.args(&cmd.pipe_args[1..]);
        }
        if let Some(out) = primary_child.stdout.take() {
            secondary.stdin(Stdio::from(out));
        }
        if has_redirect {
            secondary.stdout(Stdio::piped());
        }
        let mut secondary_child = match secondary.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };

        if has_redirect {
            let mut redirect_fd: [i32; 2] = [0, 0];
            if let Some(mut out) = secondary_child.stdout.take() {
                let mut buf = String::new();
                let _ = out.read_to_string(&mut buf);
                if let Some(name) = cmd.redirect_file_name.as_ref() {
                    if let Ok(mut f) = File::create(Path::new(name)) {
                        let _ = f.write_all(buf.as_bytes());
                    }
                }
            }
            // call write_to_redirect to keep API alive (no-op since stdout was already drained).
            write_to_redirect(&mut redirect_fd, cmd);
        }
        let _ = secondary_child.wait();
        let _ = primary_child.wait();
        return 1;
    } else if has_redirect {
        if let Some(mut out) = primary_child.stdout.take() {
            let mut buf = String::new();
            let _ = out.read_to_string(&mut buf);
            if let Some(name) = cmd.redirect_file_name.as_ref() {
                if let Ok(mut f) = File::create(Path::new(name)) {
                    let _ = f.write_all(buf.as_bytes());
                }
            }
        }
        let mut redirect_fd: [i32; 2] = [0, 0];
        write_to_redirect(&mut redirect_fd, cmd);
    }

    let _ = primary_child.wait();
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
    for (i, name) in BUILTIN_NAMES.iter().enumerate() {
        println!("\t {}. {}", i + 1, name);
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
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], _cmd: &mut Command) {
    // In the safe Rust port, redirection is handled in `bhshell_launch` using
    // `std::process::Command` piping, so this function is intentionally a
    // no-op.  It exists to preserve the original API.
}
