use std::io::{self, BufRead, Write};
use std::process;

pub const MAX_COMMAND: usize = 64;
pub const MAX_ARGS: usize = 10;
const MAX_USERS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_VARIABLES: usize = 20;

#[derive(Clone)]
pub struct User {
    name: String,
    password: String,
    permission_level: i32,
    logged_in: bool,
}

#[derive(Clone)]
pub struct File {
    filename: String,
    content: String,
    owner: String,
    permissions: i32,
}

#[derive(Clone)]
pub struct Variable {
    name: String,
    value: String,
}

pub struct State {
    pub users: Vec<User>,
    pub current_user: Option<usize>,
    pub files: Vec<File>,
    pub variables: Vec<Variable>,
    pub debug_mode: bool,
    pub verbose_mode: bool,
}

/// Replicate C strcmp: returns difference of first differing unsigned char values,
/// or 0 if equal. On glibc/Linux this is the exact byte difference.
fn c_strcmp(a: &str, b: &str) -> i32 {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let len = a.len().min(b.len());
    for i in 0..len {
        let diff = (a[i] as i32) - (b[i] as i32);
        if diff != 0 {
            return diff;
        }
    }
    if a.len() < b.len() {
        -(b[len] as i32)
    } else if a.len() > b.len() {
        a[len] as i32
    } else {
        0
    }
}

/// Replicate C strncmp
fn c_strncmp(a: &str, b: &str, n: usize) -> i32 {
    let a = a.as_bytes();
    let b = b.as_bytes();
    for i in 0..n {
        let ca = if i < a.len() { a[i] } else { 0u8 };
        let cb = if i < b.len() { b[i] } else { 0u8 };
        let diff = (ca as i32) - (cb as i32);
        if diff != 0 {
            return diff;
        }
        if ca == 0 {
            break;
        }
    }
    0
}

/// Replicate C atoi: parse leading integer, return 0 on failure
fn c_atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut neg = false;
    let mut chars = s.chars().peekable();
    if chars.peek() == Some(&'-') {
        neg = true;
        chars.next();
    } else if chars.peek() == Some(&'+') {
        chars.next();
    }
    let mut val: i32 = 0;
    for c in chars {
        if c.is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add((c as i32) - ('0' as i32));
        } else {
            break;
        }
    }
    if neg { -val } else { val }
}

/// Truncate string to at most max_len bytes (like strncpy destination size - 1)
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        s[..max_len].to_string()
    }
}

/// Parse command and arguments, replicating strtok with " \t" delimiters
/// and strncpy truncation to MAX_COMMAND-1
pub fn parse_command(input: &str) -> (String, Vec<String>) {
    let max = MAX_COMMAND - 1; // 63
    let mut args = Vec::new();
    let mut tokens = input.split(|c: char| c == ' ' || c == '\t').filter(|s| !s.is_empty());
    let cmd = match tokens.next() {
        Some(t) => truncate(t, max),
        None => return (String::new(), args),
    };
    for t in tokens {
        if args.len() >= MAX_ARGS {
            break;
        }
        args.push(truncate(t, max));
    }
    (cmd, args)
}

impl State {
    pub fn new() -> Self {
        State {
            users: Vec::new(),
            current_user: None,
            files: Vec::new(),
            variables: Vec::new(),
            debug_mode: false,
            verbose_mode: false,
        }
    }

    pub fn cmd_adduser(&mut self, args: &[String]) {
        if args.len() < 2 {
            print!("Usage: adduser <username> <password> [permission_level]\n");
            return;
        }
        if self.users.len() >= MAX_USERS {
            print!("Error: Maximum users reached\n");
            return;
        }
        for u in &self.users {
            if u.name == args[0] {
                print!("Error: User '{}' already exists\n", args[0]);
                return;
            }
        }
        let perm = if args.len() >= 3 { c_atoi(&args[2]) } else { 1 };
        self.users.push(User {
            name: args[0].clone(),
            password: args[1].clone(),
            permission_level: perm,
            logged_in: false,
        });
        print!("User '{}' added with permission level {}\n", args[0], perm);
    }

    pub fn cmd_login(&mut self, args: &[String]) {
        if args.len() < 2 {
            print!("Usage: login <username> <password>\n");
            return;
        }
        if let Some(idx) = self.current_user {
            if self.users[idx].logged_in {
                print!("Error: User '{}' already logged in. Use 'logout' first.\n", self.users[idx].name);
                return;
            }
        }
        for i in 0..self.users.len() {
            if self.users[i].name == args[0] {
                if self.users[i].password == args[1] {
                    self.users[i].logged_in = true;
                    self.current_user = Some(i);
                    print!("Login successful. Welcome, {}!\n", self.users[i].name);
                    return;
                } else {
                    print!("Error: Incorrect password\n");
                    return;
                }
            }
        }
        print!("Error: User not found\n");
    }

    pub fn cmd_logout(&mut self) {
        match self.current_user {
            Some(idx) if self.users[idx].logged_in => {
                print!("Goodbye, {}!\n", self.users[idx].name);
                self.users[idx].logged_in = false;
                self.current_user = None;
            }
            _ => {
                print!("Error: No user logged in\n");
            }
        }
    }

    pub fn cmd_whoami(&self) {
        match self.current_user {
            Some(idx) if self.users[idx].logged_in => {
                print!("Current user: {}\n", self.users[idx].name);
                print!("Permission level: {}\n", self.users[idx].permission_level);
            }
            _ => {
                print!("Not logged in\n");
            }
        }
    }

    pub fn cmd_listusers(&self) {
        if self.users.is_empty() {
            print!("No users registered\n");
            return;
        }
        print!("Registered users:\n");
        for u in &self.users {
            print!("  {} (level {}) {}\n", u.name, u.permission_level,
                   if u.logged_in { "[logged in]" } else { "" });
        }
    }

    pub fn is_logged_in(&self) -> bool {
        matches!(self.current_user, Some(idx) if self.users[idx].logged_in)
    }

    pub fn current_name(&self) -> &str {
        &self.users[self.current_user.unwrap()].name
    }

    pub fn current_perm(&self) -> i32 {
        self.users[self.current_user.unwrap()].permission_level
    }

    pub fn cmd_createfile(&mut self, args: &[String]) {
        if !self.is_logged_in() {
            print!("Error: Must be logged in\n");
            return;
        }
        if args.is_empty() {
            print!("Usage: createfile <filename> [content]\n");
            return;
        }
        if self.files.len() >= MAX_FILES {
            print!("Error: Maximum files reached\n");
            return;
        }
        for f in &self.files {
            if f.filename == args[0] {
                print!("Error: File '{}' already exists\n", args[0]);
                return;
            }
        }
        let content = if args.len() >= 2 { args[1].clone() } else { String::new() };
        let owner = self.current_name().to_string();
        self.files.push(File {
            filename: args[0].clone(),
            content,
            owner,
            permissions: 755,
        });
        print!("File '{}' created\n", args[0]);
    }

    pub fn cmd_readfile(&self, args: &[String]) {
        if args.is_empty() {
            print!("Usage: readfile <filename>\n");
            return;
        }
        for f in &self.files {
            if f.filename == args[0] {
                print!("=== {} ===\n", f.filename);
                print!("Owner: {}\n", f.owner);
                print!("Permissions: {}\n", f.permissions);
                print!("Content: {}\n", f.content);
                return;
            }
        }
        print!("Error: File '{}' not found\n", args[0]);
    }

    pub fn cmd_writefile(&mut self, args: &[String]) {
        if !self.is_logged_in() {
            print!("Error: Must be logged in\n");
            return;
        }
        if args.len() < 2 {
            print!("Usage: writefile <filename> <content>\n");
            return;
        }
        let cur_name = self.current_name().to_string();
        let cur_perm = self.current_perm();
        for f in &mut self.files {
            if f.filename == args[0] {
                if f.owner == cur_name || cur_perm >= 5 {
                    f.content = args[1].clone();
                    print!("File '{}' updated\n", args[0]);
                    return;
                } else {
                    print!("Error: Permission denied\n");
                    return;
                }
            }
        }
        print!("Error: File '{}' not found\n", args[0]);
    }

    pub fn cmd_deletefile(&mut self, args: &[String]) {
        if !self.is_logged_in() {
            print!("Error: Must be logged in\n");
            return;
        }
        if args.is_empty() {
            print!("Usage: deletefile <filename>\n");
            return;
        }
        let cur_name = self.current_name().to_string();
        let cur_perm = self.current_perm();
        for i in 0..self.files.len() {
            if self.files[i].filename == args[0] {
                if self.files[i].owner == cur_name || cur_perm >= 9 {
                    self.files.remove(i);
                    print!("File '{}' deleted\n", args[0]);
                    return;
                } else {
                    print!("Error: Permission denied\n");
                    return;
                }
            }
        }
        print!("Error: File '{}' not found\n", args[0]);
    }

    pub fn cmd_listfiles(&self) {
        if self.files.is_empty() {
            print!("No files\n");
            return;
        }
        print!("Files:\n");
        for f in &self.files {
            print!("  {} (owner: {}, perm: {})\n", f.filename, f.owner, f.permissions);
        }
    }

    pub fn cmd_set(&mut self, args: &[String]) {
        if args.len() < 2 {
            print!("Usage: set <name> <value>\n");
            return;
        }
        for v in &mut self.variables {
            if v.name == args[0] {
                v.value = args[1].clone();
                print!("Variable '{}' updated\n", args[0]);
                return;
            }
        }
        if self.variables.len() >= MAX_VARIABLES {
            print!("Error: Maximum variables reached\n");
            return;
        }
        self.variables.push(Variable {
            name: args[0].clone(),
            value: args[1].clone(),
        });
        print!("Variable '{}' set\n", args[0]);
    }

    pub fn cmd_get(&self, args: &[String]) {
        if args.is_empty() {
            print!("Usage: get <name>\n");
            return;
        }
        for v in &self.variables {
            if v.name == args[0] {
                print!("{} = {}\n", v.name, v.value);
                return;
            }
        }
        print!("Error: Variable '{}' not found\n", args[0]);
    }

    pub fn cmd_unset(&mut self, args: &[String]) {
        if args.is_empty() {
            print!("Usage: unset <name>\n");
            return;
        }
        for i in 0..self.variables.len() {
            if self.variables[i].name == args[0] {
                self.variables.remove(i);
                print!("Variable '{}' unset\n", args[0]);
                return;
            }
        }
        print!("Error: Variable '{}' not found\n", args[0]);
    }

    pub fn cmd_listvars(&self) {
        if self.variables.is_empty() {
            print!("No variables set\n");
            return;
        }
        print!("Variables:\n");
        for v in &self.variables {
            print!("  {} = {}\n", v.name, v.value);
        }
    }

    pub fn cmd_compare(&self, args: &[String]) {
        if args.len() < 2 {
            print!("Usage: compare <string1> <string2>\n");
            return;
        }
        let result = c_strcmp(&args[0], &args[1]);
        print!("strcmp('{}', '{}') = {}\n", args[0], args[1], result);
        if result == 0 {
            print!("Strings are equal\n");
        } else if result < 0 {
            print!("'{}' < '{}'\n", args[0], args[1]);
        } else {
            print!("'{}' > '{}'\n", args[0], args[1]);
        }
    }

    pub fn cmd_comparen(&self, args: &[String]) {
        if args.len() < 3 {
            print!("Usage: compareN <string1> <string2> <n>\n");
            return;
        }
        let n = c_atoi(&args[2]) as usize;
        let result = c_strncmp(&args[0], &args[1], n);
        print!("strncmp('{}', '{}', {}) = {}\n", args[0], args[1], n, result);
        if result == 0 {
            print!("First {} characters are equal\n", n);
        } else if result < 0 {
            print!("'{}' < '{}' (first {} chars)\n", args[0], args[1], n);
        } else {
            print!("'{}' > '{}' (first {} chars)\n", args[0], args[1], n);
        }
    }

    pub fn cmd_startswith(&self, args: &[String]) {
        if args.len() < 2 {
            print!("Usage: startswith <string> <prefix>\n");
            return;
        }
        let prefix_len = args[1].len();
        if c_strncmp(&args[0], &args[1], prefix_len) == 0 {
            print!("'{}' starts with '{}'\n", args[0], args[1]);
        } else {
            print!("'{}' does not start with '{}'\n", args[0], args[1]);
        }
    }

    pub fn cmd_match(&self, args: &[String]) {
        if args.len() < 2 {
            print!("Usage: match <pattern> <string1> [string2] ...\n");
            return;
        }
        print!("Matching pattern '{}':\n", args[0]);
        let mut matches = 0;
        for i in 1..args.len() {
            if c_strcmp(&args[0], &args[i]) == 0 {
                print!("  '{}' - EXACT MATCH\n", args[i]);
                matches += 1;
            } else if args[i].contains(&*args[0]) {
                print!("  '{}' - contains pattern\n", args[i]);
                matches += 1;
            } else {
                print!("  '{}' - no match\n", args[i]);
            }
        }
        print!("Total matches: {}\n", matches);
    }

    pub fn cmd_help(&self) {
        print!("\n=== Command Interpreter Help ===\n");
        print!("User Management:\n");
        print!("  adduser <user> <pass> [level] - Add new user\n");
        print!("  login <user> <pass>            - Login as user\n");
        print!("  logout                         - Logout current user\n");
        print!("  whoami                         - Show current user\n");
        print!("  listusers                      - List all users\n");
        print!("\nFile Management:\n");
        print!("  createfile <name> [content]    - Create file\n");
        print!("  readfile <name>                - Read file\n");
        print!("  writefile <name> <content>     - Write to file\n");
        print!("  deletefile <name>              - Delete file\n");
        print!("  listfiles                      - List all files\n");
        print!("\nVariable Management:\n");
        print!("  set <name> <value>             - Set variable\n");
        print!("  get <name>                     - Get variable\n");
        print!("  unset <name>                   - Unset variable\n");
        print!("  listvars                       - List all variables\n");
        print!("\nString Operations:\n");
        print!("  compare <str1> <str2>          - Compare strings\n");
        print!("  compareN <str1> <str2> <n>     - Compare first N chars\n");
        print!("  startswith <str> <prefix>      - Check if starts with\n");
        print!("  match <pattern> <str> ...      - Match pattern\n");
        print!("\nSystem:\n");
        print!("  debug [on|off]                 - Toggle debug mode\n");
        print!("  verbose [on|off]               - Toggle verbose mode\n");
        print!("  status                         - Show system status\n");
        print!("  time                           - Show current time\n");
        print!("  help                           - Show this help\n");
        print!("  exit                           - Exit program\n");
    }

    pub fn cmd_debug(&mut self, args: &[String]) {
        if args.is_empty() {
            print!("Debug mode: {}\n", if self.debug_mode { "ON" } else { "OFF" });
            return;
        }
        if args[0] == "on" {
            self.debug_mode = true;
            print!("Debug mode enabled\n");
        } else if args[0] == "off" {
            self.debug_mode = false;
            print!("Debug mode disabled\n");
        } else {
            print!("Usage: debug [on|off]\n");
        }
    }

    pub fn cmd_verbose(&mut self, args: &[String]) {
        if args.is_empty() {
            print!("Verbose mode: {}\n", if self.verbose_mode { "ON" } else { "OFF" });
            return;
        }
        if args[0] == "on" {
            self.verbose_mode = true;
            print!("Verbose mode enabled\n");
        } else if args[0] == "off" {
            self.verbose_mode = false;
            print!("Verbose mode disabled\n");
        } else {
            print!("Usage: verbose [on|off]\n");
        }
    }

    pub fn cmd_status(&self) {
        let cur = match self.current_user {
            Some(idx) if self.users[idx].logged_in => &self.users[idx].name as &str,
            _ => "none",
        };
        print!("\n=== System Status ===\n");
        print!("Users: {}/{}\n", self.users.len(), MAX_USERS);
        print!("Files: {}/{}\n", self.files.len(), MAX_FILES);
        print!("Variables: {}/{}\n", self.variables.len(), MAX_VARIABLES);
        print!("Current user: {}\n", cur);
        print!("Debug mode: {}\n", if self.debug_mode { "ON" } else { "OFF" });
        print!("Verbose mode: {}\n", if self.verbose_mode { "ON" } else { "OFF" });
    }

    pub fn cmd_time(&self) {
        extern "C" {
            fn ctime(timep: *const libc::time_t) -> *const libc::c_char;
        }
        unsafe {
            let mut now: libc::time_t = 0;
            libc::time(&mut now);
            let ct = ctime(&now);
            let s = std::ffi::CStr::from_ptr(ct);
            // ctime returns "Day Mon DD HH:MM:SS YYYY\n" — already has newline
            print!("Current time: {}", s.to_str().unwrap_or(""));
        }
    }

    pub fn process_command(&mut self, input: &str) {
        let (command, args) = parse_command(input);
        if command.is_empty() {
            return;
        }
        if self.debug_mode {
            print!("[DEBUG] Command: '{}', Args: {}\n", command, args.len());
        }

        let cmd = command.as_str();
        if cmd == "adduser" {
            self.cmd_adduser(&args);
        } else if cmd == "login" {
            self.cmd_login(&args);
        } else if cmd == "logout" {
            self.cmd_logout();
        } else if cmd == "whoami" {
            self.cmd_whoami();
        } else if cmd == "listusers" || cmd == "users" {
            self.cmd_listusers();
        } else if cmd == "createfile" || cmd == "touch" {
            self.cmd_createfile(&args);
        } else if cmd == "readfile" || cmd == "cat" {
            self.cmd_readfile(&args);
        } else if cmd == "writefile" || cmd == "write" {
            self.cmd_writefile(&args);
        } else if cmd == "deletefile" || cmd == "rm" {
            self.cmd_deletefile(&args);
        } else if cmd == "listfiles" || cmd == "ls" {
            self.cmd_listfiles();
        } else if cmd == "set" {
            self.cmd_set(&args);
        } else if cmd == "get" {
            self.cmd_get(&args);
        } else if cmd == "unset" {
            self.cmd_unset(&args);
        } else if cmd == "listvars" || cmd == "vars" {
            self.cmd_listvars();
        } else if cmd == "compare" || cmd == "cmp" {
            self.cmd_compare(&args);
        } else if cmd == "compareN" || cmd == "cmpn" {
            self.cmd_comparen(&args);
        } else if cmd == "startswith" {
            self.cmd_startswith(&args);
        } else if cmd == "match" {
            self.cmd_match(&args);
        } else if cmd == "debug" {
            self.cmd_debug(&args);
        } else if cmd == "verbose" {
            self.cmd_verbose(&args);
        } else if cmd == "status" {
            self.cmd_status();
        } else if cmd == "time" {
            self.cmd_time();
        } else if cmd == "help" || cmd == "?" {
            self.cmd_help();
        } else if cmd == "exit" || cmd == "quit" {
            print!("Goodbye!\n");
            let _ = io::stdout().flush();
            process::exit(0);
        } else if command.starts_with("add") {
            print!("Did you mean 'adduser'?\n");
        } else if command.starts_with("log") {
            print!("Did you mean 'login' or 'logout'?\n");
        } else if command.starts_with("list") {
            print!("Did you mean 'listusers', 'listfiles', or 'listvars'?\n");
        } else if command.starts_with("create") {
            print!("Did you mean 'createfile'?\n");
        } else if command.starts_with("read") {
            print!("Did you mean 'readfile'?\n");
        } else if command.starts_with("write") {
            print!("Did you mean 'writefile'?\n");
        } else if command.starts_with("delete") {
            print!("Did you mean 'deletefile'?\n");
        } else {
            print!("Unknown command: '{}'. Type 'help' for available commands.\n", command);
        }
    }
}

fn main() {
    print!("|----------------------------------------|\n");
    print!("|   COMMAND INTERPRETER                  |\n");
    print!("|   strcmp/strncmp demonstration         |\n");
    print!("|----------------------------------------|\n");
    print!("Type 'help' for available commands\n\n");

    let mut state = State::new();
    let stdin = io::stdin();

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) | Err(_) => break,
            _ => {}
        }

        // Remove trailing newline (like input[strcspn(input, "\n")] = 0)
        if input.ends_with('\n') {
            input.pop();
        }

        if state.verbose_mode {
            print!("[VERBOSE] Processing: '{}'\n", input);
        }

        state.process_command(&input);
    }
}
