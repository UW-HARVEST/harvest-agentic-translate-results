mod core;

use std::sync::Mutex;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

const MAX_COMMAND: usize = 64;

static STATE: Mutex<Option<core::State>> = Mutex::new(None);

fn with_state<F: FnOnce(&mut core::State)>(f: F) {
    let mut guard = STATE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(core::State::new());
    }
    f(guard.as_mut().unwrap());
}

/// Convert C args array (char args[][MAX_COMMAND]) to Vec<String>
unsafe fn args_to_vec(args: *const [c_char; MAX_COMMAND], arg_count: c_int) -> Vec<String> {
    let mut v = Vec::new();
    for i in 0..arg_count as usize {
        let ptr = (*args.add(i)).as_ptr();
        let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        v.push(s);
    }
    v
}

#[no_mangle]
pub unsafe extern "C" fn parse_command(
    input: *const c_char,
    cmd: *mut c_char,
    args: *mut [c_char; MAX_COMMAND],
    arg_count: *mut c_int,
) {
    let input_str = CStr::from_ptr(input).to_string_lossy();
    let (c, a) = core::parse_command(&input_str);

    // Copy command to cmd buffer
    let c_bytes = c.as_bytes();
    let copy_len = c_bytes.len().min(MAX_COMMAND - 1);
    std::ptr::copy_nonoverlapping(c_bytes.as_ptr(), cmd as *mut u8, copy_len);
    *cmd.add(copy_len) = 0;

    // Copy args
    let count = a.len().min(10); // MAX_ARGS
    for i in 0..count {
        let arg_bytes = a[i].as_bytes();
        let alen = arg_bytes.len().min(MAX_COMMAND - 1);
        let dest = (*args.add(i)).as_mut_ptr();
        std::ptr::copy_nonoverlapping(arg_bytes.as_ptr(), dest as *mut u8, alen);
        *dest.add(alen) = 0;
    }
    *arg_count = count as c_int;
}

#[no_mangle]
pub unsafe extern "C" fn cmd_adduser(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_adduser(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_login(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_login(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_logout() {
    with_state(|s| core::cmd_logout(s));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_whoami() {
    with_state(|s| core::cmd_whoami(s));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_listusers() {
    with_state(|s| core::cmd_listusers(s));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_createfile(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_createfile(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_readfile(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_readfile(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_writefile(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_writefile(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_deletefile(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_deletefile(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_listfiles() {
    with_state(|s| core::cmd_listfiles(s));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_set(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_set(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_get(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_get(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_unset(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_unset(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_listvars() {
    with_state(|s| core::cmd_listvars(s));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_compare(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    core::cmd_compare(&a);
}

#[no_mangle]
pub unsafe extern "C" fn cmd_compareN(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    core::cmd_comparen(&a);
}

#[no_mangle]
pub unsafe extern "C" fn cmd_startswith(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    core::cmd_startswith(&a);
}

#[no_mangle]
pub unsafe extern "C" fn cmd_match(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    core::cmd_match(&a);
}

#[no_mangle]
pub unsafe extern "C" fn cmd_help() {
    core::cmd_help();
}

#[no_mangle]
pub unsafe extern "C" fn cmd_debug(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_debug(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_verbose(args: *const [c_char; MAX_COMMAND], arg_count: c_int) {
    let a = args_to_vec(args, arg_count);
    with_state(|s| core::cmd_verbose(s, &a));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_status() {
    with_state(|s| core::cmd_status(s));
}

#[no_mangle]
pub unsafe extern "C" fn cmd_time() {
    core::cmd_time();
}

#[no_mangle]
pub unsafe extern "C" fn process_command(input: *const c_char) {
    let input_str = CStr::from_ptr(input).to_string_lossy();
    with_state(|s| core::process_command(s, &input_str));
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    // Match C main - interactive loop; not useful for .so testing but needed for symbol parity
    0
}
