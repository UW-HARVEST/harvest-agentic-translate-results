// Translation of c_src/src/main.c to Rust.
// Aims for byte-identical output to the original C program.

use std::io::{Read, Write};

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
    logged_in: i32,
}

#[derive(Clone, Default)]
struct File {
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
    current_user_idx: Option<usize>,
    files: Vec<File>,
    variables: Vec<Variable>,
    debug_mode: i32,
    verbose_mode: i32,
}

impl State {
    fn new() -> Self {
        Self {
            users: Vec::new(),
            current_user_idx: None,
            files: Vec::new(),
            variables: Vec::new(),
            debug_mode: 0,
            verbose_mode: 0,
        }
    }

    fn current_user(&self) -> Option<&User> {
        self.current_user_idx.and_then(|i| self.users.get(i))
    }
}

// Truncate a string to n-1 characters (mimicking strncpy with explicit null termination)
fn truncate_str(s: &str, n: usize) -> String {
    if n == 0 {
        return String::new();
    }
    let max_bytes = n - 1;
    if s.len() <= max_bytes {
        s.to_string()
    } else {
        // Truncate at byte boundary; original C is not Unicode-aware, but
        // the input here is treated as raw bytes by C. We treat it the same.
        // To be safe, we truncate by bytes.
        let bytes = s.as_bytes();
        let truncated = &bytes[..max_bytes];
        // Convert back to String, replacing any invalid utf8 just in case
        String::from_utf8_lossy(truncated).into_owned()
    }
}

// strcmp-like comparison: returns negative, zero or positive
fn strcmp(a: &str, b: &str) -> i32 {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let min = ab.len().min(bb.len());
    for i in 0..min {
        if ab[i] != bb[i] {
            return (ab[i] as i32) - (bb[i] as i32);
        }
    }
    (ab.len() as i32) - (bb.len() as i32)
}

// strncmp-like comparison limited to first n bytes
fn strncmp(a: &str, b: &str, n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    let n = n as usize;
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    for i in 0..n {
        let ac = if i < ab.len() { ab[i] } else { 0 };
        let bc = if i < bb.len() { bb[i] } else { 0 };
        if ac != bc {
            return (ac as i32) - (bc as i32);
        }
        if ac == 0 {
            return 0;
        }
    }
    0
}

// Mimics atoi: parses leading digits (with optional + or -) and returns 0 on failure.
fn atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip whitespace (atoi spec)
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n'
        || bytes[i] == b'\r' || bytes[i] == 0x0b || bytes[i] == 0x0c) {
        i += 1;
    }
    let mut sign: i64 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        result = result * 10 + (bytes[i] - b'0') as i64;
        // saturate to i32 range like real atoi (UB on overflow, just clamp)
        if result > i32::MAX as i64 + 1 {
            // continue parsing but we'll just clamp
            result = i32::MAX as i64 + 1;
        }
        i += 1;
    }
    let signed = sign * result;
    if signed > i32::MAX as i64 {
        i32::MAX
    } else if signed < i32::MIN as i64 {
        i32::MIN
    } else {
        signed as i32
    }
}

// strstr-like check: does haystack contain needle (returns true if found, or true if needle is empty)
fn strstr_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.as_bytes().windows(needle.len()).any(|w| w == needle.as_bytes())
}

fn parse_command(input: &str) -> (String, Vec<String>) {
    // mimic strncpy with MAX_INPUT, then strtok with " \t"
    let temp = truncate_str(input, MAX_INPUT);
    let mut tokens = temp.split(|c: char| c == ' ' || c == '\t').filter(|s| !s.is_empty());
    let cmd = match tokens.next() {
        Some(t) => truncate_str(t, MAX_COMMAND),
        None => String::new(),
    };
    let mut args: Vec<String> = Vec::new();
    while args.len() < MAX_ARGS {
        match tokens.next() {
            Some(t) => args.push(truncate_str(t, MAX_COMMAND)),
            None => break,
        }
    }
    (cmd, args)
}

fn p(s: &str) {
    print!("{}", s);
}

// === User management ===
fn cmd_adduser(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        p("Usage: adduser <username> <password> [permission_level]\n");
        return;
    }
    if state.users.len() >= MAX_USERS {
        p("Error: Maximum users reached\n");
        return;
    }
    for u in &state.users {
        if strcmp(&u.name, &args[0]) == 0 {
            print!("Error: User '{}' already exists\n", args[0]);
            return;
        }
    }
    let permission_level = if args.len() >= 3 { atoi(&args[2]) } else { 1 };
    let user = User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level,
        logged_in: 0,
    };
    state.users.push(user);
    print!(
        "User '{}' added with permission level {}\n",
        args[0], permission_level
    );
}

fn cmd_login(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        p("Usage: login <username> <password>\n");
        return;
    }
    if let Some(idx) = state.current_user_idx {
        if state.users[idx].logged_in != 0 {
            print!(
                "Error: User '{}' already logged in. Use 'logout' first.\n",
                state.users[idx].name
            );
            return;
        }
    }
    for i in 0..state.users.len() {
        if strcmp(&state.users[i].name, &args[0]) == 0 {
            if strcmp(&state.users[i].password, &args[1]) == 0 {
                state.users[i].logged_in = 1;
                state.current_user_idx = Some(i);
                print!("Login successful. Welcome, {}!\n", state.users[i].name);
                return;
            } else {
                p("Error: Incorrect password\n");
                return;
            }
        }
    }
    p("Error: User not found\n");
}

fn cmd_logout(state: &mut State) {
    let logged_in = match state.current_user_idx {
        Some(i) => state.users[i].logged_in != 0,
        None => false,
    };
    if !logged_in {
        p("Error: No user logged in\n");
        return;
    }
    let i = state.current_user_idx.unwrap();
    print!("Goodbye, {}!\n", state.users[i].name);
    state.users[i].logged_in = 0;
    state.current_user_idx = None;
}

fn cmd_whoami(state: &State) {
    let logged_in = match state.current_user_idx {
        Some(i) => state.users[i].logged_in != 0,
        None => false,
    };
    if !logged_in {
        p("Not logged in\n");
        return;
    }
    let u = state.current_user().unwrap();
    print!("Current user: {}\n", u.name);
    print!("Permission level: {}\n", u.permission_level);
}

fn cmd_listusers(state: &State) {
    if state.users.is_empty() {
        p("No users registered\n");
        return;
    }
    p("Registered users:\n");
    for u in &state.users {
        let marker = if u.logged_in != 0 { "[logged in]" } else { "" };
        print!("  {} (level {}) {}\n", u.name, u.permission_level, marker);
    }
}

// === File management ===
fn cmd_createfile(state: &mut State, args: &[String]) {
    let logged_in = match state.current_user_idx {
        Some(i) => state.users[i].logged_in != 0,
        None => false,
    };
    if !logged_in {
        p("Error: Must be logged in\n");
        return;
    }
    if args.is_empty() {
        p("Usage: createfile <filename> [content]\n");
        return;
    }
    if state.files.len() >= MAX_FILES {
        p("Error: Maximum files reached\n");
        return;
    }
    for f in &state.files {
        if strcmp(&f.filename, &args[0]) == 0 {
            print!("Error: File '{}' already exists\n", args[0]);
            return;
        }
    }
    let owner = state.users[state.current_user_idx.unwrap()].name.clone();
    let content = if args.len() >= 2 { args[1].clone() } else { String::new() };
    state.files.push(File {
        filename: args[0].clone(),
        content,
        owner,
        permissions: 755,
    });
    print!("File '{}' created\n", args[0]);
}

fn cmd_readfile(state: &State, args: &[String]) {
    if args.is_empty() {
        p("Usage: readfile <filename>\n");
        return;
    }
    for f in &state.files {
        if strcmp(&f.filename, &args[0]) == 0 {
            print!("=== {} ===\n", f.filename);
            print!("Owner: {}\n", f.owner);
            print!("Permissions: {}\n", f.permissions);
            print!("Content: {}\n", f.content);
            return;
        }
    }
    print!("Error: File '{}' not found\n", args[0]);
}

fn cmd_writefile(state: &mut State, args: &[String]) {
    let cur_idx = match state.current_user_idx {
        Some(i) if state.users[i].logged_in != 0 => Some(i),
        _ => None,
    };
    if cur_idx.is_none() {
        p("Error: Must be logged in\n");
        return;
    }
    if args.len() < 2 {
        p("Usage: writefile <filename> <content>\n");
        return;
    }
    let cur_idx = cur_idx.unwrap();
    let cur_name = state.users[cur_idx].name.clone();
    let cur_level = state.users[cur_idx].permission_level;
    for f in state.files.iter_mut() {
        if strcmp(&f.filename, &args[0]) == 0 {
            if strcmp(&f.owner, &cur_name) == 0 || cur_level >= 5 {
                f.content = args[1].clone();
                print!("File '{}' updated\n", args[0]);
                return;
            } else {
                p("Error: Permission denied\n");
                return;
            }
        }
    }
    print!("Error: File '{}' not found\n", args[0]);
}

fn cmd_deletefile(state: &mut State, args: &[String]) {
    let cur_idx = match state.current_user_idx {
        Some(i) if state.users[i].logged_in != 0 => Some(i),
        _ => None,
    };
    if cur_idx.is_none() {
        p("Error: Must be logged in\n");
        return;
    }
    if args.is_empty() {
        p("Usage: deletefile <filename>\n");
        return;
    }
    let cur_idx = cur_idx.unwrap();
    let cur_name = state.users[cur_idx].name.clone();
    let cur_level = state.users[cur_idx].permission_level;
    let mut found_idx: Option<usize> = None;
    for (i, f) in state.files.iter().enumerate() {
        if strcmp(&f.filename, &args[0]) == 0 {
            found_idx = Some(i);
            break;
        }
    }
    if let Some(i) = found_idx {
        let f = &state.files[i];
        if strcmp(&f.owner, &cur_name) == 0 || cur_level >= 9 {
            state.files.remove(i);
            print!("File '{}' deleted\n", args[0]);
        } else {
            p("Error: Permission denied\n");
        }
        return;
    }
    print!("Error: File '{}' not found\n", args[0]);
}

fn cmd_listfiles(state: &State) {
    if state.files.is_empty() {
        p("No files\n");
        return;
    }
    p("Files:\n");
    for f in &state.files {
        print!("  {} (owner: {}, perm: {})\n", f.filename, f.owner, f.permissions);
    }
}

// === Variable commands ===
fn cmd_set(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        p("Usage: set <name> <value>\n");
        return;
    }
    for v in state.variables.iter_mut() {
        if strcmp(&v.name, &args[0]) == 0 {
            v.value = args[1].clone();
            print!("Variable '{}' updated\n", args[0]);
            return;
        }
    }
    if state.variables.len() >= MAX_VARIABLES {
        p("Error: Maximum variables reached\n");
        return;
    }
    state.variables.push(Variable {
        name: args[0].clone(),
        value: args[1].clone(),
    });
    print!("Variable '{}' set\n", args[0]);
}

fn cmd_get(state: &State, args: &[String]) {
    if args.is_empty() {
        p("Usage: get <name>\n");
        return;
    }
    for v in &state.variables {
        if strcmp(&v.name, &args[0]) == 0 {
            print!("{} = {}\n", v.name, v.value);
            return;
        }
    }
    print!("Error: Variable '{}' not found\n", args[0]);
}

fn cmd_unset(state: &mut State, args: &[String]) {
    if args.is_empty() {
        p("Usage: unset <name>\n");
        return;
    }
    let mut found: Option<usize> = None;
    for (i, v) in state.variables.iter().enumerate() {
        if strcmp(&v.name, &args[0]) == 0 {
            found = Some(i);
            break;
        }
    }
    if let Some(i) = found {
        state.variables.remove(i);
        print!("Variable '{}' unset\n", args[0]);
        return;
    }
    print!("Error: Variable '{}' not found\n", args[0]);
}

fn cmd_listvars(state: &State) {
    if state.variables.is_empty() {
        p("No variables set\n");
        return;
    }
    p("Variables:\n");
    for v in &state.variables {
        print!("  {} = {}\n", v.name, v.value);
    }
}

// === String commands ===
fn cmd_compare(args: &[String]) {
    if args.len() < 2 {
        p("Usage: compare <string1> <string2>\n");
        return;
    }
    let result = strcmp(&args[0], &args[1]);
    print!("strcmp('{}', '{}') = {}\n", args[0], args[1], result);
    if result == 0 {
        p("Strings are equal\n");
    } else if result < 0 {
        print!("'{}' < '{}'\n", args[0], args[1]);
    } else {
        print!("'{}' > '{}'\n", args[0], args[1]);
    }
}

fn cmd_compare_n(args: &[String]) {
    if args.len() < 3 {
        p("Usage: compareN <string1> <string2> <n>\n");
        return;
    }
    let n = atoi(&args[2]);
    let result = strncmp(&args[0], &args[1], n);
    print!("strncmp('{}', '{}', {}) = {}\n", args[0], args[1], n, result);
    if result == 0 {
        print!("First {} characters are equal\n", n);
    } else if result < 0 {
        print!("'{}' < '{}' (first {} chars)\n", args[0], args[1], n);
    } else {
        print!("'{}' > '{}' (first {} chars)\n", args[0], args[1], n);
    }
}

fn cmd_startswith(args: &[String]) {
    if args.len() < 2 {
        p("Usage: startswith <string> <prefix>\n");
        return;
    }
    let prefix_len = args[1].as_bytes().len() as i32;
    if strncmp(&args[0], &args[1], prefix_len) == 0 {
        print!("'{}' starts with '{}'\n", args[0], args[1]);
    } else {
        print!("'{}' does not start with '{}'\n", args[0], args[1]);
    }
}

fn cmd_match(args: &[String]) {
    if args.len() < 2 {
        p("Usage: match <pattern> <string1> [string2] ...\n");
        return;
    }
    print!("Matching pattern '{}':\n", args[0]);
    let mut matches = 0;
    for i in 1..args.len() {
        if strcmp(&args[0], &args[i]) == 0 {
            print!("  '{}' - EXACT MATCH\n", args[i]);
            matches += 1;
        } else if strstr_contains(&args[i], &args[0]) {
            print!("  '{}' - contains pattern\n", args[i]);
            matches += 1;
        } else {
            print!("  '{}' - no match\n", args[i]);
        }
    }
    print!("Total matches: {}\n", matches);
}

// === System commands ===
fn cmd_help() {
    p("\n=== Command Interpreter Help ===\n");
    p("User Management:\n");
    p("  adduser <user> <pass> [level] - Add new user\n");
    p("  login <user> <pass>            - Login as user\n");
    p("  logout                         - Logout current user\n");
    p("  whoami                         - Show current user\n");
    p("  listusers                      - List all users\n");
    p("\nFile Management:\n");
    p("  createfile <name> [content]    - Create file\n");
    p("  readfile <name>                - Read file\n");
    p("  writefile <name> <content>     - Write to file\n");
    p("  deletefile <name>              - Delete file\n");
    p("  listfiles                      - List all files\n");
    p("\nVariable Management:\n");
    p("  set <name> <value>             - Set variable\n");
    p("  get <name>                     - Get variable\n");
    p("  unset <name>                   - Unset variable\n");
    p("  listvars                       - List all variables\n");
    p("\nString Operations:\n");
    p("  compare <str1> <str2>          - Compare strings\n");
    p("  compareN <str1> <str2> <n>     - Compare first N chars\n");
    p("  startswith <str> <prefix>      - Check if starts with\n");
    p("  match <pattern> <str> ...      - Match pattern\n");
    p("\nSystem:\n");
    p("  debug [on|off]                 - Toggle debug mode\n");
    p("  verbose [on|off]               - Toggle verbose mode\n");
    p("  status                         - Show system status\n");
    p("  time                           - Show current time\n");
    p("  help                           - Show this help\n");
    p("  exit                           - Exit program\n");
}

fn cmd_debug(state: &mut State, args: &[String]) {
    if args.is_empty() {
        print!("Debug mode: {}\n", if state.debug_mode != 0 { "ON" } else { "OFF" });
        return;
    }
    if strcmp(&args[0], "on") == 0 {
        state.debug_mode = 1;
        p("Debug mode enabled\n");
    } else if strcmp(&args[0], "off") == 0 {
        state.debug_mode = 0;
        p("Debug mode disabled\n");
    } else {
        p("Usage: debug [on|off]\n");
    }
}

fn cmd_verbose(state: &mut State, args: &[String]) {
    if args.is_empty() {
        print!("Verbose mode: {}\n", if state.verbose_mode != 0 { "ON" } else { "OFF" });
        return;
    }
    if strcmp(&args[0], "on") == 0 {
        state.verbose_mode = 1;
        p("Verbose mode enabled\n");
    } else if strcmp(&args[0], "off") == 0 {
        state.verbose_mode = 0;
        p("Verbose mode disabled\n");
    } else {
        p("Usage: verbose [on|off]\n");
    }
}

fn cmd_status(state: &State) {
    p("\n=== System Status ===\n");
    print!("Users: {}/{}\n", state.users.len(), MAX_USERS);
    print!("Files: {}/{}\n", state.files.len(), MAX_FILES);
    print!("Variables: {}/{}\n", state.variables.len(), MAX_VARIABLES);
    let cur_name = match state.current_user_idx {
        Some(i) if state.users[i].logged_in != 0 => state.users[i].name.clone(),
        _ => "none".to_string(),
    };
    print!("Current user: {}\n", cur_name);
    print!("Debug mode: {}\n", if state.debug_mode != 0 { "ON" } else { "OFF" });
    print!("Verbose mode: {}\n", if state.verbose_mode != 0 { "ON" } else { "OFF" });
}

extern "C" {
    fn ctime(timep: *const libc::time_t) -> *const libc::c_char;
}

fn cmd_time() {
    // Use libc to match exactly the C runtime's ctime() output
    unsafe {
        let now: libc::time_t = libc::time(std::ptr::null_mut());
        let cstr = ctime(&now);
        if cstr.is_null() {
            return;
        }
        // Read as C string
        let mut len = 0usize;
        while *cstr.add(len) != 0 {
            len += 1;
        }
        let bytes = std::slice::from_raw_parts(cstr as *const u8, len);
        let mut out = std::io::stdout().lock();
        out.write_all(b"Current time: ").ok();
        out.write_all(bytes).ok();
    }
}

fn process_command(state: &mut State, input: &str) {
    let (command, args) = parse_command(input);
    if command.is_empty() {
        return;
    }
    if state.debug_mode != 0 {
        print!("[DEBUG] Command: '{}', Args: {}\n", command, args.len());
    }

    if strcmp(&command, "adduser") == 0 {
        cmd_adduser(state, &args);
    } else if strcmp(&command, "login") == 0 {
        cmd_login(state, &args);
    } else if strcmp(&command, "logout") == 0 {
        cmd_logout(state);
    } else if strcmp(&command, "whoami") == 0 {
        cmd_whoami(state);
    } else if strcmp(&command, "listusers") == 0 || strcmp(&command, "users") == 0 {
        cmd_listusers(state);
    }
    // File commands
    else if strcmp(&command, "createfile") == 0 || strcmp(&command, "touch") == 0 {
        cmd_createfile(state, &args);
    } else if strcmp(&command, "readfile") == 0 || strcmp(&command, "cat") == 0 {
        cmd_readfile(state, &args);
    } else if strcmp(&command, "writefile") == 0 || strcmp(&command, "write") == 0 {
        cmd_writefile(state, &args);
    } else if strcmp(&command, "deletefile") == 0 || strcmp(&command, "rm") == 0 {
        cmd_deletefile(state, &args);
    } else if strcmp(&command, "listfiles") == 0 || strcmp(&command, "ls") == 0 {
        cmd_listfiles(state);
    }
    // Variable commands
    else if strcmp(&command, "set") == 0 {
        cmd_set(state, &args);
    } else if strcmp(&command, "get") == 0 {
        cmd_get(state, &args);
    } else if strcmp(&command, "unset") == 0 {
        cmd_unset(state, &args);
    } else if strcmp(&command, "listvars") == 0 || strcmp(&command, "vars") == 0 {
        cmd_listvars(state);
    }
    // String comparison commands
    else if strcmp(&command, "compare") == 0 || strcmp(&command, "cmp") == 0 {
        cmd_compare(&args);
    } else if strcmp(&command, "compareN") == 0 || strcmp(&command, "cmpn") == 0 {
        cmd_compare_n(&args);
    } else if strcmp(&command, "startswith") == 0 {
        cmd_startswith(&args);
    } else if strcmp(&command, "match") == 0 {
        cmd_match(&args);
    }
    // System commands
    else if strcmp(&command, "debug") == 0 {
        cmd_debug(state, &args);
    } else if strcmp(&command, "verbose") == 0 {
        cmd_verbose(state, &args);
    } else if strcmp(&command, "status") == 0 {
        cmd_status(state);
    } else if strcmp(&command, "time") == 0 {
        cmd_time();
    } else if strcmp(&command, "help") == 0 || strcmp(&command, "?") == 0 {
        cmd_help();
    } else if strcmp(&command, "exit") == 0 || strcmp(&command, "quit") == 0 {
        p("Goodbye!\n");
        // Flush stdout to match behavior at exit
        std::io::stdout().flush().ok();
        std::process::exit(0);
    }
    // Partial matches via strncmp
    else if strncmp(&command, "add", 3) == 0 {
        p("Did you mean 'adduser'?\n");
    } else if strncmp(&command, "log", 3) == 0 {
        p("Did you mean 'login' or 'logout'?\n");
    } else if strncmp(&command, "list", 4) == 0 {
        p("Did you mean 'listusers', 'listfiles', or 'listvars'?\n");
    } else if strncmp(&command, "create", 6) == 0 {
        p("Did you mean 'createfile'?\n");
    } else if strncmp(&command, "read", 4) == 0 {
        p("Did you mean 'readfile'?\n");
    } else if strncmp(&command, "write", 5) == 0 {
        p("Did you mean 'writefile'?\n");
    } else if strncmp(&command, "delete", 6) == 0 {
        p("Did you mean 'deletefile'?\n");
    } else {
        print!("Unknown command: '{}'. Type 'help' for available commands.\n", command);
    }
}

// fgets-equivalent: read up to max-1 bytes or until newline (inclusive), return None on EOF without reading any bytes
fn fgets_like<R: Read>(reader: &mut R, max: usize) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() + 1 < max {
        match reader.read(&mut byte) {
            Ok(0) => {
                if buf.is_empty() {
                    return None;
                }
                return Some(buf);
            }
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    return Some(buf);
                }
            }
            Err(_) => {
                if buf.is_empty() {
                    return None;
                }
                return Some(buf);
            }
        }
    }
    Some(buf)
}

fn main() {
    p("|----------------------------------------|\n");
    p("|   COMMAND INTERPRETER                  |\n");
    p("|   strcmp/strncmp demonstration         |\n");
    p("|----------------------------------------|\n");
    p("Type 'help' for available commands\n\n");

    let mut state = State::new();
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();

    loop {
        p("> ");
        // Match C: it does not call fflush before fgets. However, Rust's stdout
        // may be buffered differently. We don't flush to match C's piped-output
        // behavior, but when stdout is a TTY, line buffering applies. Either way,
        // the final byte sequence emitted should match C.
        let raw = match fgets_like(&mut handle, MAX_INPUT) {
            Some(v) => v,
            None => break,
        };
        // Remove first '\n' (and everything after it, like strcspn). C does:
        //   input[strcspn(input, "\n")] = 0;
        // which truncates at the first newline.
        let mut end = raw.len();
        for (i, &b) in raw.iter().enumerate() {
            if b == b'\n' {
                end = i;
                break;
            }
        }
        let input_bytes = &raw[..end];
        let input = String::from_utf8_lossy(input_bytes).into_owned();

        if state.verbose_mode != 0 {
            print!("[VERBOSE] Processing: '{}'\n", input);
        }

        process_command(&mut state, &input);
    }
}
