// Translated from C to Rust, preserving exact behavior and byte-identical output.

use std::ffi::CStr;
use std::io::{self, Read, Write};
use std::process;

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

#[derive(Clone)]
struct User {
    name: String,
    password: String,
    permission_level: i32,
    logged_in: bool,
}

#[derive(Clone)]
struct File {
    filename: String,
    content: String,
    owner: String,
    permissions: i32,
}

#[derive(Clone)]
struct Variable {
    name: String,
    value: String,
}

struct State {
    users: Vec<User>,
    current_user_idx: Option<usize>,
    files: Vec<File>,
    variables: Vec<Variable>,
    debug_mode: bool,
    verbose_mode: bool,
}

impl State {
    fn new() -> Self {
        State {
            users: Vec::new(),
            current_user_idx: None,
            files: Vec::new(),
            variables: Vec::new(),
            debug_mode: false,
            verbose_mode: false,
        }
    }

    fn current_user(&self) -> Option<&User> {
        self.current_user_idx.and_then(|i| self.users.get(i))
    }
}

// Truncate a token to mimic C's strncpy with MAX_COMMAND-1 length plus null-terminator.
fn truncate_token(s: &str) -> String {
    let bytes = s.as_bytes();
    let take = bytes.len().min(MAX_COMMAND - 1);
    // Try UTF-8 lossless; fall back to lossy for safety.
    match std::str::from_utf8(&bytes[..take]) {
        Ok(v) => v.to_string(),
        Err(_) => String::from_utf8_lossy(&bytes[..take]).into_owned(),
    }
}

// Parse command and arguments. Mirrors strtok semantics over " \t".
fn parse_command(input: &str) -> (String, Vec<String>) {
    // strncpy with MAX_INPUT - 1 then null terminator at MAX_INPUT - 1.
    // Effectively, only consider the first MAX_INPUT - 1 bytes.
    let bytes = input.as_bytes();
    let take = bytes.len().min(MAX_INPUT - 1);
    let temp = match std::str::from_utf8(&bytes[..take]) {
        Ok(v) => v.to_string(),
        Err(_) => String::from_utf8_lossy(&bytes[..take]).into_owned(),
    };

    let tokens: Vec<&str> = temp
        .split(|c: char| c == ' ' || c == '\t')
        .filter(|s| !s.is_empty())
        .collect();

    if tokens.is_empty() {
        return (String::new(), Vec::new());
    }

    let cmd = truncate_token(tokens[0]);

    let mut args = Vec::new();
    for token in tokens.iter().skip(1).take(MAX_ARGS) {
        args.push(truncate_token(token));
    }

    (cmd, args)
}

// fgets-like behavior: read up to max-1 bytes or until '\n' is encountered.
// Returns false on EOF when buffer is empty.
fn fgets_like<R: Read>(reader: &mut R, buf: &mut Vec<u8>, max: usize) -> bool {
    buf.clear();
    let mut byte = [0u8; 1];
    while buf.len() < max - 1 {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    !buf.is_empty()
}

// strcmp-like comparison returning negative, zero, or positive integer.
fn strcmp_like(a: &str, b: &str) -> i32 {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let n = ab.len().min(bb.len());
    for i in 0..n {
        if ab[i] != bb[i] {
            return ab[i] as i32 - bb[i] as i32;
        }
    }
    ab.len() as i32 - bb.len() as i32
}

// strncmp-like comparison
fn strncmp_like(a: &str, b: &str, n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    let n = n as usize;
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    for i in 0..n {
        let av = ab.get(i).copied();
        let bv = bb.get(i).copied();
        match (av, bv) {
            (Some(x), Some(y)) => {
                if x != y {
                    return x as i32 - y as i32;
                }
                if x == 0 {
                    return 0;
                }
            }
            (Some(x), None) => return x as i32,
            (None, Some(y)) => return -(y as i32),
            (None, None) => return 0,
        }
    }
    0
}

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
        if strcmp_like(&u.name, &args[0]) == 0 {
            println!("Error: User '{}' already exists", args[0]);
            return;
        }
    }

    let permission_level = if args.len() >= 3 {
        atoi(&args[2])
    } else {
        1
    };

    state.users.push(User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level,
        logged_in: false,
    });

    let last = state.users.last().unwrap();
    println!(
        "User '{}' added with permission level {}",
        args[0], last.permission_level
    );
}

fn cmd_login(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: login <username> <password>");
        return;
    }

    if let Some(idx) = state.current_user_idx {
        if state.users[idx].logged_in {
            println!(
                "Error: User '{}' already logged in. Use 'logout' first.",
                state.users[idx].name
            );
            return;
        }
    }

    for i in 0..state.users.len() {
        if strcmp_like(&state.users[i].name, &args[0]) == 0 {
            if strcmp_like(&state.users[i].password, &args[1]) == 0 {
                state.users[i].logged_in = true;
                state.current_user_idx = Some(i);
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
    let logged_in = match state.current_user_idx {
        Some(idx) => state.users[idx].logged_in,
        None => false,
    };
    if !logged_in {
        println!("Error: No user logged in");
        return;
    }
    let idx = state.current_user_idx.unwrap();
    println!("Goodbye, {}!", state.users[idx].name);
    state.users[idx].logged_in = false;
    state.current_user_idx = None;
}

fn cmd_whoami(state: &State) {
    let user = match state.current_user() {
        Some(u) if u.logged_in => u,
        _ => {
            println!("Not logged in");
            return;
        }
    };

    println!("Current user: {}", user.name);
    println!("Permission level: {}", user.permission_level);
}

fn cmd_listusers(state: &State) {
    if state.users.is_empty() {
        println!("No users registered");
        return;
    }

    println!("Registered users:");
    for u in &state.users {
        println!(
            "  {} (level {}) {}",
            u.name,
            u.permission_level,
            if u.logged_in { "[logged in]" } else { "" }
        );
    }
}

fn cmd_createfile(state: &mut State, args: &[String]) {
    let user = match state.current_user() {
        Some(u) if u.logged_in => u.clone(),
        _ => {
            println!("Error: Must be logged in");
            return;
        }
    };

    if args.is_empty() {
        println!("Usage: createfile <filename> [content]");
        return;
    }

    if state.files.len() >= MAX_FILES {
        println!("Error: Maximum files reached");
        return;
    }

    for f in &state.files {
        if strcmp_like(&f.filename, &args[0]) == 0 {
            println!("Error: File '{}' already exists", args[0]);
            return;
        }
    }

    let content = if args.len() >= 2 {
        args[1].clone()
    } else {
        String::new()
    };

    state.files.push(File {
        filename: args[0].clone(),
        content,
        owner: user.name.clone(),
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
        if strcmp_like(&f.filename, &args[0]) == 0 {
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
    let (uname, plevel) = match state.current_user() {
        Some(u) if u.logged_in => (u.name.clone(), u.permission_level),
        _ => {
            println!("Error: Must be logged in");
            return;
        }
    };

    if args.len() < 2 {
        println!("Usage: writefile <filename> <content>");
        return;
    }

    for i in 0..state.files.len() {
        if strcmp_like(&state.files[i].filename, &args[0]) == 0 {
            if strcmp_like(&state.files[i].owner, &uname) == 0 || plevel >= 5 {
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
    let (uname, plevel) = match state.current_user() {
        Some(u) if u.logged_in => (u.name.clone(), u.permission_level),
        _ => {
            println!("Error: Must be logged in");
            return;
        }
    };

    if args.is_empty() {
        println!("Usage: deletefile <filename>");
        return;
    }

    for i in 0..state.files.len() {
        if strcmp_like(&state.files[i].filename, &args[0]) == 0 {
            if strcmp_like(&state.files[i].owner, &uname) == 0 || plevel >= 9 {
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
        println!(
            "  {} (owner: {}, perm: {})",
            f.filename, f.owner, f.permissions
        );
    }
}

fn cmd_set(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: set <name> <value>");
        return;
    }

    for v in state.variables.iter_mut() {
        if strcmp_like(&v.name, &args[0]) == 0 {
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
        if strcmp_like(&v.name, &args[0]) == 0 {
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
        if strcmp_like(&state.variables[i].name, &args[0]) == 0 {
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

fn cmd_compare(args: &[String]) {
    if args.len() < 2 {
        println!("Usage: compare <string1> <string2>");
        return;
    }

    let result = strcmp_like(&args[0], &args[1]);
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

    let n = atoi(&args[2]);
    let result = strncmp_like(&args[0], &args[1], n);

    println!(
        "strncmp('{}', '{}', {}) = {}",
        args[0], args[1], n, result
    );

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

    let prefix_len = args[1].len();
    if strncmp_like(&args[0], &args[1], prefix_len as i32) == 0 {
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
    let mut matches = 0;

    for i in 1..args.len() {
        if strcmp_like(&args[0], &args[i]) == 0 {
            println!("  '{}' - EXACT MATCH", args[i]);
            matches += 1;
        } else if args[i].contains(args[0].as_str()) && !args[0].is_empty() {
            // strstr returns non-NULL when needle is found in haystack.
            // Note: strstr with empty needle returns haystack, i.e., non-NULL.
            // We handle empty needle below.
            println!("  '{}' - contains pattern", args[i]);
            matches += 1;
        } else if args[0].is_empty() {
            // Empty pattern: strstr returns haystack (non-NULL), so contains pattern.
            // But strcmp would also return 0 for two empty strings; if args[i] is empty,
            // first branch matches (EXACT MATCH). Otherwise, contains pattern.
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
        println!(
            "Debug mode: {}",
            if state.debug_mode { "ON" } else { "OFF" }
        );
        return;
    }

    if strcmp_like(&args[0], "on") == 0 {
        state.debug_mode = true;
        println!("Debug mode enabled");
    } else if strcmp_like(&args[0], "off") == 0 {
        state.debug_mode = false;
        println!("Debug mode disabled");
    } else {
        println!("Usage: debug [on|off]");
    }
}

fn cmd_verbose(state: &mut State, args: &[String]) {
    if args.is_empty() {
        println!(
            "Verbose mode: {}",
            if state.verbose_mode { "ON" } else { "OFF" }
        );
        return;
    }

    if strcmp_like(&args[0], "on") == 0 {
        state.verbose_mode = true;
        println!("Verbose mode enabled");
    } else if strcmp_like(&args[0], "off") == 0 {
        state.verbose_mode = false;
        println!("Verbose mode disabled");
    } else {
        println!("Usage: verbose [on|off]");
    }
}

fn cmd_status(state: &State) {
    println!();
    println!("=== System Status ===");
    println!("Users: {}/{}", state.users.len(), MAX_USERS);
    println!("Files: {}/{}", state.files.len(), MAX_FILES);
    println!("Variables: {}/{}", state.variables.len(), MAX_VARIABLES);
    let cur_name: String = match state.current_user() {
        Some(u) if u.logged_in => u.name.clone(),
        _ => "none".to_string(),
    };
    println!("Current user: {}", cur_name);
    println!(
        "Debug mode: {}",
        if state.debug_mode { "ON" } else { "OFF" }
    );
    println!(
        "Verbose mode: {}",
        if state.verbose_mode { "ON" } else { "OFF" }
    );
}

extern "C" {
    fn ctime(timep: *const libc::time_t) -> *const libc::c_char;
}

fn cmd_time() {
    // Match C: printf("Current time: %s", ctime(&now));
    // ctime() returns "Www Mmm dd hh:mm:ss yyyy\n"
    unsafe {
        let mut t: libc::time_t = 0;
        libc::time(&mut t as *mut _);
        let cstr_ptr = ctime(&t as *const _);
        if cstr_ptr.is_null() {
            print!("Current time: ");
            let _ = io::stdout().flush();
            return;
        }
        let cstr = CStr::from_ptr(cstr_ptr);
        let bytes = cstr.to_bytes();
        // Print "Current time: " followed by the ctime bytes (which include trailing '\n').
        let mut out = io::stdout();
        let _ = out.write_all(b"Current time: ");
        let _ = out.write_all(bytes);
        let _ = out.flush();
    }
}

// Mimic C's atoi: parse leading optional whitespace, optional sign, then digits.
fn atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len()
        && (bytes[i] == b' '
            || bytes[i] == b'\t'
            || bytes[i] == b'\n'
            || bytes[i] == b'\r'
            || bytes[i] == 0x0b
            || bytes[i] == 0x0c)
    {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        result = result.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    let result = (result * sign as i64) as i32;
    result
}

fn process_command(state: &mut State, input: &str) {
    let (command, args) = parse_command(input);

    if command.is_empty() {
        return;
    }

    if state.debug_mode {
        println!("[DEBUG] Command: '{}', Args: {}", command, args.len());
    }

    if strcmp_like(&command, "adduser") == 0 {
        cmd_adduser(state, &args);
    } else if strcmp_like(&command, "login") == 0 {
        cmd_login(state, &args);
    } else if strcmp_like(&command, "logout") == 0 {
        cmd_logout(state);
    } else if strcmp_like(&command, "whoami") == 0 {
        cmd_whoami(state);
    } else if strcmp_like(&command, "listusers") == 0 || strcmp_like(&command, "users") == 0 {
        cmd_listusers(state);
    } else if strcmp_like(&command, "createfile") == 0 || strcmp_like(&command, "touch") == 0 {
        cmd_createfile(state, &args);
    } else if strcmp_like(&command, "readfile") == 0 || strcmp_like(&command, "cat") == 0 {
        cmd_readfile(state, &args);
    } else if strcmp_like(&command, "writefile") == 0 || strcmp_like(&command, "write") == 0 {
        cmd_writefile(state, &args);
    } else if strcmp_like(&command, "deletefile") == 0 || strcmp_like(&command, "rm") == 0 {
        cmd_deletefile(state, &args);
    } else if strcmp_like(&command, "listfiles") == 0 || strcmp_like(&command, "ls") == 0 {
        cmd_listfiles(state);
    } else if strcmp_like(&command, "set") == 0 {
        cmd_set(state, &args);
    } else if strcmp_like(&command, "get") == 0 {
        cmd_get(state, &args);
    } else if strcmp_like(&command, "unset") == 0 {
        cmd_unset(state, &args);
    } else if strcmp_like(&command, "listvars") == 0 || strcmp_like(&command, "vars") == 0 {
        cmd_listvars(state);
    } else if strcmp_like(&command, "compare") == 0 || strcmp_like(&command, "cmp") == 0 {
        cmd_compare(&args);
    } else if strcmp_like(&command, "compareN") == 0 || strcmp_like(&command, "cmpn") == 0 {
        cmd_compare_n(&args);
    } else if strcmp_like(&command, "startswith") == 0 {
        cmd_startswith(&args);
    } else if strcmp_like(&command, "match") == 0 {
        cmd_match(&args);
    } else if strcmp_like(&command, "debug") == 0 {
        cmd_debug(state, &args);
    } else if strcmp_like(&command, "verbose") == 0 {
        cmd_verbose(state, &args);
    } else if strcmp_like(&command, "status") == 0 {
        cmd_status(state);
    } else if strcmp_like(&command, "time") == 0 {
        cmd_time();
    } else if strcmp_like(&command, "help") == 0 || strcmp_like(&command, "?") == 0 {
        cmd_help();
    } else if strcmp_like(&command, "exit") == 0 || strcmp_like(&command, "quit") == 0 {
        println!("Goodbye!");
        let _ = io::stdout().flush();
        process::exit(0);
    } else if strncmp_like(&command, "add", 3) == 0 {
        println!("Did you mean 'adduser'?");
    } else if strncmp_like(&command, "log", 3) == 0 {
        println!("Did you mean 'login' or 'logout'?");
    } else if strncmp_like(&command, "list", 4) == 0 {
        println!("Did you mean 'listusers', 'listfiles', or 'listvars'?");
    } else if strncmp_like(&command, "create", 6) == 0 {
        println!("Did you mean 'createfile'?");
    } else if strncmp_like(&command, "read", 4) == 0 {
        println!("Did you mean 'readfile'?");
    } else if strncmp_like(&command, "write", 5) == 0 {
        println!("Did you mean 'writefile'?");
    } else if strncmp_like(&command, "delete", 6) == 0 {
        println!("Did you mean 'deletefile'?");
    } else {
        println!(
            "Unknown command: '{}'. Type 'help' for available commands.",
            command
        );
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
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        let mut buf: Vec<u8> = Vec::new();
        if !fgets_like(&mut handle, &mut buf, MAX_INPUT) {
            break;
        }

        // Remove newline (strcspn behavior): truncate at first '\n'.
        if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            buf.truncate(pos);
        }

        // Convert to string for processing. Use lossy fallback for non-UTF-8 bytes.
        let input = match std::str::from_utf8(&buf) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(&buf).into_owned(),
        };

        if state.verbose_mode {
            println!("[VERBOSE] Processing: '{}'", input);
        }

        process_command(&mut state, &input);
    }
}
