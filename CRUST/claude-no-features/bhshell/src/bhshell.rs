use crate::input::{self, Command};
use std::io::{self, Write};
use std::process::{Command as ProcCommand, Stdio};

pub const BUF_SIZE: usize = 64;

/// Returns the list of built-in command names.
fn bhshell_builtin_str() -> &'static [&'static str] {
    &["cd", "help", "exit"]
}

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status = 1i32;
    while status != 0 {
        let dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => std::process::exit(1),
        };
        print!("[{}] $ ", dir.display());
        let _ = io::stdout().flush();

        let line = input::bhshell_read_line();
        let mut cmd = input::bhshell_parse(&line);

        if cmd.args.is_empty() && cmd.pipe_args.is_empty() && cmd.redirect_file_name.is_none() {
            println!("Invalid Command");
            continue;
        }

        status = bhshell_execute(&mut cmd);
    }
}

/// Executes the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_execute(cmd: &mut Command) -> i32 {
    if cmd.args.is_empty() {
        return 1;
    }

    let builtins = bhshell_builtin_str();
    for (i, name) in builtins.iter().enumerate() {
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
pub fn bhshell_launch(cmd: &mut Command) -> i32 {
    if cmd.args.is_empty() {
        return 1;
    }

    let mut first = ProcCommand::new(&cmd.args[0]);
    if cmd.args.len() > 1 {
        first.args(&cmd.args[1..]);
    }

    if !cmd.pipe_args.is_empty() {
        // First command pipes its stdout to second command's stdin.
        first.stdout(Stdio::piped());
        let first_child = match first.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };

        let mut second = ProcCommand::new(&cmd.pipe_args[0]);
        if cmd.pipe_args.len() > 1 {
            second.args(&cmd.pipe_args[1..]);
        }
        if let Some(stdout) = first_child.stdout {
            second.stdin(Stdio::from(stdout));
        }

        if cmd.redirect_file_name.is_some() {
            second.stdout(Stdio::piped());
        }

        let mut second_child = match second.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };

        if let Some(_fname) = &cmd.redirect_file_name {
            // Read from second child's stdout and write to file.
            if let Some(mut stdout) = second_child.stdout.take() {
                let mut buf = Vec::new();
                let _ = io::Read::read_to_end(&mut stdout, &mut buf);
                if let Some(fname) = &cmd.redirect_file_name {
                    if let Ok(mut f) = std::fs::File::create(fname) {
                        let _ = f.write_all(&buf);
                    }
                }
            }
        }
        let _ = second_child.wait();
        return 1;
    }

    if let Some(fname) = &cmd.redirect_file_name {
        match std::fs::File::create(fname) {
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
    } else {
        if let Err(e) = std::env::set_current_dir(&args[1]) {
            eprintln!("bhshell: {}", e);
        }
    }
    1
}

/// Prints help information.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_help(_args: &[String]) -> i32 {
    println!("A simple shell built to understand how processes work.");
    println!("The following functions are builtin:");
    let builtins = bhshell_builtin_str();
    for (i, name) in builtins.iter().enumerate() {
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
    bhshell_builtin_str().len() as i32
}

/// Writes to a redirected file descriptor array.
/// In C, this took an array 'int redirect_fd[2]' and a pointer to 'command'.
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], cmd: &mut Command) {
    // In the safe Rust port, redirection is handled directly via std::process
    // Stdio::from(File). This function is kept for interface compatibility.
    if let Some(fname) = &cmd.redirect_file_name {
        let _ = std::fs::File::create(fname);
    }
}
