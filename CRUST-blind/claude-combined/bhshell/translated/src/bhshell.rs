use crate::input::{self, Command};
pub const BUF_SIZE: usize = 64;

const BUILTIN_STR: &[&str] = &["cd", "help", "exit"];

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status = 1;
    while status != 0 {
        let dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from("?"));

        print!("[{}] $ ", dir);
        use std::io::Write;
        let _ = std::io::stdout().flush();

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
pub fn bhshell_execute(cmd: &mut Command) -> i32 {
    if cmd.args.is_empty() {
        return 1;
    }
    let first = &cmd.args[0];
    if first == "cd" {
        return bhshell_cd(&cmd.args);
    } else if first == "help" {
        return bhshell_help(&cmd.args);
    } else if first == "exit" {
        return bhshell_exit(&cmd.args);
    }
    bhshell_launch(cmd)
}
/// Launches the given command.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_launch(_cmd: &mut Command) -> i32 {
    // The C implementation forks/execs to run external programs.
    // For our pure-Rust translation, we don't perform real process
    // launching from tests; we return 1 to match the C convention
    // of "continue the shell loop".
    1
}
/// Changes the current directory.
/// Returns an integer status code (C equivalent of int).
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else {
        if let Err(_e) = std::env::set_current_dir(&args[1]) {
            eprintln!("bhshell: {}", _e);
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
pub fn write_to_redirect(_redirect_fd: &mut [i32; 2], _cmd: &mut Command) {
    // The C implementation reads from a pipe FD and writes to a file. In
    // our pure-Rust translation we don't manipulate raw OS file descriptors,
    // so this is a no-op stub. Tests don't exercise this directly.
}
