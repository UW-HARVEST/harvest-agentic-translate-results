use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use std::process::{Command as ProcCommand, Stdio};

use crate::input::{bhshell_parse, bhshell_read_line, destroy_command, Command};

pub const BUF_SIZE: usize = 64;

const BUILTIN_STR: &[&str] = &["cd", "help", "exit"];

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
    for (i, name) in BUILTIN_STR.iter().enumerate() {
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

    let program = &cmd.args[0];
    let arg_slice = &cmd.args[1..];

    if !cmd.pipe_args.is_empty() {
        // Build piped command: cmd | pipe_cmd, optionally redirecting output to file.
        let pipe_program = &cmd.pipe_args[0];
        let pipe_args_slice = &cmd.pipe_args[1..];

        let mut first_cmd = ProcCommand::new(program);
        first_cmd.args(arg_slice);
        first_cmd.stdout(Stdio::piped());

        let first_child = match first_cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };

        let mut second_cmd = ProcCommand::new(pipe_program);
        second_cmd.args(pipe_args_slice);
        if let Some(stdout) = first_child.stdout {
            second_cmd.stdin(unsafe {
                Stdio::from_raw_fd(stdout_to_fd(stdout))
            });
        }

        if let Some(ref redirect) = cmd.redirect_file_name {
            match File::create(redirect) {
                Ok(f) => {
                    second_cmd.stdout(Stdio::from(f));
                }
                Err(_) => {
                    eprintln!("Could not open file");
                    return 1;
                }
            }
        }

        match second_cmd.spawn() {
            Ok(mut child) => {
                let _ = child.wait();
            }
            Err(e) => {
                eprintln!("bhshell: {}", e);
            }
        }
        return 1;
    }

    let mut command = ProcCommand::new(program);
    command.args(arg_slice);

    if let Some(ref redirect) = cmd.redirect_file_name {
        match File::create(redirect) {
            Ok(f) => {
                command.stdout(Stdio::from(f));
            }
            Err(_) => {
                eprintln!("Could not open file");
                return 1;
            }
        }
    }

    match command.spawn() {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => {
            eprintln!("bhshell: {}", e);
        }
    }
    1
}

fn stdout_to_fd(stdout: std::process::ChildStdout) -> std::os::unix::io::RawFd {
    use std::os::unix::io::IntoRawFd;
    stdout.into_raw_fd()
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
        println!("\t {}. {}", i + 1, BUILTIN_STR[i as usize]);
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
    BUILTIN_STR.len() as i32
}

/// Writes to a redirected file descriptor array.
/// In C, this took an array 'int redirect_fd[2]' and a pointer to 'command'.
pub fn write_to_redirect(redirect_fd: &mut [i32; 2], cmd: &mut Command) {
    // Close the write end (mirroring the C side closing redirect_fd[1])
    unsafe {
        libc_close(redirect_fd[1]);
    }

    // SAFETY: we are taking ownership of the read fd to read from it.
    let mut file = unsafe { File::from_raw_fd(redirect_fd[0]) };
    let mut buf = Vec::new();
    if file.read_to_end(&mut buf).is_err() {
        std::process::exit(1);
    }

    let path = match &cmd.redirect_file_name {
        Some(p) => p.clone(),
        None => return,
    };

    let mut out = match File::create(&path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Could not open file");
            std::process::exit(1);
        }
    };
    if out.write_all(&buf).is_err() {
        eprintln!("Could not write to file");
        std::process::exit(1);
    }
}

unsafe fn libc_close(fd: i32) {
    // Avoid bringing in libc as a dependency; use direct syscall via std.
    // The simplest portable way is to wrap the fd in a File and drop it.
    let _ = File::from_raw_fd(fd);
}
