use libloading::Library;
use std::collections::BTreeSet;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MAX_COMMAND: usize = 64;
type CArg = [c_char; MAX_COMMAND];
type ArgsFn = unsafe extern "C" fn(*const CArg, c_int);
type NoArgsFn = unsafe extern "C" fn();
type ProcessFn = unsafe extern "C" fn(*const c_char);
type ParseFn = unsafe extern "C" fn(*const c_char, *mut c_char, *mut CArg, *mut c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    static mut stdin: *mut c_void;
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn clearerr(stream: *mut c_void);
}

struct Pair {
    c: Library,
    rust: Library,
}

impl Pair {
    unsafe fn load() -> Self {
        Self {
            c: Library::new(c_library_path()).expect("load C shared library"),
            rust: Library::new(rust_library_path()).expect("load Rust shared library"),
        }
    }

    unsafe fn args(&self, symbol: &str, values: &[Vec<u8>]) -> (Vec<u8>, Vec<u8>) {
        let raw = make_args(values);
        self.args_raw(symbol, raw.as_ptr(), raw.len() as c_int)
    }

    unsafe fn args_raw(&self, symbol: &str, args: *const CArg, count: c_int) -> (Vec<u8>, Vec<u8>) {
        let name = symbol_name(symbol);
        let c = *self.c.get::<ArgsFn>(&name).expect("C args symbol");
        let rust = *self.rust.get::<ArgsFn>(&name).expect("Rust args symbol");
        (
            capture_stdout(|| c(args, count)),
            capture_stdout(|| rust(args, count)),
        )
    }

    unsafe fn no_args(&self, symbol: &str) -> (Vec<u8>, Vec<u8>) {
        let name = symbol_name(symbol);
        let c = *self.c.get::<NoArgsFn>(&name).expect("C no-args symbol");
        let rust = *self
            .rust
            .get::<NoArgsFn>(&name)
            .expect("Rust no-args symbol");
        (capture_stdout(|| c()), capture_stdout(|| rust()))
    }

    unsafe fn process(&self, input: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let input = CString::new(input).expect("command contains no NUL");
        let c = *self
            .c
            .get::<ProcessFn>(b"process_command\0")
            .expect("C process_command");
        let rust = *self
            .rust
            .get::<ProcessFn>(b"process_command\0")
            .expect("Rust process_command");
        (
            capture_stdout(|| c(input.as_ptr())),
            capture_stdout(|| rust(input.as_ptr())),
        )
    }

    unsafe fn parse(&self, input: &[u8]) -> ParseResult {
        let input = CString::new(input).expect("parser input contains no NUL");
        let c = *self
            .c
            .get::<ParseFn>(b"parse_command\0")
            .expect("C parse_command");
        let rust = *self
            .rust
            .get::<ParseFn>(b"parse_command\0")
            .expect("Rust parse_command");
        let c_result = call_parse(c, &input);
        let rust_result = call_parse(rust, &input);
        assert_eq!(c_result, rust_result, "parse_command input {input:?}");
        c_result
    }

    unsafe fn main_with_stdin(&self, input: &[u8]) -> (MainResult, MainResult) {
        let c = *self.c.get::<MainFn>(b"main\0").expect("C main");
        let rust = *self.rust.get::<MainFn>(b"main\0").expect("Rust main");
        (call_main(c, input), call_main(rust, input))
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ParseResult {
    command: CArg,
    args: [CArg; 10],
    count: c_int,
}

#[derive(Debug, Eq, PartialEq)]
struct MainResult {
    status: c_int,
    output: Vec<u8>,
}

unsafe fn call_parse(function: ParseFn, input: &CString) -> ParseResult {
    let mut command = [0x5a_i8; MAX_COMMAND];
    let mut args = [[0x5a_i8; MAX_COMMAND]; 10];
    let mut count = -777;
    function(
        input.as_ptr(),
        command.as_mut_ptr(),
        args.as_mut_ptr(),
        &mut count,
    );
    ParseResult {
        command,
        args,
        count,
    }
}

unsafe fn call_main(function: MainFn, input: &[u8]) -> MainResult {
    let mut input_pipe = [0; 2];
    assert_eq!(pipe(input_pipe.as_mut_ptr()), 0);
    let mut writer = File::from_raw_fd(input_pipe[1]);
    writer.write_all(input).expect("write redirected stdin");
    drop(writer);

    let saved_stdin = dup(0);
    assert!(saved_stdin >= 0);
    assert_eq!(dup2(input_pipe[0], 0), 0);
    close(input_pipe[0]);
    clearerr(stdin);
    let mut status = -1;
    let output = capture_stdout(|| status = function());
    assert_eq!(dup2(saved_stdin, 0), 0);
    close(saved_stdin);
    clearerr(stdin);
    MainResult { status, output }
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    fflush(std::ptr::null_mut());
    let mut output_pipe = [0; 2];
    assert_eq!(pipe(output_pipe.as_mut_ptr()), 0);
    let saved_stdout = dup(1);
    assert!(saved_stdout >= 0);
    assert_eq!(dup2(output_pipe[1], 1), 1);
    close(output_pipe[1]);

    call();
    fflush(std::ptr::null_mut());
    assert_eq!(dup2(saved_stdout, 1), 1);
    close(saved_stdout);

    let mut output = Vec::new();
    File::from_raw_fd(output_pipe[0])
        .read_to_end(&mut output)
        .expect("read redirected stdout");
    output
}

fn make_args(values: &[Vec<u8>]) -> Vec<CArg> {
    values
        .iter()
        .map(|value| {
            assert!(value.len() < MAX_COMMAND);
            assert!(!value.contains(&0));
            let mut arg = [0; MAX_COMMAND];
            for (destination, source) in arg.iter_mut().zip(value) {
                *destination = *source as c_char;
            }
            arg
        })
        .collect()
}

fn values(items: &[&str]) -> Vec<Vec<u8>> {
    items.iter().map(|item| item.as_bytes().to_vec()).collect()
}

fn symbol_name(symbol: &str) -> Vec<u8> {
    let mut name = symbol.as_bytes().to_vec();
    name.push(0);
    name
}

fn assert_match(label: &str, output: (Vec<u8>, Vec<u8>)) {
    assert_eq!(
        output.0,
        output.1,
        "{label}\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&output.0),
        String::from_utf8_lossy(&output.1)
    );
}

fn mark(rows: &mut BTreeSet<usize>, covered: &[usize]) {
    rows.extend(covered.iter().copied());
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver_c.so")
}

fn rust_library_path() -> PathBuf {
    std::env::current_exe()
        .expect("current integration test executable")
        .parent()
        .expect("integration test deps directory")
        .join("libdriver.so")
}

#[derive(Clone)]
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn word(&mut self, min: usize, span: usize) -> Vec<u8> {
        let len = min + self.next() as usize % span;
        (0..len).map(|_| b'a' + (self.next() % 26) as u8).collect()
    }
}

unsafe fn exercise_parser(configs: &mut BTreeSet<usize>) {
    let pair = Pair::load();
    pair.parse(b"");
    pair.parse(b" \t  ");
    mark(configs, &[1]);

    pair.parse(b"status");
    mark(configs, &[2]);

    let mut rng = Lcg::new(0x6a09_e667_f3bc_c909);
    for count in 1..=10 {
        let mut input = rng.word(1, 12);
        for index in 0..count {
            input.extend_from_slice(if index % 2 == 0 { b" \t" } else { b"\t " });
            input.extend_from_slice(&rng.word(1, 20));
        }
        pair.parse(&input);
    }
    mark(configs, &[3]);

    pair.parse(b"match a b c d e f g h i j k l m");
    mark(configs, &[4]);

    let mut long_tokens = vec![b'c'; 90];
    long_tokens.push(b' ');
    long_tokens.extend(vec![b'a'; 90]);
    pair.parse(&long_tokens);
    mark(configs, &[5]);

    let mut long_input = b"compare ".to_vec();
    long_input.extend(vec![b'x'; 400]);
    pair.parse(&long_input);
    mark(configs, &[6]);
}

unsafe fn exercise_string_and_system(configs: &mut BTreeSet<usize>, errors: &mut BTreeSet<usize>) {
    let pair = Pair::load();

    for (symbol, row) in [
        ("cmd_compare", 33),
        ("cmd_compareN", 34),
        ("cmd_startswith", 35),
        ("cmd_match", 36),
    ] {
        assert_match(
            &format!("{symbol} null/zero"),
            pair.args_raw(symbol, std::ptr::null(), 0),
        );
        mark(errors, &[row]);
    }

    let mut rng = Lcg::new(0xbb67_ae85_84ca_a73b);
    for _ in 0..24 {
        let word = rng.word(1, 30);
        assert_match(
            "compare equal",
            pair.args("cmd_compare", &[word.clone(), word.clone()]),
        );

        let mut less = vec![b'a'];
        less.extend(rng.word(1, 20));
        let mut greater = vec![b'z'];
        greater.extend(rng.word(1, 20));
        assert_match(
            "compare less",
            pair.args("cmd_compare", &[less.clone(), greater.clone()]),
        );
        assert_match(
            "compare greater",
            pair.args("cmd_compare", &[greater, less]),
        );
    }
    mark(configs, &[25, 26, 27]);

    for args in [
        values(&["abc", "xyz", "0"]),
        values(&["abcdef", "abcxyz", "3"]),
        values(&["abcdef", "abcdef", "99"]),
        values(&["abc", "abd", "3"]),
        values(&["abd", "abc", "3"]),
        values(&["abc", "abd", "-1"]),
    ] {
        assert_match("compareN branch", pair.args("cmd_compareN", &args));
    }
    mark(configs, &[28, 29, 30, 31, 32]);

    for args in [
        values(&["alphabet", "alpha"]),
        values(&["alphabet", "beta"]),
        vec![b"alphabet".to_vec(), Vec::new()],
    ] {
        assert_match("startswith branch", pair.args("cmd_startswith", &args));
    }
    mark(configs, &[33, 34, 35]);

    for args in [
        values(&["abc", "abc"]),
        values(&["abc", "zabcx"]),
        values(&["abc", "zzz"]),
        values(&["ab", "ab", "zab", "none", "xxabyy", "other"]),
        values(&["", "abc", "", "xyz"]),
    ] {
        assert_match("match branch", pair.args("cmd_match", &args));
    }
    mark(configs, &[36, 37, 38, 39, 69]);

    let oversized = make_args(&values(&[
        "same", "same", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10", "x11",
    ]));
    assert_match(
        "oversized arg_count",
        pair.args_raw("cmd_compare", oversized.as_ptr(), 12),
    );

    assert_match("help", pair.no_args("cmd_help"));
    mark(configs, &[40]);

    assert_match("debug query off", pair.args("cmd_debug", &[]));
    assert_match("debug on", pair.args("cmd_debug", &values(&["on"])));
    assert_match("debug query on", pair.args("cmd_debug", &[]));
    assert_match(
        "debug invalid",
        pair.args("cmd_debug", &values(&["invalid"])),
    );
    assert_match("debug off", pair.args("cmd_debug", &values(&["off"])));
    mark(configs, &[41]);
    mark(errors, &[37]);

    assert_match("verbose query off", pair.args("cmd_verbose", &[]));
    assert_match("verbose on", pair.args("cmd_verbose", &values(&["on"])));
    assert_match("verbose query on", pair.args("cmd_verbose", &[]));
    assert_match(
        "verbose invalid",
        pair.args("cmd_verbose", &values(&["invalid"])),
    );
    assert_match("verbose off", pair.args("cmd_verbose", &values(&["off"])));
    mark(configs, &[42]);
    mark(errors, &[38]);

    assert_match("empty status", pair.no_args("cmd_status"));
    mark(configs, &[43]);

    let mut matched_time = false;
    for _ in 0..4 {
        let output = pair.no_args("cmd_time");
        if output.0 == output.1 {
            matched_time = true;
            break;
        }
    }
    assert!(
        matched_time,
        "cmd_time repeatedly crossed a one-second boundary"
    );
    mark(configs, &[45]);
}

unsafe fn exercise_stateful_handlers(configs: &mut BTreeSet<usize>, errors: &mut BTreeSet<usize>) {
    let pair = Pair::load();

    for (symbol, row) in [("cmd_adduser", 1), ("cmd_login", 4)] {
        assert_match(
            &format!("{symbol} null/zero"),
            pair.args_raw(symbol, std::ptr::null(), 0),
        );
        mark(errors, &[row]);
    }
    for (symbol, row) in [
        ("cmd_readfile", 15),
        ("cmd_set", 26),
        ("cmd_get", 28),
        ("cmd_unset", 30),
    ] {
        assert_match(
            &format!("{symbol} null/negative"),
            pair.args_raw(symbol, std::ptr::null(), -1),
        );
        mark(errors, &[row]);
    }

    assert_match("logout empty", pair.no_args("cmd_logout"));
    assert_match("whoami empty", pair.no_args("cmd_whoami"));
    assert_match("listusers empty", pair.no_args("cmd_listusers"));
    assert_match("listfiles empty", pair.no_args("cmd_listfiles"));
    assert_match("listvars empty", pair.no_args("cmd_listvars"));
    mark(errors, &[8, 9, 10, 25, 32]);

    assert_match(
        "createfile logged out",
        pair.args("cmd_createfile", &values(&["nope"])),
    );
    assert_match(
        "writefile logged out",
        pair.args("cmd_writefile", &values(&["nope", "value"])),
    );
    assert_match(
        "deletefile logged out",
        pair.args("cmd_deletefile", &values(&["nope"])),
    );
    mark(errors, &[11, 17, 21]);

    assert_match(
        "default permission",
        pair.args("cmd_adduser", &values(&["owner", "pw"])),
    );
    assert_match(
        "explicit permission",
        pair.args("cmd_adduser", &values(&["writer", "pw", "5"])),
    );
    assert_match(
        "explicit admin permission",
        pair.args("cmd_adduser", &values(&["admin", "pw", "9"])),
    );
    assert_match(
        "nonnumeric permission",
        pair.args("cmd_adduser", &values(&["plain", "pw", "nonnumeric"])),
    );
    for index in 4..10 {
        assert_match(
            "fill users",
            pair.args(
                "cmd_adduser",
                &values(&[&format!("user{index}"), "pw", &format!("{}", index % 10)]),
            ),
        );
    }
    mark(configs, &[7, 8, 10]);

    assert_match(
        "duplicate user",
        pair.args("cmd_adduser", &values(&["owner", "pw"])),
    );
    assert_match(
        "maximum users",
        pair.args("cmd_adduser", &values(&["overflow", "pw"])),
    );
    mark(errors, &[2, 3]);

    assert_match(
        "unknown login",
        pair.args("cmd_login", &values(&["missing", "pw"])),
    );
    assert_match(
        "wrong password",
        pair.args("cmd_login", &values(&["owner", "bad"])),
    );
    assert_match(
        "owner login",
        pair.args("cmd_login", &values(&["owner", "pw"])),
    );
    assert_match(
        "already logged in",
        pair.args("cmd_login", &values(&["writer", "pw"])),
    );
    assert_match("whoami owner", pair.no_args("cmd_whoami"));
    assert_match("listusers logged in", pair.no_args("cmd_listusers"));
    mark(configs, &[9, 10]);
    mark(errors, &[5, 6, 7]);

    assert_match(
        "createfile missing args",
        pair.args_raw("cmd_createfile", std::ptr::null(), 0),
    );
    assert_match(
        "create empty file",
        pair.args("cmd_createfile", &values(&["empty"])),
    );
    assert_match(
        "create content file",
        pair.args("cmd_createfile", &values(&["owned", "initial"])),
    );
    assert_match(
        "duplicate file",
        pair.args("cmd_createfile", &values(&["owned", "again"])),
    );
    assert_match(
        "read existing",
        pair.args("cmd_readfile", &values(&["owned"])),
    );
    assert_match(
        "owner write",
        pair.args("cmd_writefile", &values(&["owned", "updated"])),
    );
    mark(configs, &[11, 12, 13, 14]);
    mark(errors, &[12, 14]);

    assert_match("owner logout", pair.no_args("cmd_logout"));
    assert_match(
        "plain login",
        pair.args("cmd_login", &values(&["plain", "pw"])),
    );
    assert_match(
        "write permission denied",
        pair.args("cmd_writefile", &values(&["owned", "denied"])),
    );
    assert_match(
        "delete permission denied",
        pair.args("cmd_deletefile", &values(&["owned"])),
    );
    mark(errors, &[19, 23]);
    assert_match("plain logout", pair.no_args("cmd_logout"));

    assert_match(
        "writer login",
        pair.args("cmd_login", &values(&["writer", "pw"])),
    );
    assert_match(
        "level five write",
        pair.args("cmd_writefile", &values(&["owned", "level5"])),
    );
    assert_match(
        "owner creates temporary",
        pair.args("cmd_createfile", &values(&["writer-file", "x"])),
    );
    assert_match(
        "owner deletes temporary",
        pair.args("cmd_deletefile", &values(&["writer-file"])),
    );
    mark(configs, &[15, 16]);

    assert_match(
        "write missing args",
        pair.args_raw("cmd_writefile", std::ptr::null(), 0),
    );
    assert_match(
        "delete missing args",
        pair.args_raw("cmd_deletefile", std::ptr::null(), 0),
    );
    assert_match(
        "write missing file",
        pair.args("cmd_writefile", &values(&["missing", "x"])),
    );
    assert_match(
        "delete missing file",
        pair.args("cmd_deletefile", &values(&["missing"])),
    );
    assert_match(
        "read missing file",
        pair.args("cmd_readfile", &values(&["missing"])),
    );
    mark(errors, &[16, 18, 20, 22, 24]);
    assert_match("writer logout", pair.no_args("cmd_logout"));

    assert_match(
        "admin login",
        pair.args("cmd_login", &values(&["admin", "pw"])),
    );
    assert_match(
        "admin delete nonowner empty",
        pair.args("cmd_deletefile", &values(&["empty"])),
    );
    assert_match(
        "admin delete nonowner owned",
        pair.args("cmd_deletefile", &values(&["owned"])),
    );
    mark(configs, &[17]);

    for index in 0..20 {
        assert_match(
            "fill files",
            pair.args(
                "cmd_createfile",
                &values(&[&format!("file{index:02}"), &format!("content{index:02}")]),
            ),
        );
    }
    assert_match(
        "maximum files",
        pair.args("cmd_createfile", &values(&["overflow", "x"])),
    );
    assert_match("list many files", pair.no_args("cmd_listfiles"));
    assert_match(
        "delete middle file",
        pair.args("cmd_deletefile", &values(&["file10"])),
    );
    assert_match("list shifted files", pair.no_args("cmd_listfiles"));
    mark(configs, &[18, 19]);
    mark(errors, &[13]);

    for index in 0..20 {
        assert_match(
            "fill variables",
            pair.args(
                "cmd_set",
                &values(&[&format!("var{index:02}"), &format!("value{index:02}")]),
            ),
        );
    }
    assert_match(
        "maximum variables",
        pair.args("cmd_set", &values(&["overflow", "x"])),
    );
    assert_match(
        "update variable",
        pair.args("cmd_set", &values(&["var05", "replacement"])),
    );
    assert_match("get variable", pair.args("cmd_get", &values(&["var05"])));
    assert_match("list variables", pair.no_args("cmd_listvars"));
    assert_match(
        "get missing variable",
        pair.args("cmd_get", &values(&["missing"])),
    );
    assert_match(
        "unset missing variable",
        pair.args("cmd_unset", &values(&["missing"])),
    );
    assert_match(
        "unset middle variable",
        pair.args("cmd_unset", &values(&["var10"])),
    );
    assert_match("list shifted variables", pair.no_args("cmd_listvars"));
    mark(configs, &[20, 21, 22, 23, 24]);
    mark(errors, &[27, 29, 31]);

    assert_match("populated status", pair.no_args("cmd_status"));
    mark(configs, &[44]);
}

unsafe fn exercise_process_routes(configs: &mut BTreeSet<usize>, errors: &mut BTreeSet<usize>) {
    {
        let pair = Pair::load();
        assert_match(
            "whitespace around process command",
            pair.process(b"   status \t "),
        );
        mark(configs, &[46]);
    }

    let pair = Pair::load();
    for command in [
        "adduser route pw 9",
        "listusers",
        "users",
        "login route pw",
        "whoami",
    ] {
        assert_match(command, pair.process(command.as_bytes()));
    }
    mark(configs, &[47, 48]);

    for command in [
        "createfile exact initial",
        "readfile exact",
        "writefile exact changed",
        "listfiles",
        "deletefile exact",
        "touch alias content",
        "cat alias",
        "write alias changed",
        "ls",
        "rm alias",
    ] {
        assert_match(command, pair.process(command.as_bytes()));
    }
    mark(configs, &[49, 50]);

    for command in [
        "set name value",
        "get name",
        "listvars",
        "vars",
        "unset name",
    ] {
        assert_match(command, pair.process(command.as_bytes()));
    }
    mark(configs, &[51, 52]);

    let mut rng = Lcg::new(0x3c6e_f372_fe94_f82b);
    for _ in 0..20 {
        let left = String::from_utf8(rng.word(1, 16)).unwrap();
        let right = String::from_utf8(rng.word(1, 16)).unwrap();
        for command in [
            format!("compare {left} {right}"),
            format!("cmp {left} {right}"),
            format!("compareN {left} {right} 4"),
            format!("cmpn {left} {right} 4"),
            format!("startswith {left} a"),
            format!("match a {left} {right}"),
        ] {
            assert_match(&command, pair.process(command.as_bytes()));
        }
    }
    mark(configs, &[53, 54]);

    for command in ["debug", "verbose", "status", "time", "help", "?"] {
        let mut matched = false;
        for _ in 0..4 {
            let output = pair.process(command.as_bytes());
            if output.0 == output.1 {
                matched = true;
                break;
            }
            assert_eq!(command, "time", "non-time system route diverged");
        }
        assert!(matched, "time route repeatedly crossed a second boundary");
    }
    mark(configs, &[55, 56]);

    assert_match("debug on route", pair.process(b"debug on"));
    assert_match("debug traced route", pair.process(b"status"));
    assert_match("debug off route", pair.process(b"debug off"));
    mark(configs, &[57]);

    for (command, row) in [
        ("addition", 58),
        ("logger", 59),
        ("listing", 60),
        ("created", 61),
        ("reader", 62),
        ("writerx", 63),
        ("deleter", 64),
    ] {
        assert_match(command, pair.process(command.as_bytes()));
        mark(configs, &[row]);
    }

    assert_match("unknown route", pair.process(b"definitely-unknown"));
    mark(errors, &[39]);

    assert_match("logout route", pair.process(b"logout"));
}

unsafe fn exercise_main(configs: &mut BTreeSet<usize>, errors: &mut BTreeSet<usize>) {
    let pair = Pair::load();
    let immediate_eof = pair.main_with_stdin(b"");
    assert_eq!(immediate_eof.0, immediate_eof.1, "main immediate EOF");
    mark(configs, &[66]);
    mark(errors, &[40]);

    let mut rng = Lcg::new(0xa54f_f53a_5f1d_36f1);
    for _ in 0..12 {
        let left = String::from_utf8(rng.word(1, 20)).unwrap();
        let right = String::from_utf8(rng.word(1, 20)).unwrap();
        let script = format!("compare {left} {right}\nstatus\n");
        let output = pair.main_with_stdin(script.as_bytes());
        assert_eq!(output.0, output.1, "main randomized script");
    }

    let long_line = {
        let mut line = vec![b'x'; 300];
        line.push(b'\n');
        line
    };
    let output = pair.main_with_stdin(&long_line);
    assert_eq!(output.0, output.1, "main 255-byte fgets chunks");
    mark(configs, &[67]);

    let output = pair.main_with_stdin(b"verbose on\nstatus\nverbose off\n");
    assert_eq!(output.0, output.1, "main verbose preprocessing");
    mark(configs, &[68]);
}

fn run_child(library: &Path, mode: &str) -> Output {
    Command::new(std::env::current_exe().expect("current integration test executable"))
        .arg("--exact")
        .arg("ffi_terminating_child")
        .arg("--nocapture")
        .env("DRIVER_CHILD_LIBRARY", library)
        .env("DRIVER_CHILD_MODE", mode)
        .output()
        .expect("run terminating child")
}

fn exercise_terminating_inputs(configs: &mut BTreeSet<usize>) {
    for command in ["exit", "quit"] {
        let c = run_child(&c_library_path(), command);
        let rust = run_child(&rust_library_path(), command);
        assert_eq!(c.status.code(), Some(0), "C {command} status: {c:?}");
        assert_eq!(
            rust.status.code(),
            Some(0),
            "Rust {command} status: {rust:?}"
        );
        assert_eq!(c.stdout, rust.stdout, "{command} child stdout");
        assert!(c.stdout.ends_with(b"Goodbye!\n"));
    }
    mark(configs, &[65]);
}

#[cfg(unix)]
fn exercise_null_inputs() {
    use std::os::unix::process::ExitStatusExt;

    for mode in ["null_parse", "null_process", "null_args"] {
        let c = run_child(&c_library_path(), mode);
        let rust = run_child(&rust_library_path(), mode);
        assert_eq!(
            c.status.signal(),
            rust.status.signal(),
            "{mode} signal mismatch\nC: {c:?}\nRust: {rust:?}"
        );
        assert_eq!(c.status.signal(), Some(11), "{mode} did not SIGSEGV");
    }
}

#[test]
fn ffi_terminating_child() {
    let Some(library) = std::env::var_os("DRIVER_CHILD_LIBRARY") else {
        return;
    };
    let mode = std::env::var("DRIVER_CHILD_MODE").expect("child mode");

    unsafe {
        let library = Library::new(library).expect("load child library");
        match mode.as_str() {
            "exit" | "quit" => {
                let function = *library
                    .get::<ProcessFn>(b"process_command\0")
                    .expect("process_command");
                let input = CString::new(mode).unwrap();
                function(input.as_ptr());
                panic!("terminating process_command returned");
            }
            "null_parse" => {
                let function = *library
                    .get::<ParseFn>(b"parse_command\0")
                    .expect("parse_command");
                let mut command = [0_i8; MAX_COMMAND];
                let mut args = [[0_i8; MAX_COMMAND]; 10];
                let mut count = 0;
                function(
                    std::ptr::null(),
                    command.as_mut_ptr(),
                    args.as_mut_ptr(),
                    &mut count,
                );
                panic!("null parse_command returned");
            }
            "null_process" => {
                let function = *library
                    .get::<ProcessFn>(b"process_command\0")
                    .expect("process_command");
                function(std::ptr::null());
                panic!("null process_command returned");
            }
            "null_args" => {
                let function = *library
                    .get::<ArgsFn>(b"cmd_compare\0")
                    .expect("cmd_compare");
                function(std::ptr::null(), 2);
                panic!("null cmd_compare returned");
            }
            _ => panic!("unknown child mode {mode}"),
        }
    }
}

#[test]
fn differential_surface() {
    assert!(c_library_path().is_file(), "missing C shared library");
    assert!(rust_library_path().is_file(), "missing Rust shared library");

    let mut configs = BTreeSet::new();
    let mut errors = BTreeSet::new();

    unsafe {
        exercise_parser(&mut configs);
        exercise_string_and_system(&mut configs, &mut errors);
        exercise_stateful_handlers(&mut configs, &mut errors);
        exercise_process_routes(&mut configs, &mut errors);
        exercise_main(&mut configs, &mut errors);
    }
    exercise_terminating_inputs(&mut configs);
    #[cfg(unix)]
    exercise_null_inputs();

    let expected_configs: BTreeSet<_> = (1..=69).collect();
    let expected_errors: BTreeSet<_> = (1..=40).collect();
    assert_eq!(configs, expected_configs, "uncovered CONFIGS.md rows");
    assert_eq!(errors, expected_errors, "uncovered ERRORS.md rows");
}
