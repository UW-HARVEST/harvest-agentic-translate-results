use crate::input::Command;
use std::fs::File;
use std::io::Write;
use std::process;
pub const BUF_SIZE: usize = 64;

const BHSHELL_BUILTIN_STR: &[&str] = &["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status = 1;
    while status != 0 {
        let dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| process::exit(1));
        print!("[{}] $ ", dir);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let line = crate::input::bhshell_read_line();
        let mut cmd = crate::input::bhshell_parse(&line);
        if cmd.args.is_empty() {
            println!("Invalid Command");
            continue;
        }
        status = bhshell_execute(&mut cmd);
    }
}
/// Executes the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_execute(cmd: &mut Command) -> i32 {
    if cmd.args.is_empty() || cmd.args[0].is_empty() {
        return 1;
    }
    let builtins: &[fn(&[String]) -> i32] = &[bhshell_cd, bhshell_help, bhshell_exit];
    for i in 0..bhshell_num_builtins() as usize {
        if cmd.args[0] == BHSHELL_BUILTIN_STR[i] {
            return builtins[i](&cmd.args);
        }
    }
    bhshell_launch(cmd)
}
/// Launches the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_launch(cmd: &mut Command) -> i32 {
    use std::process::{Command as ProcCmd, Stdio};

    let has_pipe = !cmd.pipe_args.is_empty();
    let has_redirect = cmd.redirect_file_name.is_some();

    if has_pipe {
        // cmd.args | cmd.pipe_args [> file]
        let mut child1 = ProcCmd::new(&cmd.args[0])
            .args(&cmd.args[1..])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| {
                eprintln!("bhshell: {}", e);
                process::exit(1);
            });

        let pipe_stdout = child1.stdout.take().unwrap();

        let mut child2_cmd = ProcCmd::new(&cmd.pipe_args[0]);
        child2_cmd.args(&cmd.pipe_args[1..]);
        child2_cmd.stdin(pipe_stdout);

        if has_redirect {
            child2_cmd.stdout(Stdio::piped());
        }

        let mut child2 = child2_cmd.spawn().unwrap_or_else(|e| {
            eprintln!("bhshell: {}", e);
            process::exit(1);
        });

        if has_redirect {
            let output = child2.wait_with_output().unwrap_or_else(|e| {
                eprintln!("bhshell: {}", e);
                process::exit(1);
            });
            child1.wait().ok();
            write_output_to_file(cmd.redirect_file_name.as_ref().unwrap(), &output.stdout);
        } else {
            child2.wait().ok();
            child1.wait().ok();
        }
    } else if has_redirect {
        // cmd.args > file
        let output = ProcCmd::new(&cmd.args[0])
            .args(&cmd.args[1..])
            .output()
            .unwrap_or_else(|e| {
                eprintln!("bhshell: {}", e);
                process::exit(1);
            });
        write_output_to_file(cmd.redirect_file_name.as_ref().unwrap(), &output.stdout);
    } else {
        // simple command
        let mut child = ProcCmd::new(&cmd.args[0])
            .args(&cmd.args[1..])
            .spawn()
            .unwrap_or_else(|e| {
                eprintln!("bhshell: {}", e);
                process::exit(1);
            });
        child.wait().ok();
    }
    1
}

fn write_output_to_file(filename: &str, data: &[u8]) {
    let mut f = File::create(filename).unwrap_or_else(|_| {
        eprintln!("Could not open file");
        process::exit(1);
    });
    if f.write_all(data).is_err() {
        eprintln!("Could not write to file");
        process::exit(1);
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
    let count = bhshell_num_builtins();
    for i in 0..count as usize {
        println!("\t {}. {}", i + 1, BHSHELL_BUILTIN_STR[i]);
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
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], cmd: &mut Command) {
    // In the Rust version, we use std::process pipes instead of raw fd manipulation.
    // This function is kept for API compatibility but the logic is handled in bhshell_launch.
    if let Some(ref filename) = cmd.redirect_file_name {
        write_output_to_file(filename, &[]);
    }
}
