use std::io::{self, Write};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

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
struct FileEntry {
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

struct AppState {
    users: Vec<User>,
    current_user: Option<usize>,
    files: Vec<FileEntry>,
    variables: Vec<Variable>,
    debug_mode: bool,
    verbose_mode: bool,
}

impl AppState {
    fn new() -> Self {
        Self {
            users: Vec::new(),
            current_user: None,
            files: Vec::new(),
            variables: Vec::new(),
            debug_mode: false,
            verbose_mode: false,
        }
    }

    fn current_user_ref(&self) -> Option<&User> {
        self.current_user.and_then(|idx| self.users.get(idx))
    }

    fn is_logged_in(&self) -> bool {
        self.current_user_ref().map(|u| u.logged_in).unwrap_or(false)
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn parse_i32(s: &str) -> i32 {
    s.parse::<i32>().unwrap_or(0)
}

fn parse_usize(s: &str) -> usize {
    s.parse::<usize>().unwrap_or(0)
}

fn parse_command(input: &str) -> (String, Vec<String>) {
    let mut parts = input.split_whitespace();
    let cmd = parts.next().unwrap_or("");
    let cmd = truncate_chars(cmd, MAX_COMMAND - 1);
    let args = parts
        .take(MAX_ARGS)
        .map(|s| truncate_chars(s, MAX_COMMAND - 1))
        .collect::<Vec<_>>();
    (cmd, args)
}

fn cmd_adduser(state: &mut AppState, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: adduser <username> <password> [permission_level]");
        return;
    }

    if state.users.len() >= MAX_USERS {
        println!("Error: Maximum users reached");
        return;
    }

    if state.users.iter().any(|u| u.name == args[0]) {
        println!("Error: User '{}' already exists", args[0]);
        return;
    }

    let permission_level = if args.len() >= 3 { parse_i32(&args[2]) } else { 1 };
    let user = User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level,
        logged_in: false,
    };
    state.users.push(user);

    println!(
        "User '{}' added with permission level {}",
        args[0],
        state.users.last().map(|u| u.permission_level).unwrap_or(1)
    );
}

fn cmd_login(state: &mut AppState, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: login <username> <password>");
        return;
    }

    if let Some(user) = state.current_user_ref() {
        if user.logged_in {
            println!(
                "Error: User '{}' already logged in. Use 'logout' first.",
                user.name
            );
            return;
        }
    }

    if let Some(i) = state.users.iter().position(|u| u.name == args[0]) {
        if state.users[i].password == args[1] {
            state.users[i].logged_in = true;
            state.current_user = Some(i);
            println!("Login successful. Welcome, {}!", state.users[i].name);
        } else {
            println!("Error: Incorrect password");
        }
        return;
    }

    println!("Error: User not found");
}

fn cmd_logout(state: &mut AppState) {
    let Some(idx) = state.current_user else {
        println!("Error: No user logged in");
        return;
    };

    if !state.users.get(idx).map(|u| u.logged_in).unwrap_or(false) {
        println!("Error: No user logged in");
        return;
    }

    let name = state.users[idx].name.clone();
    println!("Goodbye, {}!", name);
    state.users[idx].logged_in = false;
    state.current_user = None;
}

fn cmd_whoami(state: &AppState) {
    if let Some(user) = state.current_user_ref() {
        if user.logged_in {
            println!("Current user: {}", user.name);
            println!("Permission level: {}", user.permission_level);
            return;
        }
    }

    println!("Not logged in");
}

fn cmd_listusers(state: &AppState) {
    if state.users.is_empty() {
        println!("No users registered");
        return;
    }

    println!("Registered users:");
    for user in &state.users {
        println!(
            "  {} (level {}) {}",
            user.name,
            user.permission_level,
            if user.logged_in { "[logged in]" } else { "" }
        );
    }
}

fn cmd_createfile(state: &mut AppState, args: &[String]) {
    if !state.is_logged_in() {
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

    if state.files.iter().any(|f| f.filename == args[0]) {
        println!("Error: File '{}' already exists", args[0]);
        return;
    }

    let owner = state
        .current_user_ref()
        .map(|u| u.name.clone())
        .unwrap_or_default();

    state.files.push(FileEntry {
        filename: args[0].clone(),
        content: if args.len() >= 2 { args[1].clone() } else { String::new() },
        owner,
        permissions: 755,
    });

    println!("File '{}' created", args[0]);
}

fn cmd_readfile(state: &AppState, args: &[String]) {
    if args.is_empty() {
        println!("Usage: readfile <filename>");
        return;
    }

    if let Some(file) = state.files.iter().find(|f| f.filename == args[0]) {
        println!("=== {} ===", file.filename);
        println!("Owner: {}", file.owner);
        println!("Permissions: {}", file.permissions);
        println!("Content: {}", file.content);
        return;
    }

    println!("Error: File '{}' not found", args[0]);
}

fn cmd_writefile(state: &mut AppState, args: &[String]) {
    if !state.is_logged_in() {
        println!("Error: Must be logged in");
        return;
    }

    if args.len() < 2 {
        println!("Usage: writefile <filename> <content>");
        return;
    }

    let current_name = state
        .current_user_ref()
        .map(|u| u.name.clone())
        .unwrap_or_default();
    let permission_level = state.current_user_ref().map(|u| u.permission_level).unwrap_or(0);

    if let Some(file) = state.files.iter_mut().find(|f| f.filename == args[0]) {
        if file.owner == current_name || permission_level >= 5 {
            file.content = args[1].clone();
            println!("File '{}' updated", args[0]);
        } else {
            println!("Error: Permission denied");
        }
        return;
    }

    println!("Error: File '{}' not found", args[0]);
}

fn cmd_deletefile(state: &mut AppState, args: &[String]) {
    if !state.is_logged_in() {
        println!("Error: Must be logged in");
        return;
    }

    if args.is_empty() {
        println!("Usage: deletefile <filename>");
        return;
    }

    let current_name = state
        .current_user_ref()
        .map(|u| u.name.clone())
        .unwrap_or_default();
    let permission_level = state.current_user_ref().map(|u| u.permission_level).unwrap_or(0);

    if let Some(pos) = state.files.iter().position(|f| f.filename == args[0]) {
        let allowed = state.files[pos].owner == current_name || permission_level >= 9;
        if allowed {
            state.files.remove(pos);
            println!("File '{}' deleted", args[0]);
        } else {
            println!("Error: Permission denied");
        }
        return;
    }

    println!("Error: File '{}' not found", args[0]);
}

fn cmd_listfiles(state: &AppState) {
    if state.files.is_empty() {
        println!("No files");
        return;
    }

    println!("Files:");
    for file in &state.files {
        println!(
            "  {} (owner: {}, perm: {})",
            file.filename, file.owner, file.permissions
        );
    }
}

fn cmd_set(state: &mut AppState, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: set <name> <value>");
        return;
    }

    if let Some(var) = state.variables.iter_mut().find(|v| v.name == args[0]) {
        var.value = args[1].clone();
        println!("Variable '{}' updated", args[0]);
        return;
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

fn cmd_get(state: &AppState, args: &[String]) {
    if args.is_empty() {
        println!("Usage: get <name>");
        return;
    }

    if let Some(var) = state.variables.iter().find(|v| v.name == args[0]) {
        println!("{} = {}", var.name, var.value);
        return;
    }

    println!("Error: Variable '{}' not found", args[0]);
}

fn cmd_unset(state: &mut AppState, args: &[String]) {
    if args.is_empty() {
        println!("Usage: unset <name>");
        return;
    }

    if let Some(pos) = state.variables.iter().position(|v| v.name == args[0]) {
        state.variables.remove(pos);
        println!("Variable '{}' unset", args[0]);
        return;
    }

    println!("Error: Variable '{}' not found", args[0]);
}

fn cmd_listvars(state: &AppState) {
    if state.variables.is_empty() {
        println!("No variables set");
        return;
    }

    println!("Variables:");
    for var in &state.variables {
        println!("  {} = {}", var.name, var.value);
    }
}

fn strcmp_like(a: &str, b: &str) -> i32 {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let min_len = ab.len().min(bb.len());
    for i in 0..min_len {
        if ab[i] != bb[i] {
            return ab[i] as i32 - bb[i] as i32;
        }
    }
    if ab.len() == bb.len() {
        0
    } else if ab.len() < bb.len() {
        -(bb[min_len] as i32)
    } else {
        ab[min_len] as i32
    }
}

fn strncmp_like(a: &str, b: &str, n: usize) -> i32 {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    for i in 0..n {
        let ac = *ab.get(i).unwrap_or(&0);
        let bc = *bb.get(i).unwrap_or(&0);
        if ac != bc {
            return ac as i32 - bc as i32;
        }
        if ac == 0 {
            return 0;
        }
    }
    0
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

fn cmd_comparen(args: &[String]) {
    if args.len() < 3 {
        println!("Usage: compareN <string1> <string2> <n>");
        return;
    }

    let n = parse_usize(&args[2]);
    let result = strncmp_like(&args[0], &args[1], n);

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

    if args[0].starts_with(&args[1]) {
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

    for candidate in &args[1..] {
        if args[0] == *candidate {
            println!("  '{}' - EXACT MATCH", candidate);
            matches += 1;
        } else if candidate.contains(&args[0]) {
            println!("  '{}' - contains pattern", candidate);
            matches += 1;
        } else {
            println!("  '{}' - no match", candidate);
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

fn cmd_debug(state: &mut AppState, args: &[String]) {
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

fn cmd_verbose(state: &mut AppState, args: &[String]) {
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

fn cmd_status(state: &AppState) {
    println!();
    println!("=== System Status ===");
    println!("Users: {}/{}", state.users.len(), MAX_USERS);
    println!("Files: {}/{}", state.files.len(), MAX_FILES);
    println!("Variables: {}/{}", state.variables.len(), MAX_VARIABLES);
    println!(
        "Current user: {}",
        state
            .current_user_ref()
            .filter(|u| u.logged_in)
            .map(|u| u.name.as_str())
            .unwrap_or("none")
    );
    println!("Debug mode: {}", if state.debug_mode { "ON" } else { "OFF" });
    println!("Verbose mode: {}", if state.verbose_mode { "ON" } else { "OFF" });
}

fn cmd_time() {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => println!("Current time: {}", duration.as_secs()),
        Err(_) => println!("Current time: 0"),
    }
}

fn process_command(state: &mut AppState, input: &str) {
    let (command, args) = parse_command(input);

    if command.is_empty() {
        return;
    }

    if state.debug_mode {
        println!("[DEBUG] Command: '{}', Args: {}", command, args.len());
    }

    if command == "adduser" {
        cmd_adduser(state, &args);
    } else if command == "login" {
        cmd_login(state, &args);
    } else if command == "logout" {
        cmd_logout(state);
    } else if command == "whoami" {
        cmd_whoami(state);
    } else if command == "listusers" || command == "users" {
        cmd_listusers(state);
    } else if command == "createfile" || command == "touch" {
        cmd_createfile(state, &args);
    } else if command == "readfile" || command == "cat" {
        cmd_readfile(state, &args);
    } else if command == "writefile" || command == "write" {
        cmd_writefile(state, &args);
    } else if command == "deletefile" || command == "rm" {
        cmd_deletefile(state, &args);
    } else if command == "listfiles" || command == "ls" {
        cmd_listfiles(state);
    } else if command == "set" {
        cmd_set(state, &args);
    } else if command == "get" {
        cmd_get(state, &args);
    } else if command == "unset" {
        cmd_unset(state, &args);
    } else if command == "listvars" || command == "vars" {
        cmd_listvars(state);
    } else if command == "compare" || command == "cmp" {
        cmd_compare(&args);
    } else if command == "compareN" || command == "cmpn" {
        cmd_comparen(&args);
    } else if command == "startswith" {
        cmd_startswith(&args);
    } else if command == "match" {
        cmd_match(&args);
    } else if command == "debug" {
        cmd_debug(state, &args);
    } else if command == "verbose" {
        cmd_verbose(state, &args);
    } else if command == "status" {
        cmd_status(state);
    } else if command == "time" {
        cmd_time();
    } else if command == "help" || command == "?" {
        cmd_help();
    } else if command == "exit" || command == "quit" {
        println!("Goodbye!");
        process::exit(0);
    } else if command.starts_with("add") {
        println!("Did you mean 'adduser'?");
    } else if command.starts_with("log") {
        println!("Did you mean 'login' or 'logout'?");
    } else if command.starts_with("list") {
        println!("Did you mean 'listusers', 'listfiles', or 'listvars'?");
    } else if command.starts_with("create") {
        println!("Did you mean 'createfile'?");
    } else if command.starts_with("read") {
        println!("Did you mean 'readfile'?");
    } else if command.starts_with("write") {
        println!("Did you mean 'writefile'?");
    } else if command.starts_with("delete") {
        println!("Did you mean 'deletefile'?");
    } else {
        println!("Unknown command: '{}'. Type 'help' for available commands.", command);
    }
}

fn main() {
    println!("|----------------------------------------|");
    println!("|   COMMAND INTERPRETER                  |");
    println!("|   strcmp/strncmp demonstration         |");
    println!("|----------------------------------------|");
    println!("Type 'help' for available commands\n");

    let mut state = AppState::new();
    let stdin = io::stdin();

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        if input.len() > MAX_INPUT {
            input.truncate(MAX_INPUT);
        }

        while input.ends_with('\n') || input.ends_with('\r') {
            input.pop();
        }

        if state.verbose_mode {
            println!("[VERBOSE] Processing: '{}'", input);
        }

        process_command(&mut state, &input);
    }
}
