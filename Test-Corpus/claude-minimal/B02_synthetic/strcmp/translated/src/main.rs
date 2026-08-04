// Translation of c_src/src/main.c to Rust.
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

use std::cmp::Ordering;
use std::io::{self, BufRead, Write};
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

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
struct FileEntry {
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
    current_user_index: Option<usize>,
    files: Vec<FileEntry>,
    variables: Vec<Variable>,
    debug_mode: bool,
    verbose_mode: bool,
}

impl State {
    fn new() -> Self {
        State {
            users: Vec::new(),
            current_user_index: None,
            files: Vec::new(),
            variables: Vec::new(),
            debug_mode: false,
            verbose_mode: false,
        }
    }

    fn current_user(&self) -> Option<&User> {
        self.current_user_index.map(|i| &self.users[i])
    }
}

// Truncate a string to at most `max_len` characters (mimics strncpy with
// `MAX_COMMAND - 1` size limit on input fields).
fn truncate_to(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        s[..max_len].to_string()
    } else {
        s.to_string()
    }
}

// Parse command and arguments. Returns (command, args, arg_count).
fn parse_command(input: &str) -> (String, Vec<String>, usize) {
    // Mimic the strncpy with MAX_INPUT - 1 size limit
    let temp = if input.len() > MAX_INPUT - 1 {
        &input[..MAX_INPUT - 1]
    } else {
        input
    };

    let mut tokens = temp.split(|c: char| c == ' ' || c == '\t').filter(|s| !s.is_empty());

    let cmd = match tokens.next() {
        Some(t) => truncate_to(t, MAX_COMMAND - 1),
        None => String::new(),
    };

    let mut args: Vec<String> = Vec::new();
    let mut arg_count = 0usize;
    for tok in tokens {
        if arg_count >= MAX_ARGS {
            break;
        }
        args.push(truncate_to(tok, MAX_COMMAND - 1));
        arg_count += 1;
    }

    (cmd, args, arg_count)
}

// strcmp-like comparison returning negative/zero/positive integer for byte
// ordering of strings.
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

// strncmp-like comparison considering at most `n` bytes.
fn strncmp_like(a: &str, b: &str, n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let limit = n.min(ab.len()).min(bb.len());
    for i in 0..limit {
        if ab[i] != bb[i] {
            return ab[i] as i32 - bb[i] as i32;
        }
    }
    // If we've consumed `n` bytes successfully, equal
    if limit == n {
        return 0;
    }
    // Otherwise the shorter string ended before reaching `n`.
    let a_byte = if limit < ab.len() { ab[limit] as i32 } else { 0 };
    let b_byte = if limit < bb.len() { bb[limit] as i32 } else { 0 };
    a_byte - b_byte
}

fn starts_with_prefix(s: &str, prefix: &str) -> bool {
    strncmp_like(s, prefix, prefix.len()) == 0
}

// User management commands
fn cmd_adduser(state: &mut State, args: &[String], arg_count: usize) {
    if arg_count < 2 {
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

    let permission_level = if arg_count >= 3 {
        // atoi: parse leading digits, default 0 on failure
        atoi_like(&args[2])
    } else {
        1
    };

    let user = User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level,
        logged_in: false,
    };
    state.users.push(user);

    println!(
        "User '{}' added with permission level {}",
        args[0], permission_level
    );
}

fn cmd_login(state: &mut State, args: &[String], arg_count: usize) {
    if arg_count < 2 {
        println!("Usage: login <username> <password>");
        return;
    }

    if let Some(idx) = state.current_user_index {
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
                state.current_user_index = Some(i);
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
    let idx = match state.current_user_index {
        Some(i) if state.users[i].logged_in => i,
        _ => {
            println!("Error: No user logged in");
            return;
        }
    };

    println!("Goodbye, {}!", state.users[idx].name);
    state.users[idx].logged_in = false;
    state.current_user_index = None;
}

fn cmd_whoami(state: &State) {
    match state.current_user() {
        Some(u) if u.logged_in => {
            println!("Current user: {}", u.name);
            println!("Permission level: {}", u.permission_level);
        }
        _ => {
            println!("Not logged in");
        }
    }
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

// File management commands
fn cmd_createfile(state: &mut State, args: &[String], arg_count: usize) {
    let (logged_in, owner_name) = match state.current_user() {
        Some(u) if u.logged_in => (true, u.name.clone()),
        _ => (false, String::new()),
    };

    if !logged_in {
        println!("Error: Must be logged in");
        return;
    }

    if arg_count < 1 {
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

    let content = if arg_count >= 2 {
        args[1].clone()
    } else {
        String::new()
    };

    state.files.push(FileEntry {
        filename: args[0].clone(),
        content,
        owner: owner_name,
        permissions: 755,
    });

    println!("File '{}' created", args[0]);
}

fn cmd_readfile(state: &State, args: &[String], arg_count: usize) {
    if arg_count < 1 {
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

fn cmd_writefile(state: &mut State, args: &[String], arg_count: usize) {
    let (logged_in, user_name, perm_level) = match state.current_user() {
        Some(u) if u.logged_in => (true, u.name.clone(), u.permission_level),
        _ => (false, String::new(), 0),
    };

    if !logged_in {
        println!("Error: Must be logged in");
        return;
    }

    if arg_count < 2 {
        println!("Usage: writefile <filename> <content>");
        return;
    }

    for f in state.files.iter_mut() {
        if strcmp_like(&f.filename, &args[0]) == 0 {
            if strcmp_like(&f.owner, &user_name) == 0 || perm_level >= 5 {
                f.content = args[1].clone();
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

fn cmd_deletefile(state: &mut State, args: &[String], arg_count: usize) {
    let (logged_in, user_name, perm_level) = match state.current_user() {
        Some(u) if u.logged_in => (true, u.name.clone(), u.permission_level),
        _ => (false, String::new(), 0),
    };

    if !logged_in {
        println!("Error: Must be logged in");
        return;
    }

    if arg_count < 1 {
        println!("Usage: deletefile <filename>");
        return;
    }

    for i in 0..state.files.len() {
        if strcmp_like(&state.files[i].filename, &args[0]) == 0 {
            if strcmp_like(&state.files[i].owner, &user_name) == 0 || perm_level >= 9 {
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

// Variable commands
fn cmd_set(state: &mut State, args: &[String], arg_count: usize) {
    if arg_count < 2 {
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

fn cmd_get(state: &State, args: &[String], arg_count: usize) {
    if arg_count < 1 {
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

fn cmd_unset(state: &mut State, args: &[String], arg_count: usize) {
    if arg_count < 1 {
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

// String comparison commands
fn cmd_compare(args: &[String], arg_count: usize) {
    if arg_count < 2 {
        println!("Usage: compare <string1> <string2>");
        return;
    }

    let result = strcmp_like(&args[0], &args[1]);

    println!("strcmp('{}', '{}') = {}", args[0], args[1], result);

    match result.cmp(&0) {
        Ordering::Equal => println!("Strings are equal"),
        Ordering::Less => println!("'{}' < '{}'", args[0], args[1]),
        Ordering::Greater => println!("'{}' > '{}'", args[0], args[1]),
    }
}

fn cmd_compare_n(args: &[String], arg_count: usize) {
    if arg_count < 3 {
        println!("Usage: compareN <string1> <string2> <n>");
        return;
    }

    let n_signed = atoi_like(&args[2]);
    // Match C's behavior loosely: treat negative n as 0 here to avoid underflow.
    let n_usize = if n_signed < 0 { 0 } else { n_signed as usize };
    let result = strncmp_like(&args[0], &args[1], n_usize);

    println!(
        "strncmp('{}', '{}', {}) = {}",
        args[0], args[1], n_signed, result
    );

    match result.cmp(&0) {
        Ordering::Equal => println!("First {} characters are equal", n_signed),
        Ordering::Less => println!("'{}' < '{}' (first {} chars)", args[0], args[1], n_signed),
        Ordering::Greater => println!("'{}' > '{}' (first {} chars)", args[0], args[1], n_signed),
    }
}

fn cmd_startswith(args: &[String], arg_count: usize) {
    if arg_count < 2 {
        println!("Usage: startswith <string> <prefix>");
        return;
    }

    if starts_with_prefix(&args[0], &args[1]) {
        println!("'{}' starts with '{}'", args[0], args[1]);
    } else {
        println!("'{}' does not start with '{}'", args[0], args[1]);
    }
}

fn cmd_match(args: &[String], arg_count: usize) {
    if arg_count < 2 {
        println!("Usage: match <pattern> <string1> [string2] ...");
        return;
    }

    println!("Matching pattern '{}':", args[0]);
    let mut matches = 0;

    for i in 1..arg_count {
        if strcmp_like(&args[0], &args[i]) == 0 {
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

// System commands
fn cmd_help() {
    println!("\n=== Command Interpreter Help ===");
    println!("User Management:");
    println!("  adduser <user> <pass> [level] - Add new user");
    println!("  login <user> <pass>            - Login as user");
    println!("  logout                         - Logout current user");
    println!("  whoami                         - Show current user");
    println!("  listusers                      - List all users");
    println!("\nFile Management:");
    println!("  createfile <name> [content]    - Create file");
    println!("  readfile <name>                - Read file");
    println!("  writefile <name> <content>     - Write to file");
    println!("  deletefile <name>              - Delete file");
    println!("  listfiles                      - List all files");
    println!("\nVariable Management:");
    println!("  set <name> <value>             - Set variable");
    println!("  get <name>                     - Get variable");
    println!("  unset <name>                   - Unset variable");
    println!("  listvars                       - List all variables");
    println!("\nString Operations:");
    println!("  compare <str1> <str2>          - Compare strings");
    println!("  compareN <str1> <str2> <n>     - Compare first N chars");
    println!("  startswith <str> <prefix>      - Check if starts with");
    println!("  match <pattern> <str> ...      - Match pattern");
    println!("\nSystem:");
    println!("  debug [on|off]                 - Toggle debug mode");
    println!("  verbose [on|off]               - Toggle verbose mode");
    println!("  status                         - Show system status");
    println!("  time                           - Show current time");
    println!("  help                           - Show this help");
    println!("  exit                           - Exit program");
}

fn cmd_debug(state: &mut State, args: &[String], arg_count: usize) {
    if arg_count < 1 {
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

fn cmd_verbose(state: &mut State, args: &[String], arg_count: usize) {
    if arg_count < 1 {
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
    println!("\n=== System Status ===");
    println!("Users: {}/{}", state.users.len(), MAX_USERS);
    println!("Files: {}/{}", state.files.len(), MAX_FILES);
    println!("Variables: {}/{}", state.variables.len(), MAX_VARIABLES);
    let current_name = match state.current_user() {
        Some(u) if u.logged_in => u.name.as_str(),
        _ => "none",
    };
    println!("Current user: {}", current_name);
    println!(
        "Debug mode: {}",
        if state.debug_mode { "ON" } else { "OFF" }
    );
    println!(
        "Verbose mode: {}",
        if state.verbose_mode { "ON" } else { "OFF" }
    );
}

fn cmd_time() {
    // Mimic ctime() output format: "Day Mon DD HH:MM:SS YYYY\n"
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    println!("Current time: {}", format_ctime(secs));
}

// Format epoch seconds in the same form as C's ctime():
// "Www Mmm dd hh:mm:ss yyyy" (without the trailing newline; println! adds one).
fn format_ctime(secs: i64) -> String {
    // Treat input as UTC seconds since epoch
    let days_since_epoch = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);
    let hour = (secs_of_day / 3600) as u32;
    let minute = ((secs_of_day / 60) % 60) as u32;
    let second = (secs_of_day % 60) as u32;

    // 1970-01-01 was a Thursday => weekday index 4 (where 0 = Sunday).
    let weekday = ((days_since_epoch + 4).rem_euclid(7)) as usize;
    let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    let (year, month, day) = days_to_ymd(days_since_epoch);
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    format!(
        "{} {} {:>2} {:02}:{:02}:{:02} {}",
        weekdays[weekday],
        months[(month - 1) as usize],
        day,
        hour,
        minute,
        second,
        year
    )
}

// Convert days since 1970-01-01 to (year, month, day) using a civil-from-days
// algorithm.
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Howard Hinnant's civil_from_days algorithm.
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as i64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (year, m, d)
}

// atoi-like: parse leading optional sign and digits; returns 0 if no digits.
fn atoi_like(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        result = result.saturating_mul(10).saturating_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    let signed = sign as i64 * result;
    if signed > i32::MAX as i64 {
        i32::MAX
    } else if signed < i32::MIN as i64 {
        i32::MIN
    } else {
        signed as i32
    }
}

// Main command processor
fn process_command(state: &mut State, input: &str) {
    let (command, args, arg_count) = parse_command(input);

    if command.is_empty() {
        return;
    }

    if state.debug_mode {
        println!("[DEBUG] Command: '{}', Args: {}", command, arg_count);
    }

    // User commands
    if strcmp_like(&command, "adduser") == 0 {
        cmd_adduser(state, &args, arg_count);
    } else if strcmp_like(&command, "login") == 0 {
        cmd_login(state, &args, arg_count);
    } else if strcmp_like(&command, "logout") == 0 {
        cmd_logout(state);
    } else if strcmp_like(&command, "whoami") == 0 {
        cmd_whoami(state);
    } else if strcmp_like(&command, "listusers") == 0 || strcmp_like(&command, "users") == 0 {
        cmd_listusers(state);
    }
    // File commands
    else if strcmp_like(&command, "createfile") == 0 || strcmp_like(&command, "touch") == 0 {
        cmd_createfile(state, &args, arg_count);
    } else if strcmp_like(&command, "readfile") == 0 || strcmp_like(&command, "cat") == 0 {
        cmd_readfile(state, &args, arg_count);
    } else if strcmp_like(&command, "writefile") == 0 || strcmp_like(&command, "write") == 0 {
        cmd_writefile(state, &args, arg_count);
    } else if strcmp_like(&command, "deletefile") == 0 || strcmp_like(&command, "rm") == 0 {
        cmd_deletefile(state, &args, arg_count);
    } else if strcmp_like(&command, "listfiles") == 0 || strcmp_like(&command, "ls") == 0 {
        cmd_listfiles(state);
    }
    // Variable commands
    else if strcmp_like(&command, "set") == 0 {
        cmd_set(state, &args, arg_count);
    } else if strcmp_like(&command, "get") == 0 {
        cmd_get(state, &args, arg_count);
    } else if strcmp_like(&command, "unset") == 0 {
        cmd_unset(state, &args, arg_count);
    } else if strcmp_like(&command, "listvars") == 0 || strcmp_like(&command, "vars") == 0 {
        cmd_listvars(state);
    }
    // String comparison commands
    else if strcmp_like(&command, "compare") == 0 || strcmp_like(&command, "cmp") == 0 {
        cmd_compare(&args, arg_count);
    } else if strcmp_like(&command, "compareN") == 0 || strcmp_like(&command, "cmpn") == 0 {
        cmd_compare_n(&args, arg_count);
    } else if strcmp_like(&command, "startswith") == 0 {
        cmd_startswith(&args, arg_count);
    } else if strcmp_like(&command, "match") == 0 {
        cmd_match(&args, arg_count);
    }
    // System commands
    else if strcmp_like(&command, "debug") == 0 {
        cmd_debug(state, &args, arg_count);
    } else if strcmp_like(&command, "verbose") == 0 {
        cmd_verbose(state, &args, arg_count);
    } else if strcmp_like(&command, "status") == 0 {
        cmd_status(state);
    } else if strcmp_like(&command, "time") == 0 {
        cmd_time();
    } else if strcmp_like(&command, "help") == 0 || strcmp_like(&command, "?") == 0 {
        cmd_help();
    } else if strcmp_like(&command, "exit") == 0 || strcmp_like(&command, "quit") == 0 {
        println!("Goodbye!");
        exit(0);
    }
    // Partial matches via strncmp
    else if strncmp_like(&command, "add", 3) == 0 {
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
    println!("Type 'help' for available commands\n");

    let stdin = io::stdin();
    let mut state = State::new();
    let mut input_line = String::new();
    let mut handle = stdin.lock();

    loop {
        print!("> ");
        io::stdout().flush().ok();

        input_line.clear();
        match handle.read_line(&mut input_line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        // Strip trailing newline (and CR if present), like strcspn(input, "\n").
        if let Some(idx) = input_line.find('\n') {
            input_line.truncate(idx);
        }
        if input_line.ends_with('\r') {
            input_line.pop();
        }

        // Mimic MAX_INPUT-1 truncation: fgets reads at most MAX_INPUT-1 chars.
        if input_line.len() > MAX_INPUT - 1 {
            input_line.truncate(MAX_INPUT - 1);
        }

        if state.verbose_mode {
            println!("[VERBOSE] Processing: '{}'", input_line);
        }

        process_command(&mut state, &input_line);
    }
}
