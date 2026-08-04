use crate::input::{bhshell_parse, bhshell_read_line, destroy_command, Command};
use std::env;
use std::fs::File;
use std::io::Write;
use std::process::{Command as ProcCommand, Stdio};

pub const BUF_SIZE: usize = 64;

const BUILTIN_NAMES: &[&str] = &["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status: i32 = 1;
    while status != 0 {
        let dir = match env::current_dir() {
            Ok(d) => d.to_string_lossy().to_string(),
            Err(_) => {
                std::process::exit(1);
            }
        };
        print!("[{}] $ ", dir);
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

    let program = &cmd.args[0];
    let args: Vec<&str> = cmd.args.iter().skip(1).map(|s| s.as_str()).collect();

    let mut proc = ProcCommand::new(program);
    proc.args(&args);

    if !cmd.pipe_args.is_empty() {
        proc.stdout(Stdio::piped());
        let child_res = proc.spawn();
        let mut child = match child_res {
            Ok(c) => c,
            Err(_) => {
                eprintln!("bhshell");
                return 1;
            }
        };

        let pipe_program = &cmd.pipe_args[0];
        let pipe_args: Vec<&str> = cmd.pipe_args.iter().skip(1).map(|s| s.as_str()).collect();

        let mut pipe_proc = ProcCommand::new(pipe_program);
        pipe_proc.args(&pipe_args);
        if let Some(stdout) = child.stdout.take() {
            pipe_proc.stdin(Stdio::from(stdout));
        }

        if cmd.redirect_file_name.is_some() {
            pipe_proc.stdout(Stdio::piped());
        }

        let pipe_child_res = pipe_proc.spawn();
        let mut pipe_child = match pipe_child_res {
            Ok(c) => c,
            Err(_) => {
                eprintln!("bhshell");
                let _ = child.wait();
                return 1;
            }
        };

        if let Some(file_name) = &cmd.redirect_file_name {
            if let Some(mut stdout) = pipe_child.stdout.take() {
                let mut buf = Vec::new();
                let _ = std::io::Read::read_to_end(&mut stdout, &mut buf);
                if let Ok(mut f) = File::create(file_name) {
                    let _ = f.write_all(&buf);
                }
            }
        }
        let _ = child.wait();
        let _ = pipe_child.wait();
        return 1;
    }

    if let Some(file_name) = &cmd.redirect_file_name {
        proc.stdout(Stdio::piped());
        let child_res = proc.spawn();
        let mut child = match child_res {
            Ok(c) => c,
            Err(_) => {
                eprintln!("bhshell");
                return 1;
            }
        };
        if let Some(mut stdout) = child.stdout.take() {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut stdout, &mut buf);
            if let Ok(mut f) = File::create(file_name) {
                let _ = f.write_all(&buf);
            }
        }
        let _ = child.wait();
        return 1;
    }

    match proc.status() {
        Ok(_) => 1,
        Err(_) => {
            eprintln!("bhshell");
            1
        }
    }
}

/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else {
        if let Err(_) = env::set_current_dir(&args[1]) {
            eprintln!("bhshell");
        }
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
    // Without raw FDs in safe Rust, we can't do exactly what the C version does.
    // For interface compatibility, we treat this as a no-op that simply writes
    // an empty string to the redirect file if specified.
    if let Some(file_name) = &cmd.redirect_file_name {
        if let Ok(mut f) = File::create(file_name) {
            let _ = f.write_all(b"");
        }
    }
}
