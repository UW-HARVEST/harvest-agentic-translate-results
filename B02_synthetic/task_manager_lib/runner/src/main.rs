#![cfg_attr(fuzzing, no_main)]

use cando2::*;
use std::{env, fs, io};

const DEFAULT_LOG_FILE: &str = "default.log";

harness! {
    state: {
        tasks: Option<CString>,
        env_max_tasks: Option<String>,
        env_log_file_name: Option<String>,
        log_file_contents: Option<String>,
        returns: c_int,
    },

    library: "driver",
    symbol: "driver",

    signature: unsafe extern "C" fn(*const c_char) -> c_int,

    fn run(&mut self) {
        // Just in case cleanup of env vars
        unsafe {
            env::remove_var("MAX_TASKS");
            env::remove_var("LOG_FILE");
        }

        if let Some(max_tasks) = &self.env_max_tasks {
            unsafe { env::set_var("MAX_TASKS", max_tasks); }
        }
        let log_file_name = match &self.env_log_file_name {
            Some(fname) => {
                unsafe { env::set_var("LOG_FILE", fname); }
                fname.to_string()
            },
            None => DEFAULT_LOG_FILE.to_string(),
        };

        let tasks = match &self.tasks {
            Some(t) => t.as_ptr(),
            None => std::ptr::null(),
        };

        self.returns = unsafe {
            (*SYMBOL)(tasks)
        };

        self.log_file_contents = match fs::read_to_string(&log_file_name) {
            Ok(c) => Some(c),
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => None,
                _ => panic!("Rust Runner: Error reading file"),
            }
        };

        // Cleanup file + env vars
        let _ = fs::remove_file(log_file_name);
        unsafe {
            env::remove_var("MAX_TASKS");
            env::remove_var("LOG_FILE");
        }
    }
}
