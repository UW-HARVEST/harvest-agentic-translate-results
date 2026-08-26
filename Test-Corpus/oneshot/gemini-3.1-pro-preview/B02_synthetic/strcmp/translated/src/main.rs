use std::io::{self, Write};

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
    current_user_index: Option<usize>,
    files: Vec<File>,
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

    fn cmd_adduser(&mut self, args: &[&str]) {
        if args.len() < 2 {
            println!("Usage: adduser <username> <password> [permission_level]");
            return;
        }
        if self.users.len() >= MAX_USERS {
            println!("Error: Maximum users reached");
            return;
        }
        if self.users.iter().any(|u| u.name == args[0]) {
            println!("Error: User '{}' already exists", args[0]);
            return;
        }
        let permission_level = if args.len() >= 3 {
            args[2].parse().unwrap_or(1)
        } else {
            1
        };
        self.users.push(User {
            name: args[0].to_string(),
            password: args[1].to_string(),
            permission_level,
            logged_in: false,
        });
        println!("User '{}' added with permission level {}", args[0], permission_level);
    }

    fn cmd_login(&mut self, args: &[&str]) {
        if args.len() < 2 {
            println!("Usage: login <username> <password>");
            return;
        }
        if let Some(idx) = self.current_user_index {
            if self.users[idx].logged_in {
                println!("Error: User '{}' already logged in. Use 'logout' first.", self.users[idx].name);
                return;
            }
        }
        for (i, user) in self.users.iter_mut().enumerate() {
            if user.name == args[0] {
                if user.password == args[1] {
                    user.logged_in = true;
                    self.current_user_index = Some(i);
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

    fn cmd_logout(&mut self) {
        if let Some(idx) = self.current_user_index {
            if self.users[idx].logged_in {
                println!("Goodbye, {}!", self.users[idx].name);
                self.users[idx].logged_in = false;
                self.current_user_index = None;
                return;
            }
        }
        println!("Error: No user logged in");
    }

    fn cmd_whoami(&self) {
        if let Some(idx) = self.current_user_index {
            if self.users[idx].logged_in {
                println!("Current user: {}", self.users[idx].name);
                println!("Permission level: {}", self.users[idx].permission_level);
                return;
            }
        }
        println!("Not logged in");
    }

    fn cmd_listusers(&self) {
        if self.users.is_empty() {
            println!("No users registered");
            return;
        }
        println!("Registered users:");
        for user in &self.users {
            println!("  {} (level {}) {}", user.name, user.permission_level, if user.logged_in { "[logged in]" } else { "" });
        }
    }

    fn cmd_createfile(&mut self, args: &[&str]) {
        let current_user = match self.current_user_index {
            Some(idx) if self.users[idx].logged_in => &self.users[idx],
            _ => {
                println!("Error: Must be logged in");
                return;
            }
        };
        if args.is_empty() {
            println!("Usage: createfile <filename> [content]");
            return;
        }
        if self.files.len() >= MAX_FILES {
            println!("Error: Maximum files reached");
            return;
        }
        if self.files.iter().any(|f| f.filename == args[0]) {
            println!("Error: File '{}' already exists", args[0]);
            return;
        }
        let content = if args.len() >= 2 { args[1].to_string() } else { String::new() };
        self.files.push(File {
            filename: args[0].to_string(),
            content,
            owner: current_user.name.clone(),
            permissions: 755,
        });
        println!("File '{}' created", args[0]);
    }

    fn cmd_readfile(&self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: readfile <filename>");
            return;
        }
        for file in &self.files {
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

    fn cmd_writefile(&mut self, args: &[&str]) {
        let current_user = match self.current_user_index {
            Some(idx) if self.users[idx].logged_in => &self.users[idx],
            _ => {
                println!("Error: Must be logged in");
                return;
            }
        };
        if args.len() < 2 {
            println!("Usage: writefile <filename> <content>");
            return;
        }
        let current_user_name = current_user.name.clone();
        let current_user_perm = current_user.permission_level;
        
        for file in &mut self.files {
            if file.filename == args[0] {
                if file.owner == current_user_name || current_user_perm >= 5 {
                    file.content = args[1].to_string();
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

    fn cmd_deletefile(&mut self, args: &[&str]) {
        let current_user = match self.current_user_index {
            Some(idx) if self.users[idx].logged_in => &self.users[idx],
            _ => {
                println!("Error: Must be logged in");
                return;
            }
        };
        if args.is_empty() {
            println!("Usage: deletefile <filename>");
            return;
        }
        let current_user_name = current_user.name.clone();
        let current_user_perm = current_user.permission_level;
        
        if let Some(pos) = self.files.iter().position(|f| f.filename == args[0]) {
            if self.files[pos].owner == current_user_name || current_user_perm >= 9 {
                self.files.remove(pos);
                println!("File '{}' deleted", args[0]);
            } else {
                println!("Error: Permission denied");
            }
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
        for file in &self.files {
            println!("  {} (owner: {}, perm: {})", file.filename, file.owner, file.permissions);
        }
    }

    fn cmd_set(&mut self, args: &[&str]) {
        if args.len() < 2 {
            println!("Usage: set <name> <value>");
            return;
        }
        for var in &mut self.variables {
            if var.name == args[0] {
                var.value = args[1].to_string();
                println!("Variable '{}' updated", args[0]);
                return;
            }
        }
        if self.variables.len() >= MAX_VARIABLES {
            println!("Error: Maximum variables reached");
            return;
        }
        self.variables.push(Variable {
            name: args[0].to_string(),
            value: args[1].to_string(),
        });
        println!("Variable '{}' set", args[0]);
    }

    fn cmd_get(&self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: get <name>");
            return;
        }
        for var in &self.variables {
            if var.name == args[0] {
                println!("{} = {}", var.name, var.value);
                return;
            }
        }
        println!("Error: Variable '{}' not found", args[0]);
    }

    fn cmd_unset(&mut self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: unset <name>");
            return;
        }
        if let Some(pos) = self.variables.iter().position(|v| v.name == args[0]) {
            self.variables.remove(pos);
            println!("Variable '{}' unset", args[0]);
        } else {
            println!("Error: Variable '{}' not found", args[0]);
        }
    }

    fn cmd_listvars(&self) {
        if self.variables.is_empty() {
            println!("No variables set");
            return;
        }
        println!("Variables:");
        for var in &self.variables {
            println!("  {} = {}", var.name, var.value);
        }
    }

    fn cmd_compare(&self, args: &[&str]) {
        if args.len() < 2 {
            println!("Usage: compare <string1> <string2>");
            return;
        }
        use std::cmp::Ordering;
        let result = args[0].cmp(args[1]);
        let result_int = match result {
            Ordering::Equal => 0,
            Ordering::Less => -1,
            Ordering::Greater => 1,
        };
        println!("strcmp('{}', '{}') = {}", args[0], args[1], result_int);
        match result {
            Ordering::Equal => println!("Strings are equal"),
            Ordering::Less => println!("'{}' < '{}'", args[0], args[1]),
            Ordering::Greater => println!("'{}' > '{}'", args[0], args[1]),
        }
    }

    fn cmd_compareN(&self, args: &[&str]) {
        if args.len() < 3 {
            println!("Usage: compareN <string1> <string2> <n>");
            return;
        }
        let n: usize = args[2].parse().unwrap_or(0);
        let s1: String = args[0].chars().take(n).collect();
        let s2: String = args[1].chars().take(n).collect();
        
        use std::cmp::Ordering;
        let result = s1.cmp(&s2);
        let result_int = match result {
            Ordering::Equal => 0,
            Ordering::Less => -1,
            Ordering::Greater => 1,
        };
        println!("strncmp('{}', '{}', {}) = {}", args[0], args[1], n, result_int);
        match result {
            Ordering::Equal => println!("First {} characters are equal", n),
            Ordering::Less => println!("'{}' < '{}' (first {} chars)", args[0], args[1], n),
            Ordering::Greater => println!("'{}' > '{}' (first {} chars)", args[0], args[1], n),
        }
    }

    fn cmd_startswith(&self, args: &[&str]) {
        if args.len() < 2 {
            println!("Usage: startswith <string> <prefix>");
            return;
        }
        if args[0].starts_with(args[1]) {
            println!("'{}' starts with '{}'", args[0], args[1]);
        } else {
            println!("'{}' does not start with '{}'", args[0], args[1]);
        }
    }

    fn cmd_match(&self, args: &[&str]) {
        if args.len() < 2 {
            println!("Usage: match <pattern> <string1> [string2] ...");
            return;
        }
        println!("Matching pattern '{}':", args[0]);
        let mut matches = 0;
        for s in &args[1..] {
            if *s == args[0] {
                println!("  '{}' - EXACT MATCH", s);
                matches += 1;
            } else if s.contains(args[0]) {
                println!("  '{}' - contains pattern", s);
                matches += 1;
            } else {
                println!("  '{}' - no match", s);
            }
        }
        println!("Total matches: {}", matches);
    }

    fn cmd_help(&self) {
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

    fn cmd_debug(&mut self, args: &[&str]) {
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

    fn cmd_verbose(&mut self, args: &[&str]) {
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
        println!("\n=== System Status ===");
        println!("Users: {}/{}", self.users.len(), MAX_USERS);
        println!("Files: {}/{}", self.files.len(), MAX_FILES);
        println!("Variables: {}/{}", self.variables.len(), MAX_VARIABLES);
        let current_user_name = match self.current_user_index {
            Some(idx) if self.users[idx].logged_in => self.users[idx].name.as_str(),
            _ => "none",
        };
        println!("Current user: {}", current_user_name);
        println!("Debug mode: {}", if self.debug_mode { "ON" } else { "OFF" });
        println!("Verbose mode: {}", if self.verbose_mode { "ON" } else { "OFF" });
    }

    fn cmd_time(&self) {
        let now = chrono::Local::now();
        println!("Current time: {}", now.format("%a %b %e %H:%M:%S %Y"));
    }

    fn process_command(&mut self, input: &str) {
        let mut tokens = input.split_whitespace();
        let command = match tokens.next() {
            Some(cmd) => cmd,
            None => return,
        };
        let args: Vec<&str> = tokens.take(10).collect();
        
        if self.debug_mode {
            println!("[DEBUG] Command: '{}', Args: {}", command, args.len());
        }
        
        match command {
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
            "compareN" | "cmpn" => self.cmd_compareN(&args),
            "startswith" => self.cmd_startswith(&args),
            "match" => self.cmd_match(&args),
            "debug" => self.cmd_debug(&args),
            "verbose" => self.cmd_verbose(&args),
            "status" => self.cmd_status(),
            "time" => self.cmd_time(),
            "help" | "?" => self.cmd_help(),
            "exit" | "quit" => {
                println!("Goodbye!");
                std::process::exit(0);
            }
            cmd if cmd.starts_with("add") => println!("Did you mean 'adduser'?"),
            cmd if cmd.starts_with("log") => println!("Did you mean 'login' or 'logout'?"),
            cmd if cmd.starts_with("list") => println!("Did you mean 'listusers', 'listfiles', or 'listvars'?"),
            cmd if cmd.starts_with("create") => println!("Did you mean 'createfile'?"),
            cmd if cmd.starts_with("read") => println!("Did you mean 'readfile'?"),
            cmd if cmd.starts_with("write") => println!("Did you mean 'writefile'?"),
            cmd if cmd.starts_with("delete") => println!("Did you mean 'deletefile'?"),
            _ => println!("Unknown command: '{}'. Type 'help' for available commands.", command),
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
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() || input.is_empty() {
            break;
        }
        
        let input = input.trim_end_matches(|c| c == '\r' || c == '\n');
        
        if state.verbose_mode {
            println!("[VERBOSE] Processing: '{}'", input);
        }
        
        state.process_command(input);
    }
}
