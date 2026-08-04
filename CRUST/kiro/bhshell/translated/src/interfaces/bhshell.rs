use crate::input::{Command, bhshell_parse, bhshell_read_line, destroy_command};
pub const BUF_SIZE: usize = 64;

use std::ffi::CString;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};

const BUILTIN_NAMES: &[&str] = &["cd", "help", "exit"];

fn raw_pipe() -> (i32, i32) {
    let (r, w) = nix::unistd::pipe().unwrap_or_else(|_| std::process::exit(1));
    let rfd = r.as_raw_fd();
    let wfd = w.as_raw_fd();
    // Leak the OwnedFds so they don't get closed on drop
    std::mem::forget(r);
    std::mem::forget(w);
    (rfd, wfd)
}

/// Runs the main bhshell loop.
pub fn bhshell_loop() {
    let mut status = 1;
    while status != 0 {
        let dir = std::env::current_dir().unwrap_or_else(|_| std::process::exit(1));
        print!("[{}] $ ", dir.display());
        std::io::stdout().flush().ok();

        let line = bhshell_read_line();
        let mut cmd = bhshell_parse(&line);
        if cmd.args.is_empty() {
            println!("Invalid Command");
            continue;
        }
        status = bhshell_execute(&mut cmd);
        destroy_command(cmd);
    }
}
/// Executes the given command.
pub fn bhshell_execute(cmd: &mut Command) -> i32 {
    if cmd.args.is_empty() || cmd.args[0].is_empty() {
        return 1;
    }
    for (i, name) in BUILTIN_NAMES.iter().enumerate() {
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
pub fn bhshell_launch(cmd: &mut Command) -> i32 {
    use nix::sys::wait::waitpid;
    use nix::unistd::{close, dup2, execvp, fork, ForkResult};

    let redirect_fd = if cmd.redirect_file_name.is_some() {
        Some(raw_pipe())
    } else {
        None
    };

    let pipe_fd = if !cmd.pipe_args.is_empty() {
        Some(raw_pipe())
    } else {
        None
    };

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            if let Some((r, w)) = pipe_fd {
                close(r).ok();
                dup2(w, 1).unwrap_or_else(|_| {
                    eprintln!("bhshell: Could not redirect stdout");
                    std::process::exit(1);
                });
                close(w).ok();
                if let Some((rr, rw)) = redirect_fd {
                    close(rr).ok();
                    close(rw).ok();
                }
            } else if let Some((r, w)) = redirect_fd {
                close(r).ok();
                dup2(w, 1).unwrap_or_else(|_| {
                    eprintln!("bhshell: Could not redirect stdout to file");
                    std::process::exit(1);
                });
                close(w).ok();
            }
            let c_args: Vec<CString> = cmd.args.iter()
                .map(|s| CString::new(s.as_str()).unwrap())
                .collect();
            let _ = execvp(&c_args[0], &c_args);
            eprintln!("bhshell: {}", std::io::Error::last_os_error());
            std::process::exit(1);
        }
        Ok(ForkResult::Parent { child }) => {
            if let Some((r, w)) = pipe_fd {
                match unsafe { fork() } {
                    Ok(ForkResult::Child) => {
                        if let Some((rr, rw)) = redirect_fd {
                            close(rr).ok();
                            dup2(rw, 1).unwrap_or_else(|_| {
                                eprintln!("bhshell: Could not redirect stdout to file");
                                std::process::exit(1);
                            });
                            close(rw).ok();
                        }
                        close(w).ok();
                        dup2(r, 0).unwrap_or_else(|_| {
                            eprintln!("bhshell: Could not redirect stdin");
                            std::process::exit(1);
                        });
                        close(r).ok();
                        let c_args: Vec<CString> = cmd.pipe_args.iter()
                            .map(|s| CString::new(s.as_str()).unwrap())
                            .collect();
                        let _ = execvp(&c_args[0], &c_args);
                        eprintln!("bhshell: {}", std::io::Error::last_os_error());
                        std::process::exit(1);
                    }
                    Ok(ForkResult::Parent { child: pipe_child }) => {
                        close(r).ok();
                        close(w).ok();
                        if let Some((rr, rw)) = redirect_fd {
                            let mut fds = [rr, rw];
                            write_to_redirect(&mut fds, cmd);
                        }
                        let _ = waitpid(pipe_child, None);
                        let _ = waitpid(child, None);
                    }
                    Err(_) => {
                        eprintln!("bhshell: Could not create child process");
                        std::process::exit(1);
                    }
                }
            } else {
                if let Some((rr, rw)) = redirect_fd {
                    let mut fds = [rr, rw];
                    write_to_redirect(&mut fds, cmd);
                }
                let _ = waitpid(child, None);
            }
            1
        }
        Err(_) => {
            eprintln!("bhshell: Could not create child process");
            std::process::exit(1);
        }
    }
}
/// Changes the current directory.
pub fn bhshell_cd(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("bhshell: expected argument to \"cd\" into");
    } else if std::env::set_current_dir(&args[1]).is_err() {
        eprintln!("bhshell: {}", std::io::Error::last_os_error());
    }
    1
}
/// Prints help information.
pub fn bhshell_help(_args: &[String]) -> i32 {
    println!("A simple shell built to understand how processes work.");
    println!("The following functions are builtin:");
    for (i, name) in BUILTIN_NAMES.iter().enumerate() {
        println!("\t {}. {}", i + 1, name);
    }
    1
}
/// Handles exit request.
pub fn bhshell_exit(_args: &[String]) -> i32 {
    0
}
/// Returns the number of built-in commands.
pub fn bhshell_num_builtins() -> i32 {
    BUILTIN_NAMES.len() as i32
}
/// Writes to a redirected file descriptor array.
pub fn write_to_redirect(redirect_fd: &mut [i32; 2], cmd: &mut Command) {
    nix::unistd::close(redirect_fd[1]).ok();

    let mut buf = Vec::new();
    let mut temp = [0u8; 1];
    let mut f = unsafe { std::fs::File::from_raw_fd(redirect_fd[0]) };
    loop {
        match f.read(&mut temp) {
            Ok(0) => break,
            Ok(_) => buf.push(temp[0]),
            Err(_) => {
                drop(f);
                std::process::exit(1);
            }
        }
    }
    drop(f); // closes redirect_fd[0]

    if let Some(ref filename) = cmd.redirect_file_name {
        let mut file = std::fs::File::create(filename).unwrap_or_else(|_| {
            eprintln!("Could not open file");
            std::process::exit(1);
        });
        file.write_all(&buf).unwrap_or_else(|_| {
            eprintln!("Could not write to file");
            std::process::exit(1);
        });
    }
}
