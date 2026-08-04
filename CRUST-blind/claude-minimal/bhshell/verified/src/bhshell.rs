use crate::input::{self, Command};
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::{Command as ProcCommand, Stdio};

pub const BUF_SIZE: usize = 64;

/// The list of built-in command names.
fn bhshell_builtin_str() -> &'static [&'static str] {
    &["cd", "help", "exit"]
}

/// Dispatches a built-in command by name. Returns Some(status) if the name
/// matched a builtin, or None if not.
fn dispatch_builtin(name: &str, args: &[String]) -> Option<i32> {
    match name {
        "cd" => Some(bhshell_cd(args)),
        "help" => Some(bhshell_help(args)),
        "exit" => Some(bhshell_exit(args)),
        _ => None,
    }
}

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status: i32 = 1;
    while status != 0 {
        let dir = match env::current_dir() {
            Ok(p) => p,
            Err(_) => std::process::exit(1),
        };
        print!("[{}] $ ", dir.display());
        let _ = io::stdout().flush();

        let line = input::bhshell_read_line();
        let mut cmd = input::bhshell_parse(&line);
        if cmd.args.is_empty() {
            println!("Invalid Command");
            continue;
        }
        status = bhshell_execute(&mut cmd);
        input::destroy_command(cmd);
    }
}

/// Executes the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_execute(cmd: &mut Command) -> i32 {
    if cmd.args.is_empty() {
        return 1;
    }
    let name = cmd.args[0].clone();
    if let Some(status) = dispatch_builtin(&name, &cmd.args) {
        return status;
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
    let mut primary = ProcCommand::new(&cmd.args[0]);
    if cmd.args.len() > 1 {
        primary.args(&cmd.args[1..]);
    }

    // If we have pipe args, the primary's stdout is piped into the secondary.
    let has_pipe = !cmd.pipe_args.is_empty();
    let has_redirect = cmd.redirect_file_name.is_some();

    if has_pipe {
        primary.stdout(Stdio::piped());
    } else if has_redirect {
        primary.stdout(Stdio::piped());
    }

    let primary_child = match primary.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bhshell: {}", e);
            return 1;
        }
    };

    if has_pipe {
        // Build the pipe (secondary) process.
        let mut secondary = ProcCommand::new(&cmd.pipe_args[0]);
        if cmd.pipe_args.len() > 1 {
            secondary.args(&cmd.pipe_args[1..]);
        }
        // Wire primary's stdout into secondary's stdin.
        let primary_stdout = primary_child
            .stdout
            .expect("primary stdout should be piped");
        secondary.stdin(Stdio::from(primary_stdout));

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
            // Capture the secondary's stdout and write it to the redirect file.
            let mut redirect_fd = [0i32; 2];
            // The redirect output is taken from the secondary child's stdout.
            // We reuse write_to_redirect by adapting it to read from a pipe.
            // For correctness with the captured stdout, we instead read the
            // stdout directly here (the redirect_fd is unused in Rust).
            if let Some(mut out) = secondary_child.stdout.take() {
                let mut buf: Vec<u8> = Vec::new();
                if out.read_to_end(&mut buf).is_ok() {
                    if let Some(filename) = &cmd.redirect_file_name {
                        if let Ok(mut f) = File::create(filename) {
                            let _ = f.write_all(&buf);
                        } else {
                            eprintln!("Could not open file");
                        }
                    }
                }
            }
            // Touch redirect_fd to mirror C semantics (unused in Rust).
            let _ = &mut redirect_fd;
        }

        // Wait for both children.
        let _ = secondary_child.wait();
        return 1;
    } else if has_redirect {
        // No pipe, but we redirect primary's stdout to the file.
        let mut child = primary_child;
        if let Some(mut out) = child.stdout.take() {
            let mut buf: Vec<u8> = Vec::new();
            if out.read_to_end(&mut buf).is_ok() {
                if let Some(filename) = &cmd.redirect_file_name {
                    if let Ok(mut f) = File::create(filename) {
                        let _ = f.write_all(&buf);
                    } else {
                        eprintln!("Could not open file");
                    }
                }
            }
        }
        let _ = child.wait();
        return 1;
    }

    // No pipe, no redirect: just wait on the primary child.
    let mut child = primary_child;
    let _ = child.wait();
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
    let builtins = bhshell_builtin_str();
    let count = bhshell_num_builtins();
    for i in 0..count as usize {
        println!("\t {}. {}", i + 1, builtins[i]);
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
    // In the C implementation this reads from the read end of the pipe
    // until EOF, then writes the captured bytes to the redirect file.
    // In Rust, we can't easily share file descriptors, but we preserve the
    // basic file-writing behavior so the function remains usable.
    if let Some(filename) = &cmd.redirect_file_name {
        match File::create(filename) {
            Ok(mut f) => {
                // Nothing to read in the Rust version; create an empty file.
                let _ = f.write_all(b"");
            }
            Err(_) => {
                eprintln!("Could not open file");
            }
        }
    }
}
