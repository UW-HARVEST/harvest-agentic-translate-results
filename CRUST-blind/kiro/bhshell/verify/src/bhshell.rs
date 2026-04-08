use crate::input::{self, Command as ShellCommand};
use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
pub const BUF_SIZE: usize = 64;

const BUILTIN_STR: &[&str] = &["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status = 1;
    while status != 0 {
        let dir = std::env::current_dir().unwrap_or_else(|_| std::process::exit(1));
        print!("[{}] $ ", dir.display());
        std::io::stdout().flush().ok();

        let line = input::bhshell_read_line();
        let mut cmd = input::bhshell_parse(&line);
        if cmd.args.is_empty() {
            println!("Invalid Command");
            continue;
        }
        status = bhshell_execute(&mut cmd);
    }
}
/// Executes the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_execute(cmd: &mut ShellCommand) -> i32 {
    if cmd.args.is_empty() {
        return 1;
    }
    for (i, name) in BUILTIN_STR.iter().enumerate() {
        if cmd.args[0] == *name {
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
pub fn bhshell_launch(cmd: &mut ShellCommand) -> i32 {
    let has_pipe = !cmd.pipe_args.is_empty();
    let has_redirect = cmd.redirect_file_name.is_some();

    if has_pipe {
        // cmd | pipe_cmd [> file]
        let stdout_cfg = Stdio::piped();
        let child1 = Command::new(&cmd.args[0])
            .args(&cmd.args[1..])
            .stdout(stdout_cfg)
            .spawn();
        let mut child1 = match child1 {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };

        let pipe_stdin = Stdio::from(child1.stdout.take().unwrap());
        let pipe_stdout = if has_redirect { Stdio::piped() } else { Stdio::inherit() };

        let child2 = Command::new(&cmd.pipe_args[0])
            .args(&cmd.pipe_args[1..])
            .stdin(pipe_stdin)
            .stdout(pipe_stdout)
            .spawn();
        let mut child2 = match child2 {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                child1.wait().ok();
                return 1;
            }
        };

        if has_redirect {
            let mut output = Vec::new();
            if let Some(ref mut stdout) = child2.stdout {
                stdout.read_to_end(&mut output).ok();
            }
            write_output_to_file(&output, cmd.redirect_file_name.as_ref().unwrap());
        }

        child2.wait().ok();
        child1.wait().ok();
    } else if has_redirect {
        // cmd > file
        let child = Command::new(&cmd.args[0])
            .args(&cmd.args[1..])
            .stdout(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };

        let mut output = Vec::new();
        if let Some(ref mut stdout) = child.stdout {
            stdout.read_to_end(&mut output).ok();
        }
        write_output_to_file(&output, cmd.redirect_file_name.as_ref().unwrap());
        child.wait().ok();
    } else {
        // simple command
        let child = Command::new(&cmd.args[0])
            .args(&cmd.args[1..])
            .spawn();
        match child {
            Ok(mut c) => { c.wait().ok(); }
            Err(e) => { eprintln!("bhshell: {}", e); }
        }
    }
    1
}

fn write_output_to_file(data: &[u8], filename: &str) {
    let mut f = match File::create(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Could not open file");
            std::process::exit(1);
        }
    };
    if f.write_all(data).is_err() {
        eprintln!("Could not write to file");
        std::process::exit(1);
    }
}

/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else if std::env::set_current_dir(&args[1]).is_err() {
        eprintln!("bhshell: No such file or directory");
    }
    1
}
/// Prints help information.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_help(_args: &[String]) -> i32 {
    println!("A simple shell built to understand how processes work.");
    println!("The following functions are builtin:");
    for (i, name) in BUILTIN_STR.iter().enumerate() {
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
    BUILTIN_STR.len() as i32
}
/// Writes to a redirected file descriptor array.
/// In C, this took an array 'int redirect_fd[2]' and a pointer to 'command'.
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], cmd: &mut ShellCommand) {
    // In the Rust version, redirect is handled via std::process::Stdio.
    // This function is kept for API compatibility but the actual redirect
    // logic is integrated into bhshell_launch using write_output_to_file.
    if let Some(ref filename) = cmd.redirect_file_name {
        // No-op in Rust implementation; redirect handled in bhshell_launch
        let _ = filename;
    }
}
