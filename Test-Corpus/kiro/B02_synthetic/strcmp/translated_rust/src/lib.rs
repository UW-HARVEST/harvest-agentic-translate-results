// Re-export all C-equivalent symbols from the Rust translation.
// This module provides #[no_mangle] extern "C" functions matching
// the C shared library's exported symbols.

#[path = "main.rs"]
#[allow(dead_code)]
mod driver;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::Mutex;

use driver::{State, parse_command as rs_parse_command, MAX_COMMAND, MAX_ARGS};

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<F: FnOnce(&mut State)>(f: F) {
    let mut guard = STATE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(State::new());
    }
    f(guard.as_mut().unwrap());
}

fn with_state_ref<F: FnOnce(&State)>(f: F) {
    let mut guard = STATE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(State::new());
    }
    f(guard.as_ref().unwrap());
}

/// Convert C args array to Rust Vec<String>
unsafe fn c_args_to_vec(args: *const [c_char; 64], arg_count: c_int) -> Vec<String> {
    let mut result = Vec::new();
    for i in 0..arg_count as usize {
        let ptr = (*args.add(i)).as_ptr();
        let s = CStr::from_ptr(ptr).to_string_lossy().to_string();
        result.push(s);
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn parse_command(
    input: *const c_char,
    cmd: *mut c_char,
    args: *mut [c_char; 64], // MAX_COMMAND = 64
    arg_count: *mut c_int,
) {
    let input_str = CStr::from_ptr(input).to_string_lossy();
    let (parsed_cmd, parsed_args) = rs_parse_command(&input_str);

    // Copy command to output buffer
    let cmd_bytes = parsed_cmd.as_bytes();
    let copy_len = cmd_bytes.len().min(MAX_COMMAND - 1);
    std::ptr::copy_nonoverlapping(cmd_bytes.as_ptr(), cmd as *mut u8, copy_len);
    *cmd.add(copy_len) = 0;

    // Copy args
    *arg_count = parsed_args.len().min(MAX_ARGS) as c_int;
    for i in 0..*arg_count as usize {
        let arg_bytes = parsed_args[i].as_bytes();
        let copy_len = arg_bytes.len().min(MAX_COMMAND - 1);
        let dest = (*args.add(i)).as_mut_ptr();
        std::ptr::copy_nonoverlapping(arg_bytes.as_ptr(), dest as *mut u8, copy_len);
        *dest.add(copy_len) = 0;
    }
}

#[no_mangle]
pub unsafe extern "C" fn cmd_adduser(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state(|s| s.cmd_adduser(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_login(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state(|s| s.cmd_login(&a));
}

#[no_mangle]
pub extern "C" fn cmd_logout() {
    with_state(|s| s.cmd_logout());
}

#[no_mangle]
pub extern "C" fn cmd_whoami() {
    with_state_ref(|s| s.cmd_whoami());
}

#[no_mangle]
pub extern "C" fn cmd_listusers() {
    with_state_ref(|s| s.cmd_listusers());
}

#[no_mangle]
pub unsafe extern "C" fn cmd_createfile(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state(|s| s.cmd_createfile(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_readfile(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state_ref(|s| s.cmd_readfile(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_writefile(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state(|s| s.cmd_writefile(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_deletefile(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state(|s| s.cmd_deletefile(&a));
}

#[no_mangle]
pub extern "C" fn cmd_listfiles() {
    with_state_ref(|s| s.cmd_listfiles());
}

#[no_mangle]
pub unsafe extern "C" fn cmd_set(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state(|s| s.cmd_set(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_get(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state_ref(|s| s.cmd_get(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_unset(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state(|s| s.cmd_unset(&a));
}

#[no_mangle]
pub extern "C" fn cmd_listvars() {
    with_state_ref(|s| s.cmd_listvars());
}

#[no_mangle]
pub unsafe extern "C" fn cmd_compare(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state_ref(|s| s.cmd_compare(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_compareN(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state_ref(|s| s.cmd_comparen(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_startswith(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state_ref(|s| s.cmd_startswith(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_match(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state_ref(|s| s.cmd_match(&a));
}

#[no_mangle]
pub extern "C" fn cmd_help() {
    with_state_ref(|s| s.cmd_help());
}

#[no_mangle]
pub unsafe extern "C" fn cmd_debug(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state(|s| s.cmd_debug(&a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_verbose(args: *const [c_char; 64], arg_count: c_int) {
    let a = c_args_to_vec(args, arg_count);
    with_state(|s| s.cmd_verbose(&a));
}

#[no_mangle]
pub extern "C" fn cmd_status() {
    with_state_ref(|s| s.cmd_status());
}

#[no_mangle]
pub extern "C" fn cmd_time() {
    with_state_ref(|s| s.cmd_time());
}

#[no_mangle]
pub unsafe extern "C" fn process_command(input: *const c_char) {
    let input_str = CStr::from_ptr(input).to_string_lossy();
    with_state(|s| s.process_command(&input_str));
}

// Export `main` to match C .so (which exports it as __c_main).
// We replicate the main loop logic here.
#[no_mangle]
pub extern "C" fn main() -> c_int {
    use std::io::{self, BufRead, Write};

    print!("|----------------------------------------|\n");
    print!("|   COMMAND INTERPRETER                  |\n");
    print!("|   strcmp/strncmp demonstration         |\n");
    print!("|----------------------------------------|\n");
    print!("Type 'help' for available commands\n\n");

    let stdin = io::stdin();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) | Err(_) => break,
            _ => {}
        }
        if input.ends_with('\n') {
            input.pop();
        }
        with_state(|s| {
            if s.verbose_mode {
                print!("[VERBOSE] Processing: '{}'\n", input);
            }
            s.process_command(&input);
        });
    }
    0
}
