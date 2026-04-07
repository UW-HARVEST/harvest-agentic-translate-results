use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::sync::Mutex;

const MAX_COMMAND: usize = 64;

// Serialize all tests since both libs use global state
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn c_lib_path() -> String {
    std::env::current_dir()
        .unwrap()
        .join("c_src/build/libdriver.so")
        .to_string_lossy()
        .into_owned()
}

fn rust_lib_path() -> String {
    let dir = std::env::current_dir().unwrap().join("target/debug/libdriver.so");
    dir.to_string_lossy().into_owned()
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        // Flush before redirecting
        libc::fflush(std::ptr::null_mut());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);

        let saved_stdout = libc::dup(1);
        assert!(saved_stdout >= 0);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);

        // Set read end to non-blocking and read all
        libc::fcntl(pipe_fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        let mut file = std::fs::File::from_raw_fd(pipe_fds[0]);
        let mut buf = String::new();
        file.read_to_string(&mut buf).ok();
        buf
    }
}

/// Reset both libraries' global state by loading fresh copies
/// We can't easily reset C globals, so we use dlopen/dlclose cycles.
/// Instead, we'll structure tests to account for cumulative state.

/// Run a sequence of commands through process_command on a freshly-loaded library
/// and return the concatenated stdout output.
unsafe fn run_commands_on_lib(lib: &Library, commands: &[&str]) -> String {
    let process_command: Symbol<unsafe extern "C" fn(*const c_char)> =
        lib.get(b"process_command").unwrap();

    let mut output = String::new();
    for cmd in commands {
        let c_cmd = CString::new(*cmd).unwrap();
        let captured = capture_stdout(|| {
            process_command(c_cmd.as_ptr());
        });
        output.push_str(&captured);
    }
    output
}

/// Compare a sequence of commands between C and Rust libraries.
/// Both start from their initial state (libraries freshly loaded).
fn compare_command_sequence(commands: &[&str]) {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    unsafe {
        // Load fresh copies each time to reset global state
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust lib");

        let c_output = run_commands_on_lib(&c_lib, commands);
        let rust_output = run_commands_on_lib(&rust_lib, commands);

        if c_output != rust_output {
            eprintln!("=== MISMATCH for commands: {:?} ===", commands);
            eprintln!("--- C output ---\n{}", c_output);
            eprintln!("--- Rust output ---\n{}", rust_output);
            // Show byte-level diff
            for (i, (cb, rb)) in c_output.bytes().zip(rust_output.bytes()).enumerate() {
                if cb != rb {
                    eprintln!(
                        "First diff at byte {}: C=0x{:02x}({}) Rust=0x{:02x}({})",
                        i, cb, cb as char, rb, rb as char
                    );
                    break;
                }
            }
            if c_output.len() != rust_output.len() {
                eprintln!(
                    "Length diff: C={} Rust={}",
                    c_output.len(),
                    rust_output.len()
                );
            }
            panic!("Output mismatch");
        }
    }
}

// ============ parse_command tests ============

#[test]
fn test_parse_command() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("Failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust lib");

        type ParseFn = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            *mut [c_char; MAX_COMMAND],
            *mut c_int,
        );

        let c_parse: Symbol<ParseFn> = c_lib.get(b"parse_command").unwrap();
        let r_parse: Symbol<ParseFn> = rust_lib.get(b"parse_command").unwrap();

        let inputs = [
            "hello world foo",
            "  adduser  alice  pass123  5 ",
            "singleword",
            "a b c d e f g h i j k l",  // more than MAX_ARGS
            "",
            "   ",
        ];

        for input in &inputs {
            let c_input = CString::new(*input).unwrap();

            let mut c_cmd = [0i8; MAX_COMMAND];
            let mut c_args = [[0i8; MAX_COMMAND]; 10];
            let mut c_argc: c_int = 0;

            let mut r_cmd = [0i8; MAX_COMMAND];
            let mut r_args = [[0i8; MAX_COMMAND]; 10];
            let mut r_argc: c_int = 0;

            c_parse(
                c_input.as_ptr(),
                c_cmd.as_mut_ptr(),
                c_args.as_mut_ptr(),
                &mut c_argc,
            );
            r_parse(
                c_input.as_ptr(),
                r_cmd.as_mut_ptr(),
                r_args.as_mut_ptr(),
                &mut r_argc,
            );

            assert_eq!(c_argc, r_argc, "arg_count mismatch for input: {:?}", input);
            assert_eq!(c_cmd, r_cmd, "cmd mismatch for input: {:?}", input);
            for i in 0..c_argc as usize {
                assert_eq!(
                    c_args[i], r_args[i],
                    "arg[{}] mismatch for input: {:?}",
                    i, input
                );
            }
        }
    }
}

// ============ process_command tests via compare_command_sequence ============

#[test]
fn test_help() {
    compare_command_sequence(&["help"]);
}

#[test]
fn test_status_initial() {
    compare_command_sequence(&["status"]);
}

#[test]
fn test_user_management() {
    compare_command_sequence(&[
        "adduser alice pass123",
        "adduser bob secret 5",
        "adduser alice pass123",       // duplicate
        "listusers",
        "login alice wrongpass",
        "login alice pass123",
        "whoami",
        "login bob secret",            // already logged in
        "logout",
        "login bob secret",
        "whoami",
        "logout",
        "logout",                      // no one logged in
        "whoami",                      // not logged in
    ]);
}

#[test]
fn test_adduser_usage() {
    compare_command_sequence(&["adduser", "adduser onlyname"]);
}

#[test]
fn test_file_management() {
    compare_command_sequence(&[
        "createfile test.txt",         // not logged in
        "adduser alice pass123",
        "login alice pass123",
        "createfile test.txt hello",
        "createfile test.txt",         // duplicate
        "readfile test.txt",
        "writefile test.txt newcontent",
        "readfile test.txt",
        "listfiles",
        "deletefile test.txt",
        "listfiles",
        "readfile test.txt",           // not found
    ]);
}

#[test]
fn test_file_permissions() {
    compare_command_sequence(&[
        "adduser alice pass123",
        "adduser bob secret 5",
        "login alice pass123",
        "createfile owned.txt data",
        "logout",
        "login bob secret",
        "writefile owned.txt overwritten",  // bob has perm 5, should succeed
        "readfile owned.txt",
        "deletefile owned.txt",             // bob has perm 5, needs 9 for delete
        "logout",
    ]);
}

#[test]
fn test_variable_management() {
    compare_command_sequence(&[
        "set myvar hello",
        "get myvar",
        "set myvar updated",
        "get myvar",
        "listvars",
        "set another world",
        "listvars",
        "unset myvar",
        "listvars",
        "get myvar",                   // not found
        "unset nonexistent",
    ]);
}

#[test]
fn test_variable_usage() {
    compare_command_sequence(&["set", "get", "unset"]);
}

#[test]
fn test_string_compare() {
    compare_command_sequence(&[
        "compare abc abc",
        "compare abc def",
        "compare def abc",
        "compare",                     // usage
    ]);
}

#[test]
fn test_string_comparen() {
    compare_command_sequence(&[
        "compareN hello help 3",
        "compareN hello hello 5",
        "compareN abc xyz 0",
        "compareN",                    // usage
    ]);
}

#[test]
fn test_startswith() {
    compare_command_sequence(&[
        "startswith hello hel",
        "startswith hello world",
        "startswith",                  // usage
    ]);
}

#[test]
fn test_match() {
    compare_command_sequence(&[
        "match abc abc abcdef xyz",
        "match test testing contest nothing",
        "match",                       // usage
    ]);
}

#[test]
fn test_debug_verbose() {
    compare_command_sequence(&[
        "debug",
        "debug on",
        "debug",
        "status",                      // with debug on, shows debug output
        "debug off",
        "verbose",
        "verbose on",
        "verbose",
        "verbose off",
    ]);
}

#[test]
fn test_debug_invalid() {
    compare_command_sequence(&["debug foo", "verbose bar"]);
}

#[test]
fn test_unknown_commands() {
    compare_command_sequence(&[
        "unknown",
        "adding",                      // starts with "add"
        "logging",                     // starts with "log"
        "listing",                     // starts with "list"
        "creating",                    // starts with "create"
        "reading",                     // starts with "read"
        "writing",                     // starts with "write"
        "deleting",                    // starts with "delete"
        "?",                           // alias for help
    ]);
}

#[test]
fn test_empty_input() {
    compare_command_sequence(&["", "   "]);
}

#[test]
fn test_command_aliases() {
    compare_command_sequence(&[
        "adduser alice pass123",
        "login alice pass123",
        "touch myfile.txt content",    // alias for createfile
        "cat myfile.txt",             // alias for readfile
        "write myfile.txt new",       // alias for writefile
        "ls",                         // alias for listfiles
        "rm myfile.txt",             // alias for deletefile
        "ls",
        "logout",
        "users",                      // alias for listusers
        "set x 1",
        "vars",                       // alias for listvars
        "cmp abc def",               // alias for compare
        "cmpn abc def 2",            // alias for compareN
    ]);
}

#[test]
fn test_max_users() {
    let mut cmds: Vec<String> = Vec::new();
    for i in 0..11 {  // MAX_USERS is 10, 11th should fail
        cmds.push(format!("adduser user{} pass{}", i, i));
    }
    let cmd_refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
    compare_command_sequence(&cmd_refs);
}

#[test]
fn test_login_not_found() {
    compare_command_sequence(&["login ghost pass"]);
}

#[test]
fn test_file_not_logged_in() {
    compare_command_sequence(&[
        "writefile x y",
        "deletefile x",
    ]);
}

#[test]
fn test_no_files_no_vars() {
    compare_command_sequence(&["listfiles", "listvars"]);
}

#[test]
fn test_readfile_usage() {
    compare_command_sequence(&["readfile"]);
}

#[test]
fn test_createfile_usage() {
    compare_command_sequence(&[
        "adduser a b",
        "login a b",
        "createfile",
    ]);
}

#[test]
fn test_writefile_usage() {
    compare_command_sequence(&[
        "adduser a b",
        "login a b",
        "writefile",
        "writefile onlyname",
    ]);
}

#[test]
fn test_deletefile_usage() {
    compare_command_sequence(&[
        "adduser a b",
        "login a b",
        "deletefile",
    ]);
}

#[test]
fn test_writefile_not_found() {
    compare_command_sequence(&[
        "adduser a b",
        "login a b",
        "writefile nonexistent content",
    ]);
}

#[test]
fn test_deletefile_not_found() {
    compare_command_sequence(&[
        "adduser a b",
        "login a b",
        "deletefile nonexistent",
    ]);
}

#[test]
fn test_adduser_permission_default() {
    compare_command_sequence(&[
        "adduser testuser testpass",
        "listusers",
    ]);
}

#[test]
fn test_debug_then_command() {
    compare_command_sequence(&[
        "debug on",
        "adduser alice pass123",
        "login alice pass123",
        "set x 1",
        "get x",
        "debug off",
    ]);
}

#[test]
fn test_verbose_then_command() {
    compare_command_sequence(&[
        "verbose on",
        "status",
        "verbose off",
    ]);
}

// ============ nm -D export comparison ============

#[test]
fn test_exports_match() {
    use std::process::Command;

    let c_out = Command::new("nm")
        .args(["-D", &c_lib_path()])
        .output()
        .expect("nm failed");
    let r_out = Command::new("nm")
        .args(["-D", &rust_lib_path()])
        .output()
        .expect("nm failed");

    let parse_exports = |output: &[u8]| -> Vec<String> {
        String::from_utf8_lossy(output)
            .lines()
            .filter(|l| l.contains(" T "))
            .map(|l| l.split_whitespace().last().unwrap().to_string())
            .filter(|s| !s.starts_with('_'))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    };

    let c_exports = parse_exports(&c_out.stdout);
    let r_exports = parse_exports(&r_out.stdout);

    let c_set: std::collections::BTreeSet<_> = c_exports.iter().collect();
    let r_set: std::collections::BTreeSet<_> = r_exports.iter().collect();

    let missing: Vec<_> = c_set.difference(&r_set).collect();
    assert!(
        missing.is_empty(),
        "Rust .so missing exports: {:?}",
        missing
    );
}
