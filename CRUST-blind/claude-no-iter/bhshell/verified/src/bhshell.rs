use crate::input::{self, Command};
use std::io::Write;

pub const BUF_SIZE: usize = 64;

const BHSHELL_BUILTIN_STR: &[&str] = &["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status: i32 = 1;
    while status != 0 {
        let dir = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => std::process::exit(1),
        };
        print!("[{}] $ ", dir.display());
        let _ = std::io::stdout().flush();

        let line = input::bhshell_read_line();
        let mut cmd = input::bhshell_parse(&line);
        // In Rust, an "invalid" command from the parser is signalled by an
        // empty `args` vector.
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
pub fn bhshell_execute(_cmd: &mut Command) -> i32 {
    if _cmd.args.is_empty() {
        return 1;
    }
    let first = &_cmd.args[0];
    for (i, name) in BHSHELL_BUILTIN_STR.iter().enumerate() {
        if first == *name {
            return match i {
                0 => bhshell_cd(&_cmd.args),
                1 => bhshell_help(&_cmd.args),
                2 => bhshell_exit(&_cmd.args),
                _ => 1,
            };
        }
    }
    bhshell_launch(_cmd)
}

/// Launches the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_launch(_cmd: &mut Command) -> i32 {
    use std::process::{Command as ProcCommand, Stdio};

    if _cmd.args.is_empty() {
        return 1;
    }

    // Build the primary command
    let mut primary = ProcCommand::new(&_cmd.args[0]);
    primary.args(&_cmd.args[1..]);

    let has_pipe = !_cmd.pipe_args.is_empty();
    let has_redirect = _cmd.redirect_file_name.is_some();

    if has_pipe {
        // Spawn the primary with stdout piped, then feed it into the second
        // command.
        primary.stdout(Stdio::piped());

        let primary_child = match primary.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };

        let primary_stdout = match primary_child.stdout {
            Some(s) => s,
            None => {
                eprintln!("bhshell: Could not capture stdout");
                return 1;
            }
        };

        let mut secondary = ProcCommand::new(&_cmd.pipe_args[0]);
        secondary.args(&_cmd.pipe_args[1..]);
        secondary.stdin(Stdio::from(primary_stdout));

        if has_redirect {
            secondary.stdout(Stdio::piped());
        }

        let secondary_child = match secondary.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };

        if has_redirect {
            // Capture the secondary's stdout and write it to the file.
            let output = match secondary_child.wait_with_output() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("bhshell: {}", e);
                    return 1;
                }
            };
            if let Some(name) = &_cmd.redirect_file_name {
                match std::fs::File::create(name) {
                    Ok(mut f) => {
                        if f.write_all(&output.stdout).is_err() {
                            eprintln!("Could not write to file");
                        }
                    }
                    Err(_) => {
                        eprintln!("Could not open file");
                    }
                }
            }
        } else {
            // Just wait for the secondary to finish.
            let mut child = secondary_child;
            let _ = child.wait();
        }
        return 1;
    }

    if has_redirect {
        // Capture stdout and write it to the file.
        primary.stdout(Stdio::piped());
        let child = match primary.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };
        let output = match child.wait_with_output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                return 1;
            }
        };
        if let Some(name) = &_cmd.redirect_file_name {
            match std::fs::File::create(name) {
                Ok(mut f) => {
                    if f.write_all(&output.stdout).is_err() {
                        eprintln!("Could not write to file");
                    }
                }
                Err(_) => {
                    eprintln!("Could not open file");
                }
            }
        }
        return 1;
    }

    // No pipe and no redirect: just run the command, inheriting stdio.
    match primary.spawn() {
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
pub fn bhshell_cd(_args: &[String]) -> i32 {
    if _args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else {
        if std::env::set_current_dir(&_args[1]).is_err() {
            eprintln!("bhshell: could not change directory");
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
        println!("\t {}. {}", i + 1, BHSHELL_BUILTIN_STR[i as usize]);
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
    BHSHELL_BUILTIN_STR.len() as i32
}

/// Writes to a redirected file descriptor array.
/// In C, this took an array 'int redirect_fd[2]' and a pointer to 'command'.
///
/// In the safe-Rust translation we don't directly manipulate raw file
/// descriptors. The redirection logic is handled inside `bhshell_launch`
/// using `std::process::Stdio::piped` and `wait_with_output`. This function
/// is kept for API parity and is essentially a no-op.
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], _cmd: &mut Command) {
    // No-op: actual redirection is performed in `bhshell_launch`.
    let _ = _redirect_fd;
    let _ = _cmd;
}
