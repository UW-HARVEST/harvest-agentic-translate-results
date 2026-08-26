use std::io::{self, Write};
use std::time::SystemTime;

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
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
}

fn parse_command(input: &str) -> (String, Vec<String>) {
    let mut parts = input.split_whitespace();
    let cmd = parts.next().unwrap_or("").to_string();
    let args: Vec<String> = parts.map(|s| s.to_string()).collect();
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
    
    for user in &state.users {
        if user.name == args[0] {
            println!("Error: User '{}' already exists", args[0]);
            return;
        }
    }
    
    let permission_level = if args.len() >= 3 {
        args[2].parse().unwrap_or(1)
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
    
    for (i, user) in state.users.iter_mut().enumerate() {
        if user.name == args[0] {
            if user.password == args[1] {
                user.logged_in = true;
                state.current_user = Some(i);
                println!("Login successful. Welcome, {}!", user.name);
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
    if let Some(idx) = state.current_user {
        if state.users[idx].logged_in {
            println!("Goodbye, {}!", state.users[idx].name);
            state.users[idx].logged_in = false;
            state.current_user = None;
            return;
        }
    }
    println!("Error: No user logged in");
}

fn cmd_whoami(state: &State) {
    if let Some(idx) = state.current_user {
        if state.users[idx].logged_in {
            println!("Current user: {}", state.users[idx].name);
            println!("Permission level: {}", state.users[idx].permission_level);
            return;
        }
    }
    println!("Not logged in");
}

fn cmd_listusers(state: &State) {
    if state.users.is_empty() {
        println!("No users registered");
        return;
    }
    
    println!("Registered users:");
    for user in &state.users {
        let status = if user.logged_in { " [logged in]" } else { "" };
        println!("  {} (level {}){}", user.name, user.permission_level, status);
    }
}

fn cmd_createfile(state: &mut State, args: &[String]) {
    if state.current_user.is_none() || !state.users[state.current_user.unwrap()].logged_in {
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
    
    for file in &state.files {
        if file.filename == args[0] {
            println!("Error: File '{}' already exists", args[0]);
            return;
        }
    }
    
    let content = if args.len() >= 2 { args[1].clone() } else { String::new() };
    let owner = state.users[state.current_user.unwrap()].name.clone();
    
    let file = File {
        filename: args[0].clone(),
        content,
        owner,
        permissions: 755,
    };
    state.files.push(file);
    println!("File '{}' created", args[0]);
}

fn cmd_readfile(state: &State, args: &[String]) {
    if args.is_empty() {
        println!("Usage: readfile <filename>");
        return;
    }
    
    for file in &state.files {
        if file.filename == args[0] {
            println!("=== {} ===", file.filename);
            println!("Owner: {}", file.owner);
            println!("Permissions: {}", file.permissions);
            println!("Content: {}", file.content);
            return;
        }
    }
    
    println!("Error: File '{}' not found", args[0]);
}

fn cmd_writefile(state: &mut State, args: &[String]) {
    if state.current_user.is_none() || !state.users[state.current_user.unwrap()].logged_in {
        println!("Error: Must be logged in");
        return;
    }
    
    if args.len() < 2 {
        println!("Usage: writefile <filename> <content>");
        return;
    }
    
    let current_user_idx = state.current_user.unwrap();
    let current_user_name = state.users[current_user_idx].name.clone();
    let current_user_level = state.users[current_user_idx].permission_level;
    
    for file in &mut state.files {
        if file.filename == args[0] {
            if file.owner == current_user_name || current_user_level >= 5 {
                file.content = args[1].clone();
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
    if state.current_user.is_none() || !state.users[state.current_user.unwrap()].logged_in {
        println!("Error: Must be logged in");
        return;
    }
    
    if args.is_empty() {
        println!("Usage: deletefile <filename>");
        return;
    }
    
    let current_user_idx = state.current_user.unwrap();
    let current_user_name = state.users[current_user_idx].name.clone();
    let current_user_level = state.users[current_user_idx].permission_level;
    
    for (i, file) in state.files.iter().enumerate() {
        if file.filename == args[0] {
            if file.owner == current_user_name || current_user_level >= 9 {
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
    for file in &state.files {
        println!("  {} (owner: {}, perm: {})", file.filename, file.owner, file.permissions);
    }
}

fn cmd_set(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        println!("Usage: set <name> <value>");
        return;
    }
    
    for var in &mut state.variables {
        if var.name == args[0] {
            var.value = args[1].clone();
            println!("Variable '{}' updated", args[0]);
            return;
        }
    }
    
    if state.variables.len() >= MAX_VARIABLES {
        println!("Error: Maximum variables reached");
        return;
    }
    
    let var = Variable {
        name: args[0].clone(),
        value: args[1].clone(),
    };
    state.variables.push(var);
    println!("Variable '{}' set", args[0]);
}

fn cmd_get(state: &State, args: &[String]) {
    if args.is_empty() {
        println!("Usage: get <name>");
        return;
    }
    
    for var in &state.variables {
        if var.name == args[0] {
            println!("{} = {}", var.name, var.value);
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
    
    for (i, var) in state.variables.iter().enumerate() {
        if var.name == args[0] {
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
    for var in &state.variables {
        println!("  {} = {}", var.name, var.value);
    }
}

fn cmd_compare(args: &[String]) {
    if args.len() < 2 {
        println!("Usage: compare <string1> <string2>");
        return;
    }
    
    let result = args[0].cmp(&args[1]);
    
    println!("strcmp('{}', '{}') = {:?}", args[0], args[1], result);
    
    match result {
        std::cmp::Ordering::Equal => println!("Strings are equal"),
        std::cmp::Ordering::Less => println!("'{}' < '{}'", args[0], args[1]),
        std::cmp::Ordering::Greater => println!("'{}' > '{}'", args[0], args[1]),
    }
}

fn cmd_compare_n(args: &[String]) {
    if args.len() < 3 {
        println!("Usage: compareN <string1> <string2> <n>");
        return;
    }
    
    let n: usize = args[2].parse().unwrap_or(0);
    let s1: String = args[0].chars().take(n).collect();
    let s2: String = args[1].chars().take(n).collect();
    let result = s1.cmp(&s2);
    
    println!("strncmp('{}', '{}', {}) = {:?}", args[0], args[1], n, result);
    
    match result {
        std::cmp::Ordering::Equal => println!("First {} characters are equal", n),
        std::cmp::Ordering::Less => println!("'{}' < '{}' (first {} chars)", args[0], args[1], n),
        std::cmp::Ordering::Greater => println!("'{}' > '{}' (first {} chars)", args[0], args[1], n),
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
    
    let pattern = &args[0];
    println!("Matching pattern '{}':", pattern);
    let mut matches = 0;
    
    for arg in &args[1..] {
        if arg == pattern {
            println!("  '{}' - EXACT MATCH", arg);
            matches += 1;
        } else if arg.contains(pattern) {
            println!("  '{}' - contains pattern", arg);
            matches += 1;
        } else {
            println!("  '{}' - no match", arg);
        }
    }
    
    println!("Total matches: {}", matches);
}

fn cmd_help() {
    println!("");
    println!("=== Command Interpreter Help ===");
    println!("User Management:");
    println!("  adduser <user> <pass> [level] - Add new user");
    println!("  login <user> <pass>            - Login as user");
    println!("  logout                         - Logout current user");
    println!("  whoami                         - Show current user");
    println!("  listusers                      - List all users");
    println!("");
    println!("File Management:");
    println!("  createfile <name> [content]    - Create file");
    println!("  readfile <name>                - Read file");
    println!("  writefile <name> <content>     - Write to file");
    println!("  deletefile <name>              - Delete file");
    println!("  listfiles                      - List all files");
    println!("");
    println!("Variable Management:");
    println!("  set <name> <value>             - Set variable");
    println!("  get <name>                     - Get variable");
    println!("  unset <name>                   - Unset variable");
    println!("  listvars                       - List all variables");
    println!("");
    println!("String Operations:");
    println!("  compare <str1> <str2>          - Compare strings");
    println!("  compareN <str1> <str2> <n>     - Compare first N chars");
    println!("  startswith <str> <prefix>      - Check if starts with");
    println!("  match <pattern> <str> ...      - Match pattern");
    println!("");
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
    
    match args[0].as_str() {
        "on" => {
            state.debug_mode = true;
            println!("Debug mode enabled");
        }
        "off" => {
            state.debug_mode = false;
            println!("Debug mode disabled");
        }
        _ => println!("Usage: debug [on|off]"),
    }
}

fn cmd_verbose(state: &mut State, args: &[String]) {
    if args.is_empty() {
        println!("Verbose mode: {}", if state.verbose_mode { "ON" } else { "OFF" });
        return;
    }
    
    match args[0].as_str() {
        "on" => {
            state.verbose_mode = true;
            println!("Verbose mode enabled");
        }
        "off" => {
            state.verbose_mode = false;
            println!("Verbose mode disabled");
        }
        _ => println!("Usage: verbose [on|off]"),
    }
}

fn cmd_status(state: &State) {
    let current_user_name = state.current_user
        .filter(|&idx| state.users[idx].logged_in)
        .map(|idx| state.users[idx].name.as_str())
        .unwrap_or("none");
    
    println!("");
    println!("=== System Status ===");
    println!("Users: {}/{}", state.users.len(), MAX_USERS);
    println!("Files: {}/{}", state.files.len(), MAX_FILES);
    println!("Variables: {}/{}", state.variables.len(), MAX_VARIABLES);
    println!("Current user: {}", current_user_name);
    println!("Debug mode: {}", if state.debug_mode { "ON" } else { "OFF" });
    println!("Verbose mode: {}", if state.verbose_mode { "ON" } else { "OFF" });
}

fn cmd_time() {
    let now = SystemTime::now();
    let datetime = now.duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let secs = datetime.as_secs();
    let time_str = format!("{}", secs);
    println!("Current time: {}", time_str);
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
            std::process::exit(0);
        }
        _ => {
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
    println!("Type 'help' for available commands\n");
    
    let mut state = State::new();
    let mut input = String::new();
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        input.clear();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        
        let input = input.trim();
        
        if state.verbose_mode {
            println!("[VERBOSE] Processing: '{}'", input);
        }
        
        process_command(&mut state, input);
    }
}
