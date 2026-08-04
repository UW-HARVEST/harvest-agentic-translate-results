// Translation of c_src/src/main.c to Rust
// Aims for byte-identical output for the same input.

use std::ffi::CStr;
use std::io::{Read, Write};
use std::process::exit;

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
struct FileT {
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
    current_user: Option<usize>,
    files: Vec<FileT>,
    variables: Vec<Variable>,
    debug_mode: bool,
    verbose_mode: bool,
    out: std::io::BufWriter<std::io::Stdout>,
}

impl State {
    fn new() -> Self {
        State {
            users: Vec::with_capacity(MAX_USERS),
            current_user: None,
            files: Vec::with_capacity(MAX_FILES),
            variables: Vec::with_capacity(MAX_VARIABLES),
            debug_mode: false,
            verbose_mode: false,
            out: std::io::BufWriter::new(std::io::stdout()),
        }
    }

    fn flush(&mut self) {
        let _ = self.out.flush();
    }
}

// strncpy + null-terminate semantics: copy up to max_len-1 bytes from src,
// no further padding required (we treat strings as logical strings).
fn truncate_str(src: &str, max_len_minus_one: usize) -> String {
    // Truncate to max_len_minus_one bytes, taking care of UTF-8 boundaries.
    // The C version uses bytes; we keep parity with raw bytes.
    let bytes = src.as_bytes();
    let take = bytes.len().min(max_len_minus_one);
    // To avoid splitting UTF-8 boundaries (which in plain ASCII inputs is fine),
    // we'll just take bytes and convert as lossy if needed.
    String::from_utf8_lossy(&bytes[..take]).into_owned()
}

// Mimic C's strtok with " \t" delimiters, returning a Vec of tokens.
fn tokenize_spaces_tabs(input: &str) -> Vec<&str> {
    // Split on runs of space and tab, skipping empty leading/trailing segments.
    input
        .split(|c: char| c == ' ' || c == '\t')
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_command(input: &str) -> (String, Vec<String>) {
    // Match the C parse_command: first truncate to MAX_INPUT - 1 bytes
    let bytes = input.as_bytes();
    let take = bytes.len().min(MAX_INPUT - 1);
    let truncated = String::from_utf8_lossy(&bytes[..take]).into_owned();

    let tokens = tokenize_spaces_tabs(&truncated);
    let mut cmd = String::new();
    let mut args: Vec<String> = Vec::new();

    if let Some(first) = tokens.first() {
        cmd = truncate_str(first, MAX_COMMAND - 1);
        for tok in tokens.iter().skip(1) {
            if args.len() >= MAX_ARGS {
                break;
            }
            args.push(truncate_str(tok, MAX_COMMAND - 1));
        }
    }

    (cmd, args)
}

fn arg_or_empty(args: &[String], i: usize) -> &str {
    if i < args.len() {
        args[i].as_str()
    } else {
        ""
    }
}

// C strcmp on bytes
fn strcmp(a: &str, b: &str) -> i32 {
    let aa = a.as_bytes();
    let bb = b.as_bytes();
    let n = aa.len().min(bb.len());
    for i in 0..n {
        if aa[i] != bb[i] {
            return aa[i] as i32 - bb[i] as i32;
        }
    }
    aa.len() as i32 - bb.len() as i32
}

fn strncmp(a: &str, b: &str, n: usize) -> i32 {
    let aa = a.as_bytes();
    let bb = b.as_bytes();
    let max = n.min(aa.len()).min(bb.len());
    for i in 0..max {
        if aa[i] != bb[i] {
            return aa[i] as i32 - bb[i] as i32;
        }
    }
    if max == n {
        return 0;
    }
    // One ran out before the other, before reaching n
    let ai = aa.get(max).copied().unwrap_or(0);
    let bi = bb.get(max).copied().unwrap_or(0);
    ai as i32 - bi as i32
}

// strcmp returning result that matches C's sign convention.
// Many libc implementations return values like 1, -1 or larger. Glibc returns
// the difference of unsigned chars, but the sign is what most code checks.
// We'll return the byte difference to match the raw value used in printf.
// Note: for cmd_compare we print %d, so we need an exact match.
// Glibc strcmp returns the difference of the first differing unsigned chars,
// or 0 if equal, or, when one is a prefix of the other, the value of the
// next unsigned char in the longer string (which is non-zero).
fn libc_strcmp(a: &str, b: &str) -> i32 {
    let aa = a.as_bytes();
    let bb = b.as_bytes();
    let n = aa.len().min(bb.len());
    for i in 0..n {
        if aa[i] != bb[i] {
            return aa[i] as i32 - bb[i] as i32;
        }
    }
    if aa.len() == bb.len() {
        0
    } else if aa.len() > bb.len() {
        aa[bb.len()] as i32
    } else {
        -(bb[aa.len()] as i32)
    }
}

fn libc_strncmp(a: &str, b: &str, n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    let aa = a.as_bytes();
    let bb = b.as_bytes();
    let max = n.min(aa.len()).min(bb.len());
    for i in 0..max {
        if aa[i] != bb[i] {
            return aa[i] as i32 - bb[i] as i32;
        }
    }
    // We compared `max` bytes. If max == n, we're done.
    if max == n {
        return 0;
    }
    // Otherwise, one string ended before reaching n.
    if aa.len() == bb.len() {
        0
    } else if aa.len() > bb.len() {
        aa[bb.len()] as i32
    } else {
        -(bb[aa.len()] as i32)
    }
}

fn strstr_idx(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.len() > h.len() {
        return None;
    }
    for i in 0..=(h.len() - n.len()) {
        if &h[i..i + n.len()] == n {
            return Some(i);
        }
    }
    None
}

// atoi mimicking C: skip leading whitespace, optional sign, parse digits, stop at non-digit
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n'
        || bytes[i] == b'\r' || bytes[i] == 0x0b || bytes[i] == 0x0c) {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        val = val.wrapping_neg();
    }
    // Wrap to i32 like C atoi (UB technically, but typically wraps).
    val as i32
}

// ====================== Commands ======================

fn cmd_adduser(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        let _ = writeln!(state.out, "Usage: adduser <username> <password> [permission_level]");
        return;
    }
    if state.users.len() >= MAX_USERS {
        let _ = writeln!(state.out, "Error: Maximum users reached");
        return;
    }
    for u in &state.users {
        if libc_strcmp(&u.name, &args[0]) == 0 {
            let _ = writeln!(state.out, "Error: User '{}' already exists", args[0]);
            return;
        }
    }

    let permission_level = if args.len() >= 3 { c_atoi(&args[2]) } else { 1 };
    let new_user = User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level,
        logged_in: false,
    };
    state.users.push(new_user);
    let added_level = state.users.last().unwrap().permission_level;
    let _ = writeln!(state.out, "User '{}' added with permission level {}", args[0], added_level);
}

fn cmd_login(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        let _ = writeln!(state.out, "Usage: login <username> <password>");
        return;
    }
    if let Some(idx) = state.current_user {
        if state.users[idx].logged_in {
            let _ = writeln!(
                state.out,
                "Error: User '{}' already logged in. Use 'logout' first.",
                state.users[idx].name
            );
            return;
        }
    }
    for i in 0..state.users.len() {
        if libc_strcmp(&state.users[i].name, &args[0]) == 0 {
            if libc_strcmp(&state.users[i].password, &args[1]) == 0 {
                state.users[i].logged_in = true;
                state.current_user = Some(i);
                let name = state.users[i].name.clone();
                let _ = writeln!(state.out, "Login successful. Welcome, {}!", name);
                return;
            } else {
                let _ = writeln!(state.out, "Error: Incorrect password");
                return;
            }
        }
    }
    let _ = writeln!(state.out, "Error: User not found");
}

fn cmd_logout(state: &mut State) {
    let idx = match state.current_user {
        Some(i) if state.users[i].logged_in => i,
        _ => {
            let _ = writeln!(state.out, "Error: No user logged in");
            return;
        }
    };
    let name = state.users[idx].name.clone();
    let _ = writeln!(state.out, "Goodbye, {}!", name);
    state.users[idx].logged_in = false;
    state.current_user = None;
}

fn cmd_whoami(state: &mut State) {
    let idx = match state.current_user {
        Some(i) if state.users[i].logged_in => i,
        _ => {
            let _ = writeln!(state.out, "Not logged in");
            return;
        }
    };
    let name = state.users[idx].name.clone();
    let level = state.users[idx].permission_level;
    let _ = writeln!(state.out, "Current user: {}", name);
    let _ = writeln!(state.out, "Permission level: {}", level);
}

fn cmd_listusers(state: &mut State) {
    if state.users.is_empty() {
        let _ = writeln!(state.out, "No users registered");
        return;
    }
    let _ = writeln!(state.out, "Registered users:");
    for u in &state.users {
        let suffix = if u.logged_in { "[logged in]" } else { "" };
        let _ = writeln!(
            state.out,
            "  {} (level {}) {}",
            u.name, u.permission_level, suffix
        );
    }
}

fn cmd_createfile(state: &mut State, args: &[String]) {
    let cur_idx = match state.current_user {
        Some(i) if state.users[i].logged_in => i,
        _ => {
            let _ = writeln!(state.out, "Error: Must be logged in");
            return;
        }
    };
    if args.is_empty() {
        let _ = writeln!(state.out, "Usage: createfile <filename> [content]");
        return;
    }
    if state.files.len() >= MAX_FILES {
        let _ = writeln!(state.out, "Error: Maximum files reached");
        return;
    }
    for f in &state.files {
        if libc_strcmp(&f.filename, &args[0]) == 0 {
            let _ = writeln!(state.out, "Error: File '{}' already exists", args[0]);
            return;
        }
    }
    let content = if args.len() >= 2 { args[1].clone() } else { String::new() };
    let owner = state.users[cur_idx].name.clone();
    state.files.push(FileT {
        filename: args[0].clone(),
        content,
        owner,
        permissions: 755,
    });
    let _ = writeln!(state.out, "File '{}' created", args[0]);
}

fn cmd_readfile(state: &mut State, args: &[String]) {
    if args.is_empty() {
        let _ = writeln!(state.out, "Usage: readfile <filename>");
        return;
    }
    for f in &state.files {
        if libc_strcmp(&f.filename, &args[0]) == 0 {
            let _ = writeln!(state.out, "=== {} ===", f.filename);
            let _ = writeln!(state.out, "Owner: {}", f.owner);
            let _ = writeln!(state.out, "Permissions: {}", f.permissions);
            let _ = writeln!(state.out, "Content: {}", f.content);
            return;
        }
    }
    let _ = writeln!(state.out, "Error: File '{}' not found", args[0]);
}

fn cmd_writefile(state: &mut State, args: &[String]) {
    let cur_idx = match state.current_user {
        Some(i) if state.users[i].logged_in => i,
        _ => {
            let _ = writeln!(state.out, "Error: Must be logged in");
            return;
        }
    };
    if args.len() < 2 {
        let _ = writeln!(state.out, "Usage: writefile <filename> <content>");
        return;
    }
    let cur_name = state.users[cur_idx].name.clone();
    let cur_perm = state.users[cur_idx].permission_level;
    for i in 0..state.files.len() {
        if libc_strcmp(&state.files[i].filename, &args[0]) == 0 {
            if libc_strcmp(&state.files[i].owner, &cur_name) == 0 || cur_perm >= 5 {
                state.files[i].content = args[1].clone();
                let _ = writeln!(state.out, "File '{}' updated", args[0]);
                return;
            } else {
                let _ = writeln!(state.out, "Error: Permission denied");
                return;
            }
        }
    }
    let _ = writeln!(state.out, "Error: File '{}' not found", args[0]);
}

fn cmd_deletefile(state: &mut State, args: &[String]) {
    let cur_idx = match state.current_user {
        Some(i) if state.users[i].logged_in => i,
        _ => {
            let _ = writeln!(state.out, "Error: Must be logged in");
            return;
        }
    };
    if args.is_empty() {
        let _ = writeln!(state.out, "Usage: deletefile <filename>");
        return;
    }
    let cur_name = state.users[cur_idx].name.clone();
    let cur_perm = state.users[cur_idx].permission_level;
    for i in 0..state.files.len() {
        if libc_strcmp(&state.files[i].filename, &args[0]) == 0 {
            if libc_strcmp(&state.files[i].owner, &cur_name) == 0 || cur_perm >= 9 {
                state.files.remove(i);
                let _ = writeln!(state.out, "File '{}' deleted", args[0]);
                return;
            } else {
                let _ = writeln!(state.out, "Error: Permission denied");
                return;
            }
        }
    }
    let _ = writeln!(state.out, "Error: File '{}' not found", args[0]);
}

fn cmd_listfiles(state: &mut State) {
    if state.files.is_empty() {
        let _ = writeln!(state.out, "No files");
        return;
    }
    let _ = writeln!(state.out, "Files:");
    for f in &state.files {
        let _ = writeln!(
            state.out,
            "  {} (owner: {}, perm: {})",
            f.filename, f.owner, f.permissions
        );
    }
}

fn cmd_set(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        let _ = writeln!(state.out, "Usage: set <name> <value>");
        return;
    }
    for v in state.variables.iter_mut() {
        if libc_strcmp(&v.name, &args[0]) == 0 {
            v.value = args[1].clone();
            let _ = writeln!(state.out, "Variable '{}' updated", args[0]);
            return;
        }
    }
    if state.variables.len() >= MAX_VARIABLES {
        let _ = writeln!(state.out, "Error: Maximum variables reached");
        return;
    }
    state.variables.push(Variable {
        name: args[0].clone(),
        value: args[1].clone(),
    });
    let _ = writeln!(state.out, "Variable '{}' set", args[0]);
}

fn cmd_get(state: &mut State, args: &[String]) {
    if args.is_empty() {
        let _ = writeln!(state.out, "Usage: get <name>");
        return;
    }
    for v in &state.variables {
        if libc_strcmp(&v.name, &args[0]) == 0 {
            let _ = writeln!(state.out, "{} = {}", v.name, v.value);
            return;
        }
    }
    let _ = writeln!(state.out, "Error: Variable '{}' not found", args[0]);
}

fn cmd_unset(state: &mut State, args: &[String]) {
    if args.is_empty() {
        let _ = writeln!(state.out, "Usage: unset <name>");
        return;
    }
    for i in 0..state.variables.len() {
        if libc_strcmp(&state.variables[i].name, &args[0]) == 0 {
            state.variables.remove(i);
            let _ = writeln!(state.out, "Variable '{}' unset", args[0]);
            return;
        }
    }
    let _ = writeln!(state.out, "Error: Variable '{}' not found", args[0]);
}

fn cmd_listvars(state: &mut State) {
    if state.variables.is_empty() {
        let _ = writeln!(state.out, "No variables set");
        return;
    }
    let _ = writeln!(state.out, "Variables:");
    for v in &state.variables {
        let _ = writeln!(state.out, "  {} = {}", v.name, v.value);
    }
}

fn cmd_compare(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        let _ = writeln!(state.out, "Usage: compare <string1> <string2>");
        return;
    }
    let result = libc_strcmp(&args[0], &args[1]);
    let _ = writeln!(state.out, "strcmp('{}', '{}') = {}", args[0], args[1], result);
    if result == 0 {
        let _ = writeln!(state.out, "Strings are equal");
    } else if result < 0 {
        let _ = writeln!(state.out, "'{}' < '{}'", args[0], args[1]);
    } else {
        let _ = writeln!(state.out, "'{}' > '{}'", args[0], args[1]);
    }
}

fn cmd_compare_n(state: &mut State, args: &[String]) {
    if args.len() < 3 {
        let _ = writeln!(state.out, "Usage: compareN <string1> <string2> <n>");
        return;
    }
    let n_signed = c_atoi(&args[2]);
    // strncmp takes size_t; if negative, reinterpret as huge unsigned. To match C behavior
    // we'll mimic by treating negative values as huge usize.
    let n: usize = if n_signed < 0 {
        // Mirror the C behavior (UB-ish, but practical): cast to usize via wrap.
        n_signed as isize as usize
    } else {
        n_signed as usize
    };
    let result = libc_strncmp(&args[0], &args[1], n);
    let _ = writeln!(
        state.out,
        "strncmp('{}', '{}', {}) = {}",
        args[0], args[1], n_signed, result
    );
    if result == 0 {
        let _ = writeln!(state.out, "First {} characters are equal", n_signed);
    } else if result < 0 {
        let _ = writeln!(
            state.out,
            "'{}' < '{}' (first {} chars)",
            args[0], args[1], n_signed
        );
    } else {
        let _ = writeln!(
            state.out,
            "'{}' > '{}' (first {} chars)",
            args[0], args[1], n_signed
        );
    }
}

fn cmd_startswith(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        let _ = writeln!(state.out, "Usage: startswith <string> <prefix>");
        return;
    }
    let prefix_len = args[1].as_bytes().len();
    if libc_strncmp(&args[0], &args[1], prefix_len) == 0 {
        let _ = writeln!(state.out, "'{}' starts with '{}'", args[0], args[1]);
    } else {
        let _ = writeln!(state.out, "'{}' does not start with '{}'", args[0], args[1]);
    }
}

fn cmd_match(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        let _ = writeln!(state.out, "Usage: match <pattern> <string1> [string2] ...");
        return;
    }
    let _ = writeln!(state.out, "Matching pattern '{}':", args[0]);
    let mut matches = 0;
    for i in 1..args.len() {
        if libc_strcmp(&args[0], &args[i]) == 0 {
            let _ = writeln!(state.out, "  '{}' - EXACT MATCH", args[i]);
            matches += 1;
        } else if strstr_idx(&args[i], &args[0]).is_some() {
            let _ = writeln!(state.out, "  '{}' - contains pattern", args[i]);
            matches += 1;
        } else {
            let _ = writeln!(state.out, "  '{}' - no match", args[i]);
        }
    }
    let _ = writeln!(state.out, "Total matches: {}", matches);
}

fn cmd_help(state: &mut State) {
    let _ = writeln!(state.out, "\n=== Command Interpreter Help ===");
    let _ = writeln!(state.out, "User Management:");
    let _ = writeln!(state.out, "  adduser <user> <pass> [level] - Add new user");
    let _ = writeln!(state.out, "  login <user> <pass>            - Login as user");
    let _ = writeln!(state.out, "  logout                         - Logout current user");
    let _ = writeln!(state.out, "  whoami                         - Show current user");
    let _ = writeln!(state.out, "  listusers                      - List all users");
    let _ = writeln!(state.out, "\nFile Management:");
    let _ = writeln!(state.out, "  createfile <name> [content]    - Create file");
    let _ = writeln!(state.out, "  readfile <name>                - Read file");
    let _ = writeln!(state.out, "  writefile <name> <content>     - Write to file");
    let _ = writeln!(state.out, "  deletefile <name>              - Delete file");
    let _ = writeln!(state.out, "  listfiles                      - List all files");
    let _ = writeln!(state.out, "\nVariable Management:");
    let _ = writeln!(state.out, "  set <name> <value>             - Set variable");
    let _ = writeln!(state.out, "  get <name>                     - Get variable");
    let _ = writeln!(state.out, "  unset <name>                   - Unset variable");
    let _ = writeln!(state.out, "  listvars                       - List all variables");
    let _ = writeln!(state.out, "\nString Operations:");
    let _ = writeln!(state.out, "  compare <str1> <str2>          - Compare strings");
    let _ = writeln!(state.out, "  compareN <str1> <str2> <n>     - Compare first N chars");
    let _ = writeln!(state.out, "  startswith <str> <prefix>      - Check if starts with");
    let _ = writeln!(state.out, "  match <pattern> <str> ...      - Match pattern");
    let _ = writeln!(state.out, "\nSystem:");
    let _ = writeln!(state.out, "  debug [on|off]                 - Toggle debug mode");
    let _ = writeln!(state.out, "  verbose [on|off]               - Toggle verbose mode");
    let _ = writeln!(state.out, "  status                         - Show system status");
    let _ = writeln!(state.out, "  time                           - Show current time");
    let _ = writeln!(state.out, "  help                           - Show this help");
    let _ = writeln!(state.out, "  exit                           - Exit program");
}

fn cmd_debug(state: &mut State, args: &[String]) {
    if args.is_empty() {
        let _ = writeln!(state.out, "Debug mode: {}", if state.debug_mode { "ON" } else { "OFF" });
        return;
    }
    if libc_strcmp(&args[0], "on") == 0 {
        state.debug_mode = true;
        let _ = writeln!(state.out, "Debug mode enabled");
    } else if libc_strcmp(&args[0], "off") == 0 {
        state.debug_mode = false;
        let _ = writeln!(state.out, "Debug mode disabled");
    } else {
        let _ = writeln!(state.out, "Usage: debug [on|off]");
    }
}

fn cmd_verbose(state: &mut State, args: &[String]) {
    if args.is_empty() {
        let _ = writeln!(state.out, "Verbose mode: {}", if state.verbose_mode { "ON" } else { "OFF" });
        return;
    }
    if libc_strcmp(&args[0], "on") == 0 {
        state.verbose_mode = true;
        let _ = writeln!(state.out, "Verbose mode enabled");
    } else if libc_strcmp(&args[0], "off") == 0 {
        state.verbose_mode = false;
        let _ = writeln!(state.out, "Verbose mode disabled");
    } else {
        let _ = writeln!(state.out, "Usage: verbose [on|off]");
    }
}

fn cmd_status(state: &mut State) {
    let _ = writeln!(state.out, "\n=== System Status ===");
    let _ = writeln!(state.out, "Users: {}/{}", state.users.len(), MAX_USERS);
    let _ = writeln!(state.out, "Files: {}/{}", state.files.len(), MAX_FILES);
    let _ = writeln!(state.out, "Variables: {}/{}", state.variables.len(), MAX_VARIABLES);
    let cur_name = match state.current_user {
        Some(i) if state.users[i].logged_in => state.users[i].name.clone(),
        _ => "none".to_string(),
    };
    let _ = writeln!(state.out, "Current user: {}", cur_name);
    let _ = writeln!(state.out, "Debug mode: {}", if state.debug_mode { "ON" } else { "OFF" });
    let _ = writeln!(state.out, "Verbose mode: {}", if state.verbose_mode { "ON" } else { "OFF" });
}

extern "C" {
    fn ctime(timep: *const libc::time_t) -> *const libc::c_char;
}

fn cmd_time(state: &mut State) {
    // Use libc time() and ctime() to match C output exactly.
    unsafe {
        let now: libc::time_t = libc::time(std::ptr::null_mut());
        let ptr = ctime(&now as *const libc::time_t);
        if ptr.is_null() {
            return;
        }
        let cstr = CStr::from_ptr(ptr);
        let s = cstr.to_string_lossy();
        // ctime already includes a trailing newline; printf("Current time: %s", ...)
        // does not add a newline.
        let _ = write!(state.out, "Current time: {}", s);
    }
}

fn process_command(state: &mut State, input: &str) {
    let (command, args) = parse_command(input);

    if command.is_empty() {
        return;
    }

    if state.debug_mode {
        let _ = writeln!(
            state.out,
            "[DEBUG] Command: '{}', Args: {}",
            command,
            args.len()
        );
    }

    if libc_strcmp(&command, "adduser") == 0 {
        cmd_adduser(state, &args);
    } else if libc_strcmp(&command, "login") == 0 {
        cmd_login(state, &args);
    } else if libc_strcmp(&command, "logout") == 0 {
        cmd_logout(state);
    } else if libc_strcmp(&command, "whoami") == 0 {
        cmd_whoami(state);
    } else if libc_strcmp(&command, "listusers") == 0 || libc_strcmp(&command, "users") == 0 {
        cmd_listusers(state);
    } else if libc_strcmp(&command, "createfile") == 0 || libc_strcmp(&command, "touch") == 0 {
        cmd_createfile(state, &args);
    } else if libc_strcmp(&command, "readfile") == 0 || libc_strcmp(&command, "cat") == 0 {
        cmd_readfile(state, &args);
    } else if libc_strcmp(&command, "writefile") == 0 || libc_strcmp(&command, "write") == 0 {
        cmd_writefile(state, &args);
    } else if libc_strcmp(&command, "deletefile") == 0 || libc_strcmp(&command, "rm") == 0 {
        cmd_deletefile(state, &args);
    } else if libc_strcmp(&command, "listfiles") == 0 || libc_strcmp(&command, "ls") == 0 {
        cmd_listfiles(state);
    } else if libc_strcmp(&command, "set") == 0 {
        cmd_set(state, &args);
    } else if libc_strcmp(&command, "get") == 0 {
        cmd_get(state, &args);
    } else if libc_strcmp(&command, "unset") == 0 {
        cmd_unset(state, &args);
    } else if libc_strcmp(&command, "listvars") == 0 || libc_strcmp(&command, "vars") == 0 {
        cmd_listvars(state);
    } else if libc_strcmp(&command, "compare") == 0 || libc_strcmp(&command, "cmp") == 0 {
        cmd_compare(state, &args);
    } else if libc_strcmp(&command, "compareN") == 0 || libc_strcmp(&command, "cmpn") == 0 {
        cmd_compare_n(state, &args);
    } else if libc_strcmp(&command, "startswith") == 0 {
        cmd_startswith(state, &args);
    } else if libc_strcmp(&command, "match") == 0 {
        cmd_match(state, &args);
    } else if libc_strcmp(&command, "debug") == 0 {
        cmd_debug(state, &args);
    } else if libc_strcmp(&command, "verbose") == 0 {
        cmd_verbose(state, &args);
    } else if libc_strcmp(&command, "status") == 0 {
        cmd_status(state);
    } else if libc_strcmp(&command, "time") == 0 {
        cmd_time(state);
    } else if libc_strcmp(&command, "help") == 0 || libc_strcmp(&command, "?") == 0 {
        cmd_help(state);
    } else if libc_strcmp(&command, "exit") == 0 || libc_strcmp(&command, "quit") == 0 {
        let _ = writeln!(state.out, "Goodbye!");
        state.flush();
        exit(0);
    } else if libc_strncmp(&command, "add", 3) == 0 {
        let _ = writeln!(state.out, "Did you mean 'adduser'?");
    } else if libc_strncmp(&command, "log", 3) == 0 {
        let _ = writeln!(state.out, "Did you mean 'login' or 'logout'?");
    } else if libc_strncmp(&command, "list", 4) == 0 {
        let _ = writeln!(state.out, "Did you mean 'listusers', 'listfiles', or 'listvars'?");
    } else if libc_strncmp(&command, "create", 6) == 0 {
        let _ = writeln!(state.out, "Did you mean 'createfile'?");
    } else if libc_strncmp(&command, "read", 4) == 0 {
        let _ = writeln!(state.out, "Did you mean 'readfile'?");
    } else if libc_strncmp(&command, "write", 5) == 0 {
        let _ = writeln!(state.out, "Did you mean 'writefile'?");
    } else if libc_strncmp(&command, "delete", 6) == 0 {
        let _ = writeln!(state.out, "Did you mean 'deletefile'?");
    } else {
        let _ = writeln!(
            state.out,
            "Unknown command: '{}'. Type 'help' for available commands.",
            command
        );
    }
}

// Mimic C's fgets: reads up to MAX_INPUT-1 bytes including a possible newline,
// then null-terminates (we just return the bytes). Returns None on EOF with
// no bytes read.
fn fgets_like(stdin: &mut std::io::StdinLock<'_>, buf: &mut Vec<u8>, max: usize) -> Option<()> {
    buf.clear();
    let mut byte = [0u8; 1];
    loop {
        if buf.len() >= max - 1 {
            // C fgets stops here; null terminator implicit. We don't put a NUL.
            break;
        }
        match stdin.read(&mut byte) {
            Ok(0) => {
                // EOF. If we read nothing, return None.
                if buf.is_empty() {
                    return None;
                }
                break;
            }
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
        }
    }
    Some(())
}

fn main() {
    let mut state = State::new();

    let _ = writeln!(state.out, "|----------------------------------------|");
    let _ = writeln!(state.out, "|   COMMAND INTERPRETER                  |");
    let _ = writeln!(state.out, "|   strcmp/strncmp demonstration         |");
    let _ = writeln!(state.out, "|----------------------------------------|");
    let _ = writeln!(state.out, "Type 'help' for available commands");
    let _ = writeln!(state.out);

    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut buf: Vec<u8> = Vec::with_capacity(MAX_INPUT);

    loop {
        let _ = write!(state.out, "> ");
        // Match C stdio buffering: typically when stdout is line-buffered (terminal),
        // it auto-flushes on newline. For "> " with no newline, C might not flush
        // until the next line is read. To match runtime appearance, we just don't
        // flush here. However for byte-correctness it doesn't matter.

        if fgets_like(&mut stdin_lock, &mut buf, MAX_INPUT).is_none() {
            break;
        }

        // Convert to a string, removing first '\n' (matching strcspn(input, "\n") = 0).
        let mut line_bytes: &[u8] = &buf;
        if let Some(pos) = line_bytes.iter().position(|&b| b == b'\n') {
            line_bytes = &line_bytes[..pos];
        }
        let input_str = String::from_utf8_lossy(line_bytes).into_owned();

        if state.verbose_mode {
            let _ = writeln!(state.out, "[VERBOSE] Processing: '{}'", input_str);
        }

        process_command(&mut state, &input_str);
    }

    state.flush();
}
