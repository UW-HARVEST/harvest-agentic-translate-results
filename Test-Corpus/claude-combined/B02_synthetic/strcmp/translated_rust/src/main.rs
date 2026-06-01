// Translation of c_src/src/main.c to Rust.
// Aims to produce byte-identical output for the same inputs.

use std::ffi::CStr;
use std::io::{self, Read, Write};
use std::process::exit;
use std::ptr;

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

#[derive(Clone, Default)]
struct User {
    name: String,
    password: String,
    permission_level: i32,
    logged_in: bool,
}

#[derive(Clone, Default)]
struct FileT {
    filename: String,
    content: String,
    owner: String,
    permissions: i32,
}

#[derive(Clone, Default)]
struct Variable {
    name: String,
    value: String,
}

struct State {
    users: Vec<User>,
    current_user: Option<usize>,
    files: Vec<FileT>,
    variables: Vec<Variable>,
    debug_mode: bool,
    verbose_mode: bool,
}

impl State {
    fn new() -> Self {
        State {
            users: Vec::new(),
            current_user: None,
            files: Vec::new(),
            variables: Vec::new(),
            debug_mode: false,
            verbose_mode: false,
        }
    }
}

// Mimic C's atoi: skip leading whitespace, optional sign, parse digits, stop on non-digit.
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n'
        || bytes[i] == b'\r' || bytes[i] == 0x0b || bytes[i] == 0x0c) {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i32;
        result = result.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    sign.wrapping_mul(result)
}

// Implement fgets-like behavior: reads up to max-1 bytes from stdin, stops at newline (included).
// Returns None on EOF with no data read, otherwise Some(bytes_read).
fn fgets(stdin: &mut io::Stdin, buf: &mut Vec<u8>, max: usize) -> Option<()> {
    buf.clear();
    let mut byte = [0u8; 1];
    let mut got_any = false;
    while buf.len() + 1 < max {
        match stdin.read(&mut byte) {
            Ok(0) => {
                if !got_any {
                    return None;
                }
                return Some(());
            }
            Ok(_) => {
                got_any = true;
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    return Some(());
                }
            }
            Err(_) => {
                if !got_any {
                    return None;
                }
                return Some(());
            }
        }
    }
    Some(())
}

// strncpy-like truncation to (max - 1) bytes, mimicking C's behavior of writing null
// at index max-1. The result is a String; we also truncate from the leftmost.
fn truncate_to(s: &str, max_with_nul: usize) -> String {
    let bytes = s.as_bytes();
    let max_len = max_with_nul.saturating_sub(1);
    if bytes.len() <= max_len {
        s.to_string()
    } else {
        // Truncate to max_len bytes. Try to keep valid UTF-8 if possible, but we work in bytes.
        // Since inputs are typically ASCII for this program, this should be fine.
        let mut end = max_len;
        // Step back to a valid UTF-8 boundary if we end up in the middle of a code point.
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        s[..end].to_string()
    }
}

// Parse command and args by splitting on space/tab (strtok behavior with " \t").
// First token becomes cmd (truncated to MAX_COMMAND-1), subsequent tokens are args.
fn parse_command(input: &str) -> (String, Vec<String>) {
    let mut tokens: Vec<&str> = input.split(|c: char| c == ' ' || c == '\t')
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return (String::new(), Vec::new());
    }
    let cmd = truncate_to(tokens.remove(0), MAX_COMMAND);
    let mut args: Vec<String> = Vec::new();
    for tok in tokens.iter() {
        if args.len() >= MAX_ARGS {
            break;
        }
        args.push(truncate_to(tok, MAX_COMMAND));
    }
    (cmd, args)
}

// User management commands
fn cmd_adduser(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: adduser <username> <password> [permission_level]");
        return;
    }
    if state.users.len() >= MAX_USERS {
        println!("Error: Maximum users reached");
        return;
    }
    for u in &state.users {
        if u.name == args[0] {
            println!("Error: User '{}' already exists", args[0]);
            return;
        }
    }
    let permission_level = if args.len() >= 3 { c_atoi(&args[2]) } else { 1 };
    state.users.push(User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level,
        logged_in: false,
    });
    println!("User '{}' added with permission level {}", args[0], permission_level);
}

fn cmd_login(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: login <username> <password>");
        return;
    }
    if let Some(idx) = state.current_user {
        if state.users[idx].logged_in {
            println!("Error: User '{}' already logged in. Use 'logout' first.", state.users[idx].name);
            return;
        }
    }
    for i in 0..state.users.len() {
        if state.users[i].name == args[0] {
            if state.users[i].password == args[1] {
                state.users[i].logged_in = true;
                state.current_user = Some(i);
                println!("Login successful. Welcome, {}!", state.users[i].name);
                return;
            } else {
                println!("Error: Incorrect password");
                return;
            }
        }
    }
    println!("Error: User not found");
}

fn cmd_logout(state: &mut State) {
    let logged_in = match state.current_user {
        Some(idx) => state.users[idx].logged_in,
        None => false,
    };
    if !logged_in {
        println!("Error: No user logged in");
        return;
    }
    let idx = state.current_user.unwrap();
    println!("Goodbye, {}!", state.users[idx].name);
    state.users[idx].logged_in = false;
    state.current_user = None;
}

fn cmd_whoami(state: &State) {
    let logged_in = match state.current_user {
        Some(idx) => state.users[idx].logged_in,
        None => false,
    };
    if !logged_in {
        println!("Not logged in");
        return;
    }
    let idx = state.current_user.unwrap();
    println!("Current user: {}", state.users[idx].name);
    println!("Permission level: {}", state.users[idx].permission_level);
}

fn cmd_listusers(state: &State) {
    if state.users.is_empty() {
        println!("No users registered");
        return;
    }
    println!("Registered users:");
    for u in &state.users {
        println!("  {} (level {}) {}",
            u.name,
            u.permission_level,
            if u.logged_in { "[logged in]" } else { "" });
    }
}

// File management commands
fn is_logged_in(state: &State) -> bool {
    match state.current_user {
        Some(idx) => state.users[idx].logged_in,
        None => false,
    }
}

fn cmd_createfile(state: &mut State, args: &[String]) {
    if !is_logged_in(state) {
        println!("Error: Must be logged in");
        return;
    }
    if args.is_empty() {
        println!("Usage: createfile <filename> [content]");
        return;
    }
    if state.files.len() >= MAX_FILES {
        println!("Error: Maximum files reached");
        return;
    }
    for f in &state.files {
        if f.filename == args[0] {
            println!("Error: File '{}' already exists", args[0]);
            return;
        }
    }
    let owner = state.users[state.current_user.unwrap()].name.clone();
    let content = if args.len() >= 2 { args[1].clone() } else { String::new() };
    state.files.push(FileT {
        filename: args[0].clone(),
        content,
        owner,
        permissions: 755,
    });
    println!("File '{}' created", args[0]);
}

fn cmd_readfile(state: &State, args: &[String]) {
    if args.is_empty() {
        println!("Usage: readfile <filename>");
        return;
    }
    for f in &state.files {
        if f.filename == args[0] {
            println!("=== {} ===", f.filename);
            println!("Owner: {}", f.owner);
            println!("Permissions: {}", f.permissions);
            println!("Content: {}", f.content);
            return;
        }
    }
    println!("Error: File '{}' not found", args[0]);
}

fn cmd_writefile(state: &mut State, args: &[String]) {
    if !is_logged_in(state) {
        println!("Error: Must be logged in");
        return;
    }
    if args.len() < 2 {
        println!("Usage: writefile <filename> <content>");
        return;
    }
    let user_idx = state.current_user.unwrap();
    let user_name = state.users[user_idx].name.clone();
    let user_perm = state.users[user_idx].permission_level;
    for i in 0..state.files.len() {
        if state.files[i].filename == args[0] {
            if state.files[i].owner == user_name || user_perm >= 5 {
                state.files[i].content = args[1].clone();
                println!("File '{}' updated", args[0]);
                return;
            } else {
                println!("Error: Permission denied");
                return;
            }
        }
    }
    println!("Error: File '{}' not found", args[0]);
}

fn cmd_deletefile(state: &mut State, args: &[String]) {
    if !is_logged_in(state) {
        println!("Error: Must be logged in");
        return;
    }
    if args.is_empty() {
        println!("Usage: deletefile <filename>");
        return;
    }
    let user_idx = state.current_user.unwrap();
    let user_name = state.users[user_idx].name.clone();
    let user_perm = state.users[user_idx].permission_level;
    for i in 0..state.files.len() {
        if state.files[i].filename == args[0] {
            if state.files[i].owner == user_name || user_perm >= 9 {
                state.files.remove(i);
                println!("File '{}' deleted", args[0]);
                return;
            } else {
                println!("Error: Permission denied");
                return;
            }
        }
    }
    println!("Error: File '{}' not found", args[0]);
}

fn cmd_listfiles(state: &State) {
    if state.files.is_empty() {
        println!("No files");
        return;
    }
    println!("Files:");
    for f in &state.files {
        println!("  {} (owner: {}, perm: {})", f.filename, f.owner, f.permissions);
    }
}

// Variable commands
fn cmd_set(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: set <name> <value>");
        return;
    }
    for v in state.variables.iter_mut() {
        if v.name == args[0] {
            v.value = args[1].clone();
            println!("Variable '{}' updated", args[0]);
            return;
        }
    }
    if state.variables.len() >= MAX_VARIABLES {
        println!("Error: Maximum variables reached");
        return;
    }
    state.variables.push(Variable {
        name: args[0].clone(),
        value: args[1].clone(),
    });
    println!("Variable '{}' set", args[0]);
}

fn cmd_get(state: &State, args: &[String]) {
    if args.is_empty() {
        println!("Usage: get <name>");
        return;
    }
    for v in &state.variables {
        if v.name == args[0] {
            println!("{} = {}", v.name, v.value);
            return;
        }
    }
    println!("Error: Variable '{}' not found", args[0]);
}

fn cmd_unset(state: &mut State, args: &[String]) {
    if args.is_empty() {
        println!("Usage: unset <name>");
        return;
    }
    for i in 0..state.variables.len() {
        if state.variables[i].name == args[0] {
            state.variables.remove(i);
            println!("Variable '{}' unset", args[0]);
            return;
        }
    }
    println!("Error: Variable '{}' not found", args[0]);
}

fn cmd_listvars(state: &State) {
    if state.variables.is_empty() {
        println!("No variables set");
        return;
    }
    println!("Variables:");
    for v in &state.variables {
        println!("  {} = {}", v.name, v.value);
    }
}

// Mimic C's strcmp signed-byte comparison semantics, returning a negative, zero, or positive int.
fn c_strcmp(a: &str, b: &str) -> i32 {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let n = ab.len().min(bb.len());
    for i in 0..n {
        if ab[i] != bb[i] {
            // C's strcmp on most platforms compares as unsigned char; the "result" is the
            // difference of unsigned-char values (positive or negative int).
            return (ab[i] as i32) - (bb[i] as i32);
        }
    }
    (ab.len() as i32) - (bb.len() as i32)
}

fn c_strncmp(a: &str, b: &str, n: usize) -> i32 {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let take_a = ab.len().min(n);
    let take_b = bb.len().min(n);
    let cmp_n = take_a.min(take_b);
    for i in 0..cmp_n {
        if ab[i] != bb[i] {
            return (ab[i] as i32) - (bb[i] as i32);
        }
    }
    if take_a == take_b {
        0
    } else {
        (take_a as i32) - (take_b as i32)
    }
}

fn cmd_compare(args: &[String]) {
    if args.len() < 2 {
        println!("Usage: compare <string1> <string2>");
        return;
    }
    let result = c_strcmp(&args[0], &args[1]);
    println!("strcmp('{}', '{}') = {}", args[0], args[1], result);
    if result == 0 {
        println!("Strings are equal");
    } else if result < 0 {
        println!("'{}' < '{}'", args[0], args[1]);
    } else {
        println!("'{}' > '{}'", args[0], args[1]);
    }
}

fn cmd_compare_n(args: &[String]) {
    if args.len() < 3 {
        println!("Usage: compareN <string1> <string2> <n>");
        return;
    }
    let n = c_atoi(&args[2]);
    // C: int n = atoi(args[2]); strncmp(a, b, n) — passing the int promotes to size_t.
    // A negative int sign-extends to a huge size_t, so strncmp ends up comparing
    // the full strings. Mirror that behavior here.
    let n_us: usize = if n < 0 {
        // Sign-extend on platforms where size_t is 64-bit (Linux x86_64); use as i64 then
        // cast through usize bit pattern.
        n as isize as usize
    } else {
        n as usize
    };
    let result = c_strncmp(&args[0], &args[1], n_us);
    println!("strncmp('{}', '{}', {}) = {}", args[0], args[1], n, result);
    if result == 0 {
        println!("First {} characters are equal", n);
    } else if result < 0 {
        println!("'{}' < '{}' (first {} chars)", args[0], args[1], n);
    } else {
        println!("'{}' > '{}' (first {} chars)", args[0], args[1], n);
    }
}

fn cmd_startswith(args: &[String]) {
    if args.len() < 2 {
        println!("Usage: startswith <string> <prefix>");
        return;
    }
    let prefix_len = args[1].as_bytes().len();
    if c_strncmp(&args[0], &args[1], prefix_len) == 0 {
        println!("'{}' starts with '{}'", args[0], args[1]);
    } else {
        println!("'{}' does not start with '{}'", args[0], args[1]);
    }
}

fn cmd_match(args: &[String]) {
    if args.len() < 2 {
        println!("Usage: match <pattern> <string1> [string2] ...");
        return;
    }
    println!("Matching pattern '{}':", args[0]);
    let mut matches = 0i32;
    for i in 1..args.len() {
        if args[0] == args[i] {
            println!("  '{}' - EXACT MATCH", args[i]);
            matches += 1;
        } else if args[i].contains(args[0].as_str()) {
            println!("  '{}' - contains pattern", args[i]);
            matches += 1;
        } else {
            println!("  '{}' - no match", args[i]);
        }
    }
    println!("Total matches: {}", matches);
}

fn cmd_help() {
    println!();
    println!("=== Command Interpreter Help ===");
    println!("User Management:");
    println!("  adduser <user> <pass> [level] - Add new user");
    println!("  login <user> <pass>            - Login as user");
    println!("  logout                         - Logout current user");
    println!("  whoami                         - Show current user");
    println!("  listusers                      - List all users");
    println!();
    println!("File Management:");
    println!("  createfile <name> [content]    - Create file");
    println!("  readfile <name>                - Read file");
    println!("  writefile <name> <content>     - Write to file");
    println!("  deletefile <name>              - Delete file");
    println!("  listfiles                      - List all files");
    println!();
    println!("Variable Management:");
    println!("  set <name> <value>             - Set variable");
    println!("  get <name>                     - Get variable");
    println!("  unset <name>                   - Unset variable");
    println!("  listvars                       - List all variables");
    println!();
    println!("String Operations:");
    println!("  compare <str1> <str2>          - Compare strings");
    println!("  compareN <str1> <str2> <n>     - Compare first N chars");
    println!("  startswith <str> <prefix>      - Check if starts with");
    println!("  match <pattern> <str> ...      - Match pattern");
    println!();
    println!("System:");
    println!("  debug [on|off]                 - Toggle debug mode");
    println!("  verbose [on|off]               - Toggle verbose mode");
    println!("  status                         - Show system status");
    println!("  time                           - Show current time");
    println!("  help                           - Show this help");
    println!("  exit                           - Exit program");
}

fn cmd_debug(state: &mut State, args: &[String]) {
    if args.is_empty() {
        println!("Debug mode: {}", if state.debug_mode { "ON" } else { "OFF" });
        return;
    }
    if args[0] == "on" {
        state.debug_mode = true;
        println!("Debug mode enabled");
    } else if args[0] == "off" {
        state.debug_mode = false;
        println!("Debug mode disabled");
    } else {
        println!("Usage: debug [on|off]");
    }
}

fn cmd_verbose(state: &mut State, args: &[String]) {
    if args.is_empty() {
        println!("Verbose mode: {}", if state.verbose_mode { "ON" } else { "OFF" });
        return;
    }
    if args[0] == "on" {
        state.verbose_mode = true;
        println!("Verbose mode enabled");
    } else if args[0] == "off" {
        state.verbose_mode = false;
        println!("Verbose mode disabled");
    } else {
        println!("Usage: verbose [on|off]");
    }
}

fn cmd_status(state: &State) {
    let current = match state.current_user {
        Some(idx) if state.users[idx].logged_in => state.users[idx].name.clone(),
        _ => "none".to_string(),
    };
    println!();
    println!("=== System Status ===");
    println!("Users: {}/{}", state.users.len(), MAX_USERS);
    println!("Files: {}/{}", state.files.len(), MAX_FILES);
    println!("Variables: {}/{}", state.variables.len(), MAX_VARIABLES);
    println!("Current user: {}", current);
    println!("Debug mode: {}", if state.debug_mode { "ON" } else { "OFF" });
    println!("Verbose mode: {}", if state.verbose_mode { "ON" } else { "OFF" });
}

// Use libc's time/ctime to match the C output exactly (including trailing newline from ctime).
extern "C" {
    fn ctime(timep: *const libc::time_t) -> *const libc::c_char;
}

fn cmd_time() {
    unsafe {
        let now: libc::time_t = libc::time(ptr::null_mut());
        let s_ptr = ctime(&now);
        if s_ptr.is_null() {
            // Should not happen for a valid time_t.
            print!("Current time: ");
            return;
        }
        let cstr = CStr::from_ptr(s_ptr);
        // ctime always ends with '\n'; printf("Current time: %s", ctime(&now)) writes raw bytes.
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"Current time: ").ok();
        handle.write_all(cstr.to_bytes()).ok();
        handle.flush().ok();
    }
}

fn process_command(state: &mut State, input: &str) {
    let (command, args) = parse_command(input);
    if command.is_empty() {
        return;
    }
    if state.debug_mode {
        println!("[DEBUG] Command: '{}', Args: {}", command, args.len());
    }

    match command.as_str() {
        "adduser" => cmd_adduser(state, &args),
        "login" => cmd_login(state, &args),
        "logout" => cmd_logout(state),
        "whoami" => cmd_whoami(state),
        "listusers" | "users" => cmd_listusers(state),
        "createfile" | "touch" => cmd_createfile(state, &args),
        "readfile" | "cat" => cmd_readfile(state, &args),
        "writefile" | "write" => cmd_writefile(state, &args),
        "deletefile" | "rm" => cmd_deletefile(state, &args),
        "listfiles" | "ls" => cmd_listfiles(state),
        "set" => cmd_set(state, &args),
        "get" => cmd_get(state, &args),
        "unset" => cmd_unset(state, &args),
        "listvars" | "vars" => cmd_listvars(state),
        "compare" | "cmp" => cmd_compare(&args),
        "compareN" | "cmpn" => cmd_compare_n(&args),
        "startswith" => cmd_startswith(&args),
        "match" => cmd_match(&args),
        "debug" => cmd_debug(state, &args),
        "verbose" => cmd_verbose(state, &args),
        "status" => cmd_status(state),
        "time" => cmd_time(),
        "help" | "?" => cmd_help(),
        "exit" | "quit" => {
            println!("Goodbye!");
            // Flush stdout before exit so all buffered data is emitted.
            io::stdout().flush().ok();
            exit(0);
        }
        _ => {
            // Partial-match suggestions (preserve C's order of strncmp checks).
            let cmd_b = command.as_bytes();
            if cmd_b.len() >= 3 && c_strncmp(&command, "add", 3) == 0 {
                println!("Did you mean 'adduser'?");
            } else if cmd_b.len() >= 3 && c_strncmp(&command, "log", 3) == 0 {
                println!("Did you mean 'login' or 'logout'?");
            } else if cmd_b.len() >= 4 && c_strncmp(&command, "list", 4) == 0 {
                println!("Did you mean 'listusers', 'listfiles', or 'listvars'?");
            } else if cmd_b.len() >= 6 && c_strncmp(&command, "create", 6) == 0 {
                println!("Did you mean 'createfile'?");
            } else if cmd_b.len() >= 4 && c_strncmp(&command, "read", 4) == 0 {
                println!("Did you mean 'readfile'?");
            } else if cmd_b.len() >= 5 && c_strncmp(&command, "write", 5) == 0 {
                println!("Did you mean 'writefile'?");
            } else if cmd_b.len() >= 6 && c_strncmp(&command, "delete", 6) == 0 {
                println!("Did you mean 'deletefile'?");
            } else {
                println!("Unknown command: '{}'. Type 'help' for available commands.", command);
            }
        }
    }
}

fn main() {
    println!("|----------------------------------------|");
    println!("|   COMMAND INTERPRETER                  |");
    println!("|   strcmp/strncmp demonstration         |");
    println!("|----------------------------------------|");
    println!("Type 'help' for available commands");
    println!();

    let mut state = State::new();
    let mut stdin = io::stdin();
    let mut buf: Vec<u8> = Vec::with_capacity(MAX_INPUT);

    loop {
        // Print prompt without newline.
        {
            let stdout = io::stdout();
            let mut h = stdout.lock();
            h.write_all(b"> ").ok();
            h.flush().ok();
        }

        if fgets(&mut stdin, &mut buf, MAX_INPUT).is_none() {
            break;
        }

        // Build a string from the buffer; treat as bytes for newline stripping.
        let mut bytes = buf.clone();
        if let Some(pos) = bytes.iter().position(|&b| b == b'\n') {
            bytes.truncate(pos);
        }
        let input = String::from_utf8_lossy(&bytes).into_owned();

        if state.verbose_mode {
            println!("[VERBOSE] Processing: '{}'", input);
        }

        process_command(&mut state, &input);
    }
}
