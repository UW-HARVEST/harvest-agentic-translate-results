use crate::input::{self, Command};
pub const BUF_SIZE: usize = 64;

const BUILTIN_NAMES: [&str; 3] = ["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    use std::io::Write;
    let mut status: i32 = 1;
    while status != 0 {
        let dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from("?"));
        print!("[{}] $ ", dir);
        let _ = std::io::stdout().flush();

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
pub fn bhshell_execute(_cmd: &mut Command) -> i32 {
    if _cmd.args.is_empty() {
        return 1;
    }
    let first = &_cmd.args[0];
    for (i, name) in BUILTIN_NAMES.iter().enumerate() {
        if first == name {
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
    use std::process::{Command as PCommand, Stdio};

    if _cmd.args.is_empty() {
        return 1;
    }

    // Build the primary process.
    let program = _cmd.args[0].clone();
    let extra_args: Vec<String> = _cmd.args.iter().skip(1).cloned().collect();
    let mut child_builder = PCommand::new(&program);
    child_builder.args(&extra_args);

    // If we have pipe args, the primary command's stdout pipes into the
    // secondary command's stdin.
    let has_pipe = !_cmd.pipe_args.is_empty();
    let has_redirect = _cmd.redirect_file_name.is_some();

    if has_pipe {
        child_builder.stdout(Stdio::piped());
    } else if has_redirect {
        child_builder.stdout(Stdio::piped());
    }

    if has_pipe {
        // Pipelines are handled by a dedicated helper.
        return run_pipeline(_cmd);
    }

    let mut primary = match child_builder.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bhshell: {}", e);
            return 1;
        }
    };
    if has_redirect {
        // Read primary's stdout and write to file (mirrors write_to_redirect).
        let path = _cmd.redirect_file_name.clone().unwrap();
        let mut buf = Vec::<u8>::new();
        if let Some(mut out) = primary.stdout.take() {
            use std::io::Read;
            let _ = out.read_to_end(&mut buf);
        }
        let _ = primary.wait();

        match std::fs::File::create(&path) {
            Ok(mut f) => {
                use std::io::Write;
                if f.write_all(&buf).is_err() {
                    eprintln!("Could not write to file");
                }
            }
            Err(_) => {
                eprintln!("Could not open file");
            }
        }
    } else {
        let _ = primary.wait();
    }
    1
}

fn run_pipeline(cmd: &mut Command) -> i32 {
    use std::io::Read;
    use std::io::Write;
    use std::process::{Command as PCommand, Stdio};

    let program = cmd.args[0].clone();
    let extra_args: Vec<String> = cmd.args.iter().skip(1).cloned().collect();

    let pipe_program = cmd.pipe_args[0].clone();
    let pipe_extra: Vec<String> = cmd.pipe_args.iter().skip(1).cloned().collect();

    let primary = PCommand::new(&program)
        .args(&extra_args)
        .stdout(Stdio::piped())
        .spawn();
    let mut primary = match primary {
        Ok(p) => p,
        Err(e) => {
            eprintln!("bhshell: {}", e);
            return 1;
        }
    };

    let primary_stdout = match primary.stdout.take() {
        Some(s) => s,
        None => {
            eprintln!("bhshell: Could not redirect stdout");
            return 1;
        }
    };

    let has_redirect = cmd.redirect_file_name.is_some();

    let mut pipe_builder = PCommand::new(&pipe_program);
    pipe_builder
        .args(&pipe_extra)
        .stdin(Stdio::from(primary_stdout));
    if has_redirect {
        pipe_builder.stdout(Stdio::piped());
    }

    let mut secondary = match pipe_builder.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bhshell: {}", e);
            let _ = primary.wait();
            return 1;
        }
    };

    if has_redirect {
        let mut buf = Vec::<u8>::new();
        if let Some(mut out) = secondary.stdout.take() {
            let _ = out.read_to_end(&mut buf);
        }
        let _ = primary.wait();
        let _ = secondary.wait();
        let path = cmd.redirect_file_name.clone().unwrap();
        match std::fs::File::create(&path) {
            Ok(mut f) => {
                if f.write_all(&buf).is_err() {
                    eprintln!("Could not write to file");
                }
            }
            Err(_) => {
                eprintln!("Could not open file");
            }
        }
    } else {
        let _ = primary.wait();
        let _ = secondary.wait();
    }
    1
}

/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(_args: &[String]) -> i32 {
    if _args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else {
        let target = &_args[1];
        if let Err(e) = std::env::set_current_dir(target) {
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
    let count = bhshell_num_builtins();
    for i in 0..count as usize {
        println!("\t {}. {}", i + 1, BUILTIN_NAMES[i]);
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
    // The C version reads from the read end of a pipe and writes the data to
    // the redirect target file. In our pure-Rust pipeline implementation we
    // don't manage raw file descriptors, so this helper is a no-op kept for
    // API compatibility. The actual redirect logic lives in `bhshell_launch`
    // and `run_pipeline`.
    let _ = _redirect_fd;
    let _ = _cmd;
}
