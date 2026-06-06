use crate::input::{bhshell_parse, bhshell_read_line, destroy_command, Command};
use std::io::Write;

pub const BUF_SIZE: usize = 64;

const BUILTINS: [&str; 3] = ["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status = 1i32;
    while status != 0 {
        let dir = match std::env::current_dir() {
            Ok(d) => d.display().to_string(),
            Err(_) => std::process::exit(1),
        };

        print!("[{}] $ ", dir);
        let _ = std::io::stdout().flush();

        let line = bhshell_read_line();
        let mut cmd = bhshell_parse(&line);
        if cmd.args.is_empty() {
            println!("Invalid Command");
            continue;
        }

        status = bhshell_execute(&mut cmd);

        // Mirrors the C call to destroy_command(cmd) at end of loop body.
        destroy_command(cmd);
    }
}

/// Executes the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_execute(cmd: &mut Command) -> i32 {
    if cmd.args.is_empty() {
        return 1;
    }

    for (i, name) in BUILTINS.iter().enumerate() {
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
    use std::process::{Command as PCommand, Stdio};

    if cmd.args.is_empty() {
        return 1;
    }

    let program = &cmd.args[0];
    let args: &[String] = if cmd.args.len() > 1 { &cmd.args[1..] } else { &[] };

    let mut first = PCommand::new(program);
    first.args(args);

    // If we have pipe args, the first command's stdout becomes the input
    // for the piped command.
    let has_pipe = !cmd.pipe_args.is_empty();
    let has_redirect = cmd.redirect_file_name.is_some();

    if has_pipe {
        first.stdout(Stdio::piped());
    } else if has_redirect {
        first.stdout(Stdio::piped());
    }

    let first_child = match first.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bhshell: {}", e);
            return 1;
        }
    };

    if has_pipe {
        let pipe_program = &cmd.pipe_args[0];
        let pipe_args: &[String] = if cmd.pipe_args.len() > 1 {
            &cmd.pipe_args[1..]
        } else {
            &[]
        };

        let mut second = PCommand::new(pipe_program);
        second.args(pipe_args);

        // Take the first child's stdout and feed it as the second's stdin.
        let stdin_source = first_child
            .stdout
            .as_ref()
            .and_then(|_| None::<Stdio>);
        // We have to consume first_child to get its stdout; rebind below.
        let mut first_child = first_child;
        if let Some(out) = first_child.stdout.take() {
            second.stdin(Stdio::from(out));
        }

        if has_redirect {
            second.stdout(Stdio::piped());
        }
        let _ = stdin_source; // suppress unused

        let second_child = match second.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bhshell: {}", e);
                let _ = first_child.wait();
                return 1;
            }
        };

        if has_redirect {
            let mut second_child = second_child;
            let redirect_fd = [0i32, 0i32];
            // Read all of the second command's stdout, then write to the
            // redirect file via write_to_redirect().
            let captured: Vec<u8> = second_child
                .stdout
                .take()
                .map(|mut s| {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    let _ = s.read_to_end(&mut buf);
                    buf
                })
                .unwrap_or_default();
            // Stuff the captured output into the Command via a small detour:
            // we just call write_to_redirect with the buffer encoded inline.
            if let Some(name) = cmd.redirect_file_name.clone() {
                if let Ok(mut f) = std::fs::File::create(&name) {
                    let _ = f.write_all(&captured);
                }
            }
            let _ = redirect_fd;
            let _ = second_child.wait();
            let _ = first_child.wait();
        } else {
            let mut second_child = second_child;
            let _ = second_child.wait();
            let mut first_child = first_child;
            let _ = first_child.wait();
        }
    } else if has_redirect {
        let mut first_child = first_child;
        let captured: Vec<u8> = first_child
            .stdout
            .take()
            .map(|mut s| {
                use std::io::Read;
                let mut buf = Vec::new();
                let _ = s.read_to_end(&mut buf);
                buf
            })
            .unwrap_or_default();
        if let Some(name) = cmd.redirect_file_name.clone() {
            if let Ok(mut f) = std::fs::File::create(&name) {
                let _ = f.write_all(&captured);
            }
        }
        let _ = first_child.wait();
    } else {
        let mut first_child = first_child;
        let _ = first_child.wait();
    }

    1
}

/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else if let Err(e) = std::env::set_current_dir(&args[1]) {
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
        println!("\t {}. {}", i + 1, BUILTINS[i as usize]);
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
///
/// In our safe-Rust port we don't have raw file descriptors. The function
/// is preserved for API compatibility: when invoked it simply ensures the
/// redirect target exists (creating it empty if it does not), mirroring the
/// fact that the C code opens the file with `fopen(... , "w")`.
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], cmd: &mut Command) {
    if let Some(name) = &cmd.redirect_file_name {
        let _ = std::fs::File::create(name);
    }
}
