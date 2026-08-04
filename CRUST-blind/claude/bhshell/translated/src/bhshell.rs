use crate::input::{bhshell_parse, bhshell_read_line, destroy_command, Command};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

pub const BUF_SIZE: usize = 64;

/// Returns the table of built-in command names.
fn bhshell_builtin_str() -> &'static [&'static str] {
    &["cd", "help", "exit"]
}

/// Returns the function corresponding to a built-in by index.
fn dispatch_builtin(index: usize, args: &[String]) -> i32 {
    match index {
        0 => bhshell_cd(args),
        1 => bhshell_help(args),
        2 => bhshell_exit(args),
        _ => 1,
    }
}

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status: i32 = 1;

    while status != 0 {
        let dir = match env::current_dir() {
            Ok(d) => d,
            Err(_) => std::process::exit(1),
        };
        print!("[{}] $ ", dir.display());
        // Make sure the prompt is flushed before reading input.
        let _ = std::io::stdout().flush();

        let line = bhshell_read_line();
        let mut cmd = bhshell_parse(&line);
        // An "invalid" command from the parser is represented by a Command
        // with an empty `args` vector.
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

    let first = &cmd.args[0];
    let builtins = bhshell_builtin_str();
    for (i, name) in builtins.iter().enumerate() {
        if first == *name {
            return dispatch_builtin(i, &cmd.args);
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
    let extra_args = &cmd.args[1..];

    let mut process = ProcessCommand::new(program);
    process.args(extra_args);

    if !cmd.pipe_args.is_empty() {
        // pipe stdout to the next process
        process.stdout(Stdio::piped());
    } else if cmd.redirect_file_name.is_some() {
        // redirect stdout into a pipe so we can capture output and write it
        process.stdout(Stdio::piped());
    }

    let first_child_result = process.spawn();
    let mut first_child = match first_child_result {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bhshell: {}", e);
            return 1;
        }
    };

    if !cmd.pipe_args.is_empty() {
        let pipe_program = &cmd.pipe_args[0];
        let pipe_extra = &cmd.pipe_args[1..];

        let mut second = ProcessCommand::new(pipe_program);
        second.args(pipe_extra);

        // Hook up stdin from the first child's stdout.
        if let Some(stdout) = first_child.stdout.take() {
            second.stdin(Stdio::from(stdout));
        }

        if cmd.redirect_file_name.is_some() {
            second.stdout(Stdio::piped());
        }

        let second_child_result = second.spawn();
        let mut second_child = match second_child_result {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                let _ = first_child.wait();
                return 1;
            }
        };

        if cmd.redirect_file_name.is_some() {
            // Take the second child's stdout and write it to the file.
            let mut redirect_fd = [0i32; 2];
            // Use a helper that consumes the stdout pipe directly.
            write_to_redirect_from_child(&mut second_child, cmd);
            // Avoid unused-variable warnings on `redirect_fd`.
            let _ = &mut redirect_fd;
        }

        let _ = second_child.wait();
        let _ = first_child.wait();
        return 1;
    }

    if cmd.redirect_file_name.is_some() {
        let mut redirect_fd = [0i32; 2];
        write_to_redirect_from_child(&mut first_child, cmd);
        let _ = &mut redirect_fd;
    }

    let _ = first_child.wait();
    1
}

/// Helper: read all of `child`'s stdout and write it into `cmd.redirect_file_name`.
fn write_to_redirect_from_child(child: &mut std::process::Child, cmd: &mut Command) {
    use std::io::Read;

    let mut buf = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut buf);
    }

    if let Some(ref path) = cmd.redirect_file_name {
        match File::create(Path::new(path)) {
            Ok(mut f) => {
                if !buf.is_empty() {
                    if f.write_all(&buf).is_err() {
                        eprintln!("Could not write to file");
                    }
                }
            }
            Err(_) => {
                eprintln!("Could not open file");
            }
        }
    }
}

/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else {
        if let Err(e) = env::set_current_dir(Path::new(&args[1])) {
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
///
/// This function is preserved for API parity with the C version. The Rust
/// implementation of `bhshell_launch` uses higher-level pipe-handle helpers
/// (`write_to_redirect_from_child`) instead of raw file descriptors.
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], cmd: &mut Command) {
    // With no real file descriptor we simply ensure the redirect target
    // file exists / is created (and otherwise empty), matching the
    // semantics of "open and write the redirected content".
    if let Some(ref path) = cmd.redirect_file_name {
        let _ = File::create(Path::new(path));
    }
}
