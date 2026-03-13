use std::io::{self, BufRead, Write};
use std::process;

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
    current_user: Option<usize>,
    files: Vec<File>,
    variables: Vec<Variable>,
    debug_mode: bool,
    verbose_mode: bool,
}

/// Replicate C strcmp: return difference of first differing unsigned bytes, or 0
fn c_strcmp(a: &str, b: &str) -> i32 {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let len = a.len().min(b.len());
    for i in 0..len {
        if a[i] != b[i] {
            return a[i] as i32 - b[i] as i32;
        }
    }
    if a.len() < b.len() {
        -(b[a.len()] as i32)
    } else if a.len() > b.len() {
        a[b.len()] as i32
    } else {
        0
    }
}

/// Replicate C strncmp
fn c_strncmp(a: &str, b: &str, n: usize) -> i32 {
    let a = a.as_bytes();
    let b = b.as_bytes();
    for i in 0..n {
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            break;
        }
    }
    0
}

/// Truncate string to max len (like strncpy + null term at max-1)
fn trunc(s: &str, max: usize) -> String {
    if s.len() > max {
        s[..max].to_string()
    } else {
        s.to_string()
    }
}

fn parse_command(input: &str) -> (String, Vec<String>) {
    let mut args: Vec<String> = Vec::new();
    let mut cmd = String::new();
    let mut tokens = input.split(&[' ', '\t'][..]).filter(|s| !s.is_empty());
    if let Some(t) = tokens.next() {
        cmd = trunc(t, MAX_COMMAND - 1);
        for t in tokens {
            if args.len() >= MAX_ARGS {
                break;
            }
            args.push(trunc(t, MAX_COMMAND - 1));
        }
    }
    (cmd, args)
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
        if let Some(idx) = self.current_user {
            self.users[idx].logged_in
        } else {
            false
        }
    }

    fn cmd_adduser(&mut self, args: &[String]) {
        if args.len() < 2 {
            println!("Usage: adduser <username> <password> [permission_level]");
            return;
        }
        if self.users.len() >= MAX_USERS {
            println!("Error: Maximum users reached");
            return;
        }
        for u in &self.users {
            if u.name == args[0] {
                println!("Error: User '{}' already exists", args[0]);
                return;
            }
        }
        let perm = if args.len() >= 3 {
            c_atoi(&args[2])
        } else {
            1
        };
        self.users.push(User {
            name: args[0].clone(),
            password: args[1].clone(),
            permission_level: perm,
            logged_in: false,
        });
        println!("User '{}' added with permission level {}", args[0], perm);
    }

    fn cmd_login(&mut self, args: &[String]) {
        if args.len() < 2 {
            println!("Usage: login <username> <password>");
            return;
        }
        if let Some(idx) = self.current_user {
            if self.users[idx].logged_in {
                println!(
                    "Error: User '{}' already logged in. Use 'logout' first.",
                    self.users[idx].name
                );
                return;
            }
        }
        for i in 0..self.users.len() {
            if self.users[i].name == args[0] {
                if self.users[i].password == args[1] {
                    self.users[i].logged_in = true;
                    self.current_user = Some(i);
                    println!("Login successful. Welcome, {}!", self.users[i].name);
                    return;
                } else {
                    println!("Error: Incorrect password");
                    return;
                }
            }
        }
        println!("Error: User not found");
    }

    fn cmd_logout(&mut self) {
        if !self.current_logged_in() {
            println!("Error: No user logged in");
            return;
        }
        let idx = self.current_user.unwrap();
        println!("Goodbye, {}!", self.users[idx].name);
        self.users[idx].logged_in = false;
        self.current_user = None;
    }

    fn cmd_whoami(&self) {
        if !self.current_logged_in() {
            println!("Not logged in");
            return;
        }
        let idx = self.current_user.unwrap();
        println!("Current user: {}", self.users[idx].name);
        println!("Permission level: {}", self.users[idx].permission_level);
    }

    fn cmd_listusers(&self) {
        if self.users.is_empty() {
            println!("No users registered");
            return;
        }
        println!("Registered users:");
        for u in &self.users {
            println!(
                "  {} (level {}) {}",
                u.name,
                u.permission_level,
                if u.logged_in { "[logged in]" } else { "" }
            );
        }
    }

    fn cmd_createfile(&mut self, args: &[String]) {
        if !self.current_logged_in() {
            println!("Error: Must be logged in");
            return;
        }
        if args.is_empty() {
            println!("Usage: createfile <filename> [content]");
            return;
        }
        if self.files.len() >= MAX_FILES {
            println!("Error: Maximum files reached");
            return;
        }
        for f in &self.files {
            if f.filename == args[0] {
                println!("Error: File '{}' already exists", args[0]);
                return;
            }
        }
        let owner = self.users[self.current_user.unwrap()].name.clone();
        let content = if args.len() >= 2 {
            args[1].clone()
        } else {
            String::new()
        };
        self.files.push(File {
            filename: args[0].clone(),
            content,
            owner,
            permissions: 755,
        });
        println!("File '{}' created", args[0]);
    }

    fn cmd_readfile(&self, args: &[String]) {
        if args.is_empty() {
            println!("Usage: readfile <filename>");
            return;
        }
        for f in &self.files {
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

    fn cmd_writefile(&mut self, args: &[String]) {
        if !self.current_logged_in() {
            println!("Error: Must be logged in");
            return;
        }
        if args.len() < 2 {
            println!("Usage: writefile <filename> <content>");
            return;
        }
        let cur_idx = self.current_user.unwrap();
        let cur_name = self.users[cur_idx].name.clone();
        let cur_perm = self.users[cur_idx].permission_level;
        for f in &mut self.files {
            if f.filename == args[0] {
                if f.owner == cur_name || cur_perm >= 5 {
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

    fn cmd_deletefile(&mut self, args: &[String]) {
        if !self.current_logged_in() {
            println!("Error: Must be logged in");
            return;
        }
        if args.is_empty() {
            println!("Usage: deletefile <filename>");
            return;
        }
        let cur_idx = self.current_user.unwrap();
        let cur_name = self.users[cur_idx].name.clone();
        let cur_perm = self.users[cur_idx].permission_level;
        let mut found = None;
        for (i, f) in self.files.iter().enumerate() {
            if f.filename == args[0] {
                if f.owner == cur_name || cur_perm >= 9 {
                    found = Some(i);
                } else {
                    println!("Error: Permission denied");
                    return;
                }
                break;
            }
        }
        if let Some(i) = found {
            self.files.remove(i);
            println!("File '{}' deleted", args[0]);
        } else {
            println!("Error: File '{}' not found", args[0]);
        }
    }

    fn cmd_listfiles(&self) {
        if self.files.is_empty() {
            println!("No files");
            return;
        }
        println!("Files:");
        for f in &self.files {
            println!("  {} (owner: {}, perm: {})", f.filename, f.owner, f.permissions);
        }
    }

    fn cmd_set(&mut self, args: &[String]) {
        if args.len() < 2 {
            println!("Usage: set <name> <value>");
            return;
        }
        for v in &mut self.variables {
            if v.name == args[0] {
                v.value = args[1].clone();
                println!("Variable '{}' updated", args[0]);
                return;
            }
        }
        if self.variables.len() >= MAX_VARIABLES {
            println!("Error: Maximum variables reached");
            return;
        }
        self.variables.push(Variable {
            name: args[0].clone(),
            value: args[1].clone(),
        });
        println!("Variable '{}' set", args[0]);
    }

    fn cmd_get(&self, args: &[String]) {
        if args.is_empty() {
            println!("Usage: get <name>");
            return;
        }
        for v in &self.variables {
            if v.name == args[0] {
                println!("{} = {}", v.name, v.value);
                return;
            }
        }
        println!("Error: Variable '{}' not found", args[0]);
    }

    fn cmd_unset(&mut self, args: &[String]) {
        if args.is_empty() {
            println!("Usage: unset <name>");
            return;
        }
        for i in 0..self.variables.len() {
            if self.variables[i].name == args[0] {
                self.variables.remove(i);
                println!("Variable '{}' unset", args[0]);
                return;
            }
        }
        println!("Error: Variable '{}' not found", args[0]);
    }

    fn cmd_listvars(&self) {
        if self.variables.is_empty() {
            println!("No variables set");
            return;
        }
        println!("Variables:");
        for v in &self.variables {
            println!("  {} = {}", v.name, v.value);
        }
    }

    fn cmd_compare(&self, args: &[String]) {
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

    fn cmd_comparen(&self, args: &[String]) {
        if args.len() < 3 {
            println!("Usage: compareN <string1> <string2> <n>");
            return;
        }
        let n = c_atoi(&args[2]) as usize;
        let result = c_strncmp(&args[0], &args[1], n);
        println!("strncmp('{}', '{}', {}) = {}", args[0], args[1], n, result);
        if result == 0 {
            println!("First {} characters are equal", n);
        } else if result < 0 {
            println!("'{}' < '{}' (first {} chars)", args[0], args[1], n);
        } else {
            println!("'{}' > '{}' (first {} chars)", args[0], args[1], n);
        }
    }

    fn cmd_startswith(&self, args: &[String]) {
        if args.len() < 2 {
            println!("Usage: startswith <string> <prefix>");
            return;
        }
        let prefix_len = args[1].len();
        if c_strncmp(&args[0], &args[1], prefix_len) == 0 {
            println!("'{}' starts with '{}'", args[0], args[1]);
        } else {
            println!("'{}' does not start with '{}'", args[0], args[1]);
        }
    }

    fn cmd_match(&self, args: &[String]) {
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

    fn cmd_help(&self) {
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

    fn cmd_debug(&mut self, args: &[String]) {
        if args.is_empty() {
            println!("Debug mode: {}", if self.debug_mode { "ON" } else { "OFF" });
            return;
        }
        if args[0] == "on" {
            self.debug_mode = true;
            println!("Debug mode enabled");
        } else if args[0] == "off" {
            self.debug_mode = false;
            println!("Debug mode disabled");
        } else {
            println!("Usage: debug [on|off]");
        }
    }

    fn cmd_verbose(&mut self, args: &[String]) {
        if args.is_empty() {
            println!("Verbose mode: {}", if self.verbose_mode { "ON" } else { "OFF" });
            return;
        }
        if args[0] == "on" {
            self.verbose_mode = true;
            println!("Verbose mode enabled");
        } else if args[0] == "off" {
            self.verbose_mode = false;
            println!("Verbose mode disabled");
        } else {
            println!("Usage: verbose [on|off]");
        }
    }

    fn cmd_status(&self) {
        println!();
        println!("=== System Status ===");
        println!("Users: {}/{}", self.users.len(), MAX_USERS);
        println!("Files: {}/{}", self.files.len(), MAX_FILES);
        println!("Variables: {}/{}", self.variables.len(), MAX_VARIABLES);
        let cur = if self.current_logged_in() {
            self.users[self.current_user.unwrap()].name.as_str()
        } else {
            "none"
        };
        println!("Current user: {}", cur);
        println!("Debug mode: {}", if self.debug_mode { "ON" } else { "OFF" });
        println!("Verbose mode: {}", if self.verbose_mode { "ON" } else { "OFF" });
    }

    fn cmd_time(&self) {
        // Match C's ctime() format: "Day Mon DD HH:MM:SS YYYY\n"
        let now = unsafe { libc::time(std::ptr::null_mut()) };
        let tm = unsafe { *libc::localtime(&now) };
        let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                       "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        print!(
            "Current time: {} {} {:2} {:02}:{:02}:{:02} {}\n",
            days[tm.tm_wday as usize],
            months[tm.tm_mon as usize],
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec,
            tm.tm_year + 1900
        );
    }

    fn process_command(&mut self, input: &str) {
        let (command, args) = parse_command(input);
        if command.is_empty() {
            return;
        }
        if self.debug_mode {
            println!("[DEBUG] Command: '{}', Args: {}", command, args.len());
        }
        let cmd = command.as_str();
        match cmd {
            "adduser" => self.cmd_adduser(&args),
            "login" => self.cmd_login(&args),
            "logout" => self.cmd_logout(),
            "whoami" => self.cmd_whoami(),
            "listusers" | "users" => self.cmd_listusers(),
            "createfile" | "touch" => self.cmd_createfile(&args),
            "readfile" | "cat" => self.cmd_readfile(&args),
            "writefile" | "write" => self.cmd_writefile(&args),
            "deletefile" | "rm" => self.cmd_deletefile(&args),
            "listfiles" | "ls" => self.cmd_listfiles(),
            "set" => self.cmd_set(&args),
            "get" => self.cmd_get(&args),
            "unset" => self.cmd_unset(&args),
            "listvars" | "vars" => self.cmd_listvars(),
            "compare" | "cmp" => self.cmd_compare(&args),
            "compareN" | "cmpn" => self.cmd_comparen(&args),
            "startswith" => self.cmd_startswith(&args),
            "match" => self.cmd_match(&args),
            "debug" => self.cmd_debug(&args),
            "verbose" => self.cmd_verbose(&args),
            "status" => self.cmd_status(),
            "time" => self.cmd_time(),
            "help" | "?" => self.cmd_help(),
            "exit" | "quit" => {
                println!("Goodbye!");
                process::exit(0);
            }
            _ => {
                // Partial matches using strncmp equivalent
                if cmd.starts_with("add") {
                    println!("Did you mean 'adduser'?");
                } else if cmd.starts_with("log") {
                    println!("Did you mean 'login' or 'logout'?");
                } else if cmd.starts_with("list") {
                    println!("Did you mean 'listusers', 'listfiles', or 'listvars'?");
                } else if cmd.starts_with("create") {
                    println!("Did you mean 'createfile'?");
                } else if cmd.starts_with("read") {
                    println!("Did you mean 'readfile'?");
                } else if cmd.starts_with("write") {
                    println!("Did you mean 'writefile'?");
                } else if cmd.starts_with("delete") {
                    println!("Did you mean 'deletefile'?");
                } else {
                    println!("Unknown command: '{}'. Type 'help' for available commands.", cmd);
                }
            }
        }
    }
}

/// Replicate C atoi: parse leading decimal integer, 0 on failure
fn c_atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let neg = if chars.peek() == Some(&'-') {
        chars.next();
        true
    } else {
        if chars.peek() == Some(&'+') {
            chars.next();
        }
        false
    };
    let mut val: i32 = 0;
    for c in chars {
        if c.is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add(c as i32 - '0' as i32);
        } else {
            break;
        }
    }
    if neg { val.wrapping_neg() } else { val }
}

fn main() {
    println!("|----------------------------------------|");
    println!("|   COMMAND INTERPRETER                  |");
    println!("|   strcmp/strncmp demonstration         |");
    println!("|----------------------------------------|");
    println!("Type 'help' for available commands\n");

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
            if input.ends_with('\r') {
                input.pop();
            }
        }

        if state.verbose_mode {
            println!("[VERBOSE] Processing: '{}'", input);
        }

        state.process_command(&input);
    }
}
