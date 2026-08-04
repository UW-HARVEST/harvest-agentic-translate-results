// Integration tests that compare the C reference implementation against the
// Rust translation by running each as a subprocess and comparing stdout
// byte-for-byte.
//
// Both the C source and Rust source produce executables (with main()), so
// libloading-style FFI loading is not applicable. The `libloading` crate is
// still listed under [dev-dependencies] per the task instructions.
//
// Tests start from the lowest-level command handlers (compare/compareN/etc.)
// and work upward to higher-level scenarios that exercise multiple commands
// together.
//
// NOTE: The C `process_command` function calls `strlen` on the local
// `command` buffer in main.c without initializing it, so when no token is
// produced (empty input or pure whitespace) the C output is non-deterministic.
// We never feed empty or whitespace-only lines to either program.

#![allow(unused_imports)]

use libloading as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    project_root().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn ensure_built() {
    assert!(
        c_binary().exists(),
        "C binary not built at {:?}. Run cmake/make in c_src/build first.",
        c_binary()
    );
    assert!(
        rust_binary().exists(),
        "Rust binary not built at {:?}.",
        rust_binary()
    );
}

fn run_program(bin: &Path, input: &str) -> Vec<u8> {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e));
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait_with_output");
    out.stdout
}

fn compare_outputs(label: &str, input: &str) {
    ensure_built();
    let c_out = run_program(&c_binary(), input);
    let r_out = run_program(&rust_binary(), input);
    if c_out != r_out {
        let c_str = String::from_utf8_lossy(&c_out);
        let r_str = String::from_utf8_lossy(&r_out);
        panic!(
            "Mismatch in test '{}'\n\
             ---- INPUT ----\n{}\n\
             ---- C OUTPUT ----\n{}\n\
             ---- RUST OUTPUT ----\n{}\n",
            label, input, c_str, r_str
        );
    }
}

// Helper to keep tests focused: the program only exits when it receives
// "exit" or EOF. Always end input with "exit\n".
fn build_input(commands: &[&str]) -> String {
    let mut s = String::new();
    for c in commands {
        s.push_str(c);
        s.push('\n');
    }
    s.push_str("exit\n");
    s
}

// ====================== Lowest-level commands ======================

#[test]
fn t_compare_equal() {
    compare_outputs(
        "compare equal",
        &build_input(&["compare hello hello"]),
    );
}

#[test]
fn t_compare_less() {
    compare_outputs(
        "compare less",
        &build_input(&["compare abc abd"]),
    );
}

#[test]
fn t_compare_greater() {
    compare_outputs(
        "compare greater",
        &build_input(&["compare zzz aaa"]),
    );
}

#[test]
fn t_compare_prefix() {
    // One string is a prefix of the other - exercises the libc behavior of
    // returning the next char value.
    compare_outputs(
        "compare prefix",
        &build_input(&["compare abc abcd", "compare abcd abc"]),
    );
}

#[test]
fn t_compare_usage() {
    compare_outputs(
        "compare usage",
        &build_input(&["compare", "compare onlyone"]),
    );
}

#[test]
fn t_compare_alias_cmp() {
    compare_outputs(
        "compare via cmp alias",
        &build_input(&["cmp foo bar"]),
    );
}

#[test]
fn t_compareN_basic() {
    compare_outputs(
        "compareN basic",
        &build_input(&[
            "compareN hello world 3",
            "compareN abcdef abcxyz 3",
            "compareN abcdef abcxyz 4",
            "compareN same same 5",
            "compareN abc abcd 5",
        ]),
    );
}

#[test]
fn t_compareN_zero() {
    // n=0 should always print 0 / "First 0 characters are equal"
    compare_outputs(
        "compareN zero",
        &build_input(&["compareN abc xyz 0"]),
    );
}

#[test]
fn t_compareN_usage() {
    compare_outputs(
        "compareN usage",
        &build_input(&["compareN", "compareN onlyone", "compareN one two"]),
    );
}

#[test]
fn t_compareN_alias_cmpn() {
    compare_outputs(
        "compareN via cmpn alias",
        &build_input(&["cmpn abc abd 2"]),
    );
}

#[test]
fn t_startswith_basic() {
    compare_outputs(
        "startswith basic",
        &build_input(&[
            "startswith hello hel",
            "startswith hello world",
            "startswith hello hello",
            "startswith hi hello",
        ]),
    );
}

#[test]
fn t_startswith_usage() {
    compare_outputs(
        "startswith usage",
        &build_input(&["startswith", "startswith only"]),
    );
}

#[test]
fn t_match_basic() {
    compare_outputs(
        "match basic",
        &build_input(&[
            "match foo foo bar foobar fo",
            "match abc abcdef xabcd qrabc abc nomatch",
        ]),
    );
}

#[test]
fn t_match_usage() {
    compare_outputs(
        "match usage",
        &build_input(&["match", "match onlypattern"]),
    );
}

// ====================== Variable commands ======================

#[test]
fn t_variables_basic() {
    compare_outputs(
        "variables basic",
        &build_input(&[
            "listvars",
            "set x 42",
            "set y hello",
            "listvars",
            "get x",
            "get y",
            "get notfound",
            "set x 99",
            "get x",
            "unset y",
            "get y",
            "listvars",
            "unset notthere",
        ]),
    );
}

#[test]
fn t_variables_usage() {
    compare_outputs(
        "variables usage",
        &build_input(&["set", "set onlykey", "get", "unset"]),
    );
}

#[test]
fn t_variables_max() {
    // Add 21 vars to exceed MAX_VARIABLES (20)
    let mut cmds: Vec<String> = (0..21).map(|i| format!("set v{} val{}", i, i)).collect();
    cmds.push("listvars".to_string());
    let cmds_refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
    compare_outputs("variables max", &build_input(&cmds_refs));
}

#[test]
fn t_variables_alias_vars() {
    compare_outputs(
        "vars alias",
        &build_input(&["set a b", "vars"]),
    );
}

// ====================== User commands ======================

#[test]
fn t_user_listusers_empty() {
    compare_outputs("listusers empty", &build_input(&["listusers"]));
}

#[test]
fn t_user_basic_flow() {
    compare_outputs(
        "user basic flow",
        &build_input(&[
            "adduser alice secret 5",
            "adduser bob hunter2",
            "listusers",
            "users", // alias
            "login alice secret",
            "whoami",
            "listusers",
            "logout",
            "whoami",
            "logout", // already logged out
        ]),
    );
}

#[test]
fn t_user_login_failures() {
    compare_outputs(
        "user login failures",
        &build_input(&[
            "adduser alice secret",
            "login alice wrongpass",
            "login nosuch pass",
            "login alice secret",
            "login alice secret", // already logged in
            "logout",
        ]),
    );
}

#[test]
fn t_user_duplicate() {
    compare_outputs(
        "duplicate user",
        &build_input(&[
            "adduser alice s",
            "adduser alice s",
        ]),
    );
}

#[test]
fn t_user_usage() {
    compare_outputs(
        "user command usage",
        &build_input(&["adduser", "adduser only", "login", "login only", "whoami"]),
    );
}

#[test]
fn t_user_max() {
    // MAX_USERS = 10. Add 11 users.
    let mut cmds: Vec<String> = (0..11).map(|i| format!("adduser u{} p{}", i, i)).collect();
    cmds.push("listusers".to_string());
    let cmds_refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
    compare_outputs("max users", &build_input(&cmds_refs));
}

// ====================== File commands ======================

#[test]
fn t_file_no_login() {
    compare_outputs(
        "file commands without login",
        &build_input(&[
            "createfile foo",
            "writefile foo bar",
            "deletefile foo",
            "readfile foo", // doesn't require login
            "listfiles",
        ]),
    );
}

#[test]
fn t_file_basic_flow() {
    compare_outputs(
        "file basic flow",
        &build_input(&[
            "adduser alice s",
            "login alice s",
            "createfile foo hello",
            "createfile bar",
            "listfiles",
            "ls", // alias
            "readfile foo",
            "readfile bar",
            "readfile nosuch",
            "writefile foo updated",
            "readfile foo",
            "deletefile bar",
            "listfiles",
            "deletefile nosuch",
        ]),
    );
}

#[test]
fn t_file_aliases() {
    compare_outputs(
        "file aliases",
        &build_input(&[
            "adduser alice s",
            "login alice s",
            "touch hello",
            "cat hello",
            "write hello world",
            "rm hello",
        ]),
    );
}

#[test]
fn t_file_duplicate() {
    compare_outputs(
        "duplicate file",
        &build_input(&[
            "adduser alice s",
            "login alice s",
            "createfile foo",
            "createfile foo",
        ]),
    );
}

#[test]
fn t_file_permission_denied_write() {
    // Alice creates file, bob (low perm) tries to write → denied
    compare_outputs(
        "file write permission denied",
        &build_input(&[
            "adduser alice apass 1",
            "adduser bob bpass 1",
            "login alice apass",
            "createfile shared content",
            "logout",
            "login bob bpass",
            "writefile shared evil",
            "deletefile shared",
            "logout",
        ]),
    );
}

#[test]
fn t_file_permission_admin() {
    // Bob has high perm so can overwrite/delete others' files
    compare_outputs(
        "file admin perm",
        &build_input(&[
            "adduser alice apass 1",
            "adduser bob bpass 9",
            "login alice apass",
            "createfile shared content",
            "logout",
            "login bob bpass",
            "writefile shared overwritten",
            "readfile shared",
            "deletefile shared",
            "listfiles",
        ]),
    );
}

#[test]
fn t_file_usage() {
    compare_outputs(
        "file commands usage",
        &build_input(&[
            "adduser alice s",
            "login alice s",
            "createfile",
            "readfile",
            "writefile",
            "writefile only",
            "deletefile",
        ]),
    );
}

#[test]
fn t_file_max() {
    // MAX_FILES = 20. Try to create 21.
    let mut cmds = vec![
        "adduser alice s".to_string(),
        "login alice s".to_string(),
    ];
    for i in 0..21 {
        cmds.push(format!("createfile f{} c{}", i, i));
    }
    cmds.push("listfiles".to_string());
    let cmds_refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
    compare_outputs("max files", &build_input(&cmds_refs));
}

// ====================== System commands ======================

#[test]
fn t_help() {
    compare_outputs("help", &build_input(&["help", "?"]));
}

#[test]
fn t_status() {
    compare_outputs(
        "status",
        &build_input(&[
            "status",
            "adduser alice s",
            "login alice s",
            "set x 1",
            "createfile f",
            "status",
        ]),
    );
}

#[test]
fn t_debug_toggle() {
    compare_outputs(
        "debug toggle",
        &build_input(&[
            "debug",
            "debug on",
            "status",
            "compare a b",
            "debug off",
            "compare a b",
            "debug bogus",
        ]),
    );
}

#[test]
fn t_verbose_toggle() {
    // Note: verbose only adds a "[VERBOSE] Processing: '...'" line which is
    // emitted in the read loop, not from process_command. Check parity.
    compare_outputs(
        "verbose toggle",
        &build_input(&[
            "verbose",
            "verbose on",
            "compare a b",
            "verbose off",
            "compare a b",
            "verbose bogus",
        ]),
    );
}

// ====================== Did-you-mean fallback ======================

#[test]
fn t_did_you_mean() {
    compare_outputs(
        "did-you-mean fallbacks",
        &build_input(&[
            "addxyz",       // strncmp("add",3) match
            "logxx",        // log
            "listxxx",      // list
            "createxx",     // create
            "readxx",       // read
            "writexx",      // write
            "deletexx",     // delete
            "totallybogus", // unknown
        ]),
    );
}

// ====================== Long-token / truncation ======================

#[test]
fn t_long_args_truncated() {
    // MAX_COMMAND = 64 → strncpy keeps 63 bytes. Both implementations should
    // truncate identically.
    let very_long = "a".repeat(80);
    let cmd = format!("compare {} bbb", very_long);
    compare_outputs("long args truncated", &build_input(&[&cmd]));
}

#[test]
fn t_long_input_line_truncated() {
    // MAX_INPUT = 256. Provide a long line of arguments to make sure the
    // implementations handle the input copy bound the same way.
    let mut line = String::from("compare ");
    line.push_str(&"x".repeat(300));
    line.push_str(" y");
    compare_outputs("long input truncated", &build_input(&[&line]));
}

#[test]
fn t_extra_whitespace_tabs() {
    // Multiple spaces and tabs should be collapsed by strtok the same way.
    compare_outputs(
        "extra whitespace",
        &build_input(&[
            "compare    abc   abd",
            "compare\tfoo\tbar",
            "  compare leading spaces", // leading space before command
            "\tcompare tabbed first arg",
        ]),
    );
}

#[test]
fn t_max_args_truncation() {
    // MAX_ARGS = 10. Provide more args than that to "match" and confirm
    // identical handling.
    compare_outputs(
        "many args (capped at MAX_ARGS=10)",
        &build_input(&[
            "match a a a a a a a a a a a a a a a", // 15 tokens after cmd
        ]),
    );
}

// ====================== Composite scenario ======================

#[test]
fn t_full_workflow() {
    compare_outputs(
        "full workflow",
        &build_input(&[
            "help",
            "status",
            "adduser admin adminpass 9",
            "adduser alice alicepass 1",
            "adduser bob bobpass 1",
            "login alice alicepass",
            "createfile alice_doc \"hi\"",
            "writefile alice_doc updated",
            "logout",
            "login admin adminpass",
            "deletefile alice_doc",
            "listfiles",
            "set greeting hello",
            "get greeting",
            "set greeting world",
            "get greeting",
            "unset greeting",
            "compare hello hello",
            "compareN apple apricot 2",
            "startswith superman super",
            "match cat catalog scatter dog cat",
            "debug on",
            "status",
            "debug off",
            "logout",
        ]),
    );
}
