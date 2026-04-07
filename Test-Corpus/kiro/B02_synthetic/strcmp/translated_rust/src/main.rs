use std::io::{self, BufRead, Write};
use std::process;

const MAX_ARGS: usize = 10;
const MAX_USERS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_VARIABLES: usize = 20;

struct User {
    name: String,
    password: String,
    permission_level: i32,
    logged_in: bool,
}

struct File {
    filename: String,
    content: String,
    owner: String,
    permissions: i32,
}

struct Variable {
    name: String,
    value: String,
}

struct State {
    users: Vec<User>,
    current_user: Option<usize>,
    files: Vec<File>,
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

    fn current_logged_in(&self) -> bool {
        match self.current_user {
            Some(i) => self.users[i].logged_in,
            None => false,
        }
    }
}

fn strcmp(a: &str, b: &str) -> i32 {
    let a = a.as_bytes();
    let b = b.as_bytes();
    for i in 0..a.len().min(b.len()) {
        if a[i] != b[i] {
            return a[i] as i32 - b[i] as i32;
        }
    }
    a.len() as i32 - b.len() as i32
}

fn strncmp(a: &str, b: &str, n: usize) -> i32 {
    let a = a.as_bytes();
    let b = b.as_bytes();
    for i in 0..n.min(a.len()).min(b.len()) {
        if a[i] != b[i] {
            return a[i] as i32 - b[i] as i32;
        }
    }
    let al = a.len().min(n);
    let bl = b.len().min(n);
    al as i32 - bl as i32
}

fn parse_command(input: &str) -> (String, Vec<String>) {
    let mut args = Vec::new();
    let mut cmd = String::new();
    let mut tokens = input.split_whitespace();
    if let Some(c) = tokens.next() {
        cmd = c.to_string();
        for tok in tokens {
            if args.len() < MAX_ARGS {
                args.push(tok.to_string());
            }
        }
    }
    (cmd, args)
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
        if u.name == args[0] {
            println!("Error: User '{}' already exists", args[0]);
            return;
        }
    }
    let perm = if args.len() >= 3 {
        args[2].parse::<i32>().unwrap_or(0)
    } else {
        1
    };
    state.users.push(User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level: perm,
        logged_in: false,
    });
    println!("User '{}' added with permission level {}", args[0], perm);
}

fn cmd_login(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: login <username> <password>");
        return;
    }
    if let Some(ci) = state.current_user {
        if state.users[ci].logged_in {
            println!(
                "Error: User '{}' already logged in. Use 'logout' first.",
                state.users[ci].name
            );
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
    if !state.current_logged_in() {
        println!("Error: No user logged in");
        return;
    }
    let ci = state.current_user.unwrap();
    println!("Goodbye, {}!", state.users[ci].name);
    state.users[ci].logged_in = false;
    state.current_user = None;
}

fn cmd_whoami(state: &State) {
    if !state.current_logged_in() {
        println!("Not logged in");
        return;
    }
    let ci = state.current_user.unwrap();
    println!("Current user: {}", state.users[ci].name);
    println!("Permission level: {}", state.users[ci].permission_level);
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
    if !state.current_logged_in() {
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
    let ci = state.current_user.unwrap();
    let content = if args.len() >= 2 {
        args[1].clone()
    } else {
        String::new()
    };
    state.files.push(File {
        filename: args[0].clone(),
        content,
        owner: state.users[ci].name.clone(),
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
    if !state.current_logged_in() {
        println!("Error: Must be logged in");
        return;
    }
    if args.len() < 2 {
        println!("Usage: writefile <filename> <content>");
        return;
    }
    let ci = state.current_user.unwrap();
    let user_name = state.users[ci].name.clone();
    let user_perm = state.users[ci].permission_level;
    for f in &mut state.files {
        if f.filename == args[0] {
            if f.owner == user_name || user_perm >= 5 {
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

fn cmd_deletefile(state: &mut State, args: &[String]) {
    if !state.current_logged_in() {
        println!("Error: Must be logged in");
        return;
    }
    if args.is_empty() {
        println!("Usage: deletefile <filename>");
        return;
    }
    let ci = state.current_user.unwrap();
    let user_name = state.users[ci].name.clone();
    let user_perm = state.users[ci].permission_level;
    let mut found = None;
    for (i, f) in state.files.iter().enumerate() {
        if f.filename == args[0] {
            found = Some(i);
            break;
        }
    }
    if let Some(i) = found {
        if state.files[i].owner == user_name || user_perm >= 9 {
            state.files.remove(i);
            println!("File '{}' deleted", args[0]);
        } else {
            println!("Error: Permission denied");
        }
    } else {
        println!("Error: File '{}' not found", args[0]);
    }
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

fn cmd_set(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: set <name> <value>");
        return;
    }
    for v in &mut state.variables {
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
    let mut found = None;
    for (i, v) in state.variables.iter().enumerate() {
        if v.name == args[0] {
            found = Some(i);
            break;
        }
    }
    if let Some(i) = found {
        state.variables.remove(i);
        println!("Variable '{}' unset", args[0]);
    } else {
        println!("Error: Variable '{}' not found", args[0]);
    }
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
    let result = strcmp(&args[0], &args[1]);
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
    let n = args[2].parse::<i32>().unwrap_or(0);
    let result = strncmp(&args[0], &args[1], n as usize);
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
    let prefix_len = args[1].len();
    if strncmp(&args[0], &args[1], prefix_len) == 0 {
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
        if args[0] == args[i] {
            println!("  '{}' - EXACT MATCH", args[i]);
            matches += 1;
        } else if args[i].contains(&args[0] as &str) {
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

fn cmd_debug(state: &mut State, args: &[String]) {
    if args.is_empty() {
        println!(
            "Debug mode: {}",
            if state.debug_mode { "ON" } else { "OFF" }
        );
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
        println!(
            "Verbose mode: {}",
            if state.verbose_mode { "ON" } else { "OFF" }
        );
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
    println!();
    println!("=== System Status ===");
    println!("Users: {}/{}", state.users.len(), MAX_USERS);
    println!("Files: {}/{}", state.files.len(), MAX_FILES);
    println!("Variables: {}/{}", state.variables.len(), MAX_VARIABLES);
    let cur = if state.current_logged_in() {
        state.users[state.current_user.unwrap()].name.as_str()
    } else {
        "none"
    };
    println!("Current user: {}", cur);
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
    extern "C" {
        fn ctime(timep: *const libc::time_t) -> *const libc::c_char;
    }
    unsafe {
        let mut now: libc::time_t = 0;
        libc::time(&mut now);
        let ct = ctime(&now);
        let s = std::ffi::CStr::from_ptr(ct).to_bytes();
        // ctime's string ends with '\n', matching C's printf("Current time: %s", ctime(&now))
        print!("Current time: ");
        io::stdout().write_all(s).unwrap();
        io::stdout().flush().unwrap();
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
        "compareN" | "cmpn" => cmd_comparen(&args),
        "startswith" => cmd_startswith(&args),
        "match" => cmd_match(&args),
        "debug" => cmd_debug(state, &args),
        "verbose" => cmd_verbose(state, &args),
        "status" => cmd_status(state),
        "time" => cmd_time(),
        "help" | "?" => cmd_help(),
        "exit" | "quit" => {
            println!("Goodbye!");
            process::exit(0);
        }
        _ => {
            // Partial matches using strncmp-like prefix checks
            if command.starts_with("add") {
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
    let stdout = io::stdout();

    loop {
        print!("> ");
        stdout.lock().flush().unwrap();

        let mut input = String::new();
        if stdin.lock().read_line(&mut input).unwrap() == 0 {
            break;
        }
        // Remove trailing newline (like C's strcspn)
        if input.ends_with('\n') {
            input.pop();
        }
        if input.ends_with('\r') {
            input.pop();
        }

        if state.verbose_mode {
            println!("[VERBOSE] Processing: '{}'", input);
        }

        process_command(&mut state, &input);
    }
}
