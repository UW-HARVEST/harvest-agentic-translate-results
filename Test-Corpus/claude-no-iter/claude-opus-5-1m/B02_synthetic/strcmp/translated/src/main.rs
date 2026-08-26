// Translation of c_src/src/main.c — preserves the original C behavior.
use std::io::{self, BufRead, BufWriter, Write};
use std::process;

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

#[derive(Clone)]
struct User {
    name: Vec<u8>,
    password: Vec<u8>,
    permission_level: i32,
    logged_in: bool,
}

#[derive(Clone)]
struct FileEntry {
    filename: Vec<u8>,
    content: Vec<u8>,
    owner: Vec<u8>,
    permissions: i32,
}

#[derive(Clone)]
struct Variable {
    name: Vec<u8>,
    value: Vec<u8>,
}

struct State {
    users: Vec<User>,
    current_user_idx: Option<usize>,
    files: Vec<FileEntry>,
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

    fn logged_in(&self) -> bool {
        match self.current_user_idx {
            Some(i) if i < self.users.len() => self.users[i].logged_in,
            _ => false,
        }
    }

    fn current_user_name(&self) -> Option<&[u8]> {
        match self.current_user_idx {
            Some(i) if i < self.users.len() && self.users[i].logged_in => {
                Some(&self.users[i].name)
            }
            _ => None,
        }
    }

    fn current_permission_level(&self) -> Option<i32> {
        match self.current_user_idx {
            Some(i) if i < self.users.len() && self.users[i].logged_in => {
                Some(self.users[i].permission_level)
            }
            _ => None,
        }
    }
}

// C-like atoi: skip whitespace, optional sign, parse digits, stop at first non-digit.
fn c_atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len()
        && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
    {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        if s[i] == b'-' {
            neg = true;
        }
        i += 1;
    }
    let mut result: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        result = result.wrapping_mul(10).wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        result = result.wrapping_neg();
    }
    result as i32
}

// C-style strcmp on byte slices (treating slices as full strings without null terminators).
fn strcmp(a: &[u8], b: &[u8]) -> i32 {
    let min_len = a.len().min(b.len());
    for i in 0..min_len {
        if a[i] != b[i] {
            return (a[i] as i32) - (b[i] as i32);
        }
    }
    match a.len().cmp(&b.len()) {
        std::cmp::Ordering::Less => -(b[a.len()] as i32),
        std::cmp::Ordering::Greater => a[b.len()] as i32,
        std::cmp::Ordering::Equal => 0,
    }
}

fn strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    let mut i = 0;
    while i < n && i < a.len() && i < b.len() {
        if a[i] != b[i] {
            return (a[i] as i32) - (b[i] as i32);
        }
        i += 1;
    }
    if i == n {
        return 0;
    }
    // One slice exhausted before n bytes were compared.
    if i >= a.len() && i >= b.len() {
        return 0;
    }
    if i >= a.len() {
        return -(b[i] as i32);
    }
    a[i] as i32
}

fn strstr_pos(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn truncate_bytes(s: &[u8], max_len: usize) -> Vec<u8> {
    if s.len() <= max_len {
        s.to_vec()
    } else {
        s[..max_len].to_vec()
    }
}

// Mirrors parse_command(): truncates input to MAX_INPUT-1 bytes,
// tokenizes on ' ' and '\t', then truncates each token to MAX_COMMAND-1 bytes.
// First token becomes cmd; subsequent tokens (up to MAX_ARGS) are args.
fn parse_command(input: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    let work: &[u8] = if input.len() > MAX_INPUT - 1 {
        &input[..MAX_INPUT - 1]
    } else {
        input
    };

    let mut tokens: Vec<&[u8]> = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &c) in work.iter().enumerate() {
        if c == b' ' || c == b'\t' {
            if let Some(s) = start.take() {
                tokens.push(&work[s..i]);
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        tokens.push(&work[s..]);
    }

    let mut cmd: Vec<u8> = Vec::new();
    let mut args: Vec<Vec<u8>> = Vec::new();
    if let Some(first) = tokens.first() {
        cmd = truncate_bytes(first, MAX_COMMAND - 1);
        for t in tokens.iter().skip(1) {
            if args.len() >= MAX_ARGS {
                break;
            }
            args.push(truncate_bytes(t, MAX_COMMAND - 1));
        }
    }
    (cmd, args)
}

// Emulates fgets: reads up to max-1 bytes, stopping at and including '\n', or at EOF.
// Returns None only at EOF with no bytes read (matches fgets returning NULL).
fn read_line_fgets<R: BufRead>(reader: &mut R, max: usize) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let limit = max - 1;
    while buf.len() < limit {
        let consumed_to_end;
        let pos_opt;
        let take;
        {
            let available = match reader.fill_buf() {
                Ok(b) => b,
                Err(_) => return None,
            };
            if available.is_empty() {
                if buf.is_empty() {
                    return None;
                }
                return Some(buf);
            }
            let remaining = limit - buf.len();
            take = available.len().min(remaining);
            pos_opt = available[..take].iter().position(|&b| b == b'\n');
            match pos_opt {
                Some(pos) => {
                    buf.extend_from_slice(&available[..=pos]);
                    consumed_to_end = pos + 1;
                }
                None => {
                    buf.extend_from_slice(&available[..take]);
                    consumed_to_end = take;
                }
            }
        }
        reader.consume(consumed_to_end);
        if pos_opt.is_some() {
            return Some(buf);
        }
    }
    Some(buf)
}

#[inline]
fn w_bytes<W: Write>(w: &mut W, b: &[u8]) {
    let _ = w.write_all(b);
}

#[inline]
fn w_str<W: Write>(w: &mut W, s: &str) {
    let _ = w.write_all(s.as_bytes());
}

// ----- User commands -----

fn cmd_adduser<W: Write>(w: &mut W, state: &mut State, args: &[Vec<u8>]) {
    if args.len() < 2 {
        w_str(w, "Usage: adduser <username> <password> [permission_level]\n");
        return;
    }
    if state.users.len() >= MAX_USERS {
        w_str(w, "Error: Maximum users reached\n");
        return;
    }
    for u in &state.users {
        if u.name == args[0] {
            w_str(w, "Error: User '");
            w_bytes(w, &args[0]);
            w_str(w, "' already exists\n");
            return;
        }
    }
    let permission_level = if args.len() >= 3 {
        c_atoi(&args[2])
    } else {
        1
    };
    state.users.push(User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level,
        logged_in: false,
    });
    w_str(w, "User '");
    w_bytes(w, &args[0]);
    let _ = write!(w, "' added with permission level {}\n", permission_level);
}

fn cmd_login<W: Write>(w: &mut W, state: &mut State, args: &[Vec<u8>]) {
    if args.len() < 2 {
        w_str(w, "Usage: login <username> <password>\n");
        return;
    }
    if state.logged_in() {
        let name = state.users[state.current_user_idx.unwrap()].name.clone();
        w_str(w, "Error: User '");
        w_bytes(w, &name);
        w_str(w, "' already logged in. Use 'logout' first.\n");
        return;
    }
    for i in 0..state.users.len() {
        if state.users[i].name == args[0] {
            if state.users[i].password == args[1] {
                state.users[i].logged_in = true;
                state.current_user_idx = Some(i);
                w_str(w, "Login successful. Welcome, ");
                w_bytes(w, &state.users[i].name);
                w_str(w, "!\n");
                return;
            } else {
                w_str(w, "Error: Incorrect password\n");
                return;
            }
        }
    }
    w_str(w, "Error: User not found\n");
}

fn cmd_logout<W: Write>(w: &mut W, state: &mut State) {
    if !state.logged_in() {
        w_str(w, "Error: No user logged in\n");
        return;
    }
    let idx = state.current_user_idx.unwrap();
    w_str(w, "Goodbye, ");
    w_bytes(w, &state.users[idx].name);
    w_str(w, "!\n");
    state.users[idx].logged_in = false;
    state.current_user_idx = None;
}

fn cmd_whoami<W: Write>(w: &mut W, state: &State) {
    if !state.logged_in() {
        w_str(w, "Not logged in\n");
        return;
    }
    let idx = state.current_user_idx.unwrap();
    w_str(w, "Current user: ");
    w_bytes(w, &state.users[idx].name);
    w_str(w, "\n");
    let _ = write!(
        w,
        "Permission level: {}\n",
        state.users[idx].permission_level
    );
}

fn cmd_listusers<W: Write>(w: &mut W, state: &State) {
    if state.users.is_empty() {
        w_str(w, "No users registered\n");
        return;
    }
    w_str(w, "Registered users:\n");
    for u in &state.users {
        w_str(w, "  ");
        w_bytes(w, &u.name);
        let _ = write!(w, " (level {}) ", u.permission_level);
        if u.logged_in {
            w_str(w, "[logged in]");
        }
        w_str(w, "\n");
    }
}

// ----- File commands -----

fn cmd_createfile<W: Write>(w: &mut W, state: &mut State, args: &[Vec<u8>]) {
    if !state.logged_in() {
        w_str(w, "Error: Must be logged in\n");
        return;
    }
    if args.is_empty() {
        w_str(w, "Usage: createfile <filename> [content]\n");
        return;
    }
    if state.files.len() >= MAX_FILES {
        w_str(w, "Error: Maximum files reached\n");
        return;
    }
    for f in &state.files {
        if f.filename == args[0] {
            w_str(w, "Error: File '");
            w_bytes(w, &args[0]);
            w_str(w, "' already exists\n");
            return;
        }
    }
    let owner = state.current_user_name().unwrap().to_vec();
    let content = if args.len() >= 2 {
        args[1].clone()
    } else {
        Vec::new()
    };
    state.files.push(FileEntry {
        filename: args[0].clone(),
        content,
        owner,
        permissions: 755,
    });
    w_str(w, "File '");
    w_bytes(w, &args[0]);
    w_str(w, "' created\n");
}

fn cmd_readfile<W: Write>(w: &mut W, state: &State, args: &[Vec<u8>]) {
    if args.is_empty() {
        w_str(w, "Usage: readfile <filename>\n");
        return;
    }
    for f in &state.files {
        if f.filename == args[0] {
            w_str(w, "=== ");
            w_bytes(w, &f.filename);
            w_str(w, " ===\n");
            w_str(w, "Owner: ");
            w_bytes(w, &f.owner);
            w_str(w, "\n");
            let _ = write!(w, "Permissions: {}\n", f.permissions);
            w_str(w, "Content: ");
            w_bytes(w, &f.content);
            w_str(w, "\n");
            return;
        }
    }
    w_str(w, "Error: File '");
    w_bytes(w, &args[0]);
    w_str(w, "' not found\n");
}

fn cmd_writefile<W: Write>(w: &mut W, state: &mut State, args: &[Vec<u8>]) {
    if !state.logged_in() {
        w_str(w, "Error: Must be logged in\n");
        return;
    }
    if args.len() < 2 {
        w_str(w, "Usage: writefile <filename> <content>\n");
        return;
    }
    let cur_name = state.current_user_name().unwrap().to_vec();
    let cur_perm = state.current_permission_level().unwrap();
    for i in 0..state.files.len() {
        if state.files[i].filename == args[0] {
            if state.files[i].owner == cur_name || cur_perm >= 5 {
                state.files[i].content = args[1].clone();
                w_str(w, "File '");
                w_bytes(w, &args[0]);
                w_str(w, "' updated\n");
                return;
            } else {
                w_str(w, "Error: Permission denied\n");
                return;
            }
        }
    }
    w_str(w, "Error: File '");
    w_bytes(w, &args[0]);
    w_str(w, "' not found\n");
}

fn cmd_deletefile<W: Write>(w: &mut W, state: &mut State, args: &[Vec<u8>]) {
    if !state.logged_in() {
        w_str(w, "Error: Must be logged in\n");
        return;
    }
    if args.is_empty() {
        w_str(w, "Usage: deletefile <filename>\n");
        return;
    }
    let cur_name = state.current_user_name().unwrap().to_vec();
    let cur_perm = state.current_permission_level().unwrap();
    for i in 0..state.files.len() {
        if state.files[i].filename == args[0] {
            if state.files[i].owner == cur_name || cur_perm >= 9 {
                state.files.remove(i);
                w_str(w, "File '");
                w_bytes(w, &args[0]);
                w_str(w, "' deleted\n");
                return;
            } else {
                w_str(w, "Error: Permission denied\n");
                return;
            }
        }
    }
    w_str(w, "Error: File '");
    w_bytes(w, &args[0]);
    w_str(w, "' not found\n");
}

fn cmd_listfiles<W: Write>(w: &mut W, state: &State) {
    if state.files.is_empty() {
        w_str(w, "No files\n");
        return;
    }
    w_str(w, "Files:\n");
    for f in &state.files {
        w_str(w, "  ");
        w_bytes(w, &f.filename);
        w_str(w, " (owner: ");
        w_bytes(w, &f.owner);
        let _ = write!(w, ", perm: {})\n", f.permissions);
    }
}

// ----- Variable commands -----

fn cmd_set<W: Write>(w: &mut W, state: &mut State, args: &[Vec<u8>]) {
    if args.len() < 2 {
        w_str(w, "Usage: set <name> <value>\n");
        return;
    }
    for i in 0..state.variables.len() {
        if state.variables[i].name == args[0] {
            state.variables[i].value = args[1].clone();
            w_str(w, "Variable '");
            w_bytes(w, &args[0]);
            w_str(w, "' updated\n");
            return;
        }
    }
    if state.variables.len() >= MAX_VARIABLES {
        w_str(w, "Error: Maximum variables reached\n");
        return;
    }
    state.variables.push(Variable {
        name: args[0].clone(),
        value: args[1].clone(),
    });
    w_str(w, "Variable '");
    w_bytes(w, &args[0]);
    w_str(w, "' set\n");
}

fn cmd_get<W: Write>(w: &mut W, state: &State, args: &[Vec<u8>]) {
    if args.is_empty() {
        w_str(w, "Usage: get <name>\n");
        return;
    }
    for v in &state.variables {
        if v.name == args[0] {
            w_bytes(w, &v.name);
            w_str(w, " = ");
            w_bytes(w, &v.value);
            w_str(w, "\n");
            return;
        }
    }
    w_str(w, "Error: Variable '");
    w_bytes(w, &args[0]);
    w_str(w, "' not found\n");
}

fn cmd_unset<W: Write>(w: &mut W, state: &mut State, args: &[Vec<u8>]) {
    if args.is_empty() {
        w_str(w, "Usage: unset <name>\n");
        return;
    }
    for i in 0..state.variables.len() {
        if state.variables[i].name == args[0] {
            state.variables.remove(i);
            w_str(w, "Variable '");
            w_bytes(w, &args[0]);
            w_str(w, "' unset\n");
            return;
        }
    }
    w_str(w, "Error: Variable '");
    w_bytes(w, &args[0]);
    w_str(w, "' not found\n");
}

fn cmd_listvars<W: Write>(w: &mut W, state: &State) {
    if state.variables.is_empty() {
        w_str(w, "No variables set\n");
        return;
    }
    w_str(w, "Variables:\n");
    for v in &state.variables {
        w_str(w, "  ");
        w_bytes(w, &v.name);
        w_str(w, " = ");
        w_bytes(w, &v.value);
        w_str(w, "\n");
    }
}

// ----- String comparison commands -----

fn cmd_compare<W: Write>(w: &mut W, args: &[Vec<u8>]) {
    if args.len() < 2 {
        w_str(w, "Usage: compare <string1> <string2>\n");
        return;
    }
    let result = strcmp(&args[0], &args[1]);
    w_str(w, "strcmp('");
    w_bytes(w, &args[0]);
    w_str(w, "', '");
    w_bytes(w, &args[1]);
    let _ = write!(w, "') = {}\n", result);
    if result == 0 {
        w_str(w, "Strings are equal\n");
    } else if result < 0 {
        w_str(w, "'");
        w_bytes(w, &args[0]);
        w_str(w, "' < '");
        w_bytes(w, &args[1]);
        w_str(w, "'\n");
    } else {
        w_str(w, "'");
        w_bytes(w, &args[0]);
        w_str(w, "' > '");
        w_bytes(w, &args[1]);
        w_str(w, "'\n");
    }
}

fn cmd_compare_n<W: Write>(w: &mut W, args: &[Vec<u8>]) {
    if args.len() < 3 {
        w_str(w, "Usage: compareN <string1> <string2> <n>\n");
        return;
    }
    let n = c_atoi(&args[2]);
    let n_size: usize = if n < 0 { usize::MAX } else { n as usize };
    let result = strncmp(&args[0], &args[1], n_size);
    w_str(w, "strncmp('");
    w_bytes(w, &args[0]);
    w_str(w, "', '");
    w_bytes(w, &args[1]);
    let _ = write!(w, "', {}) = {}\n", n, result);
    if result == 0 {
        let _ = write!(w, "First {} characters are equal\n", n);
    } else if result < 0 {
        w_str(w, "'");
        w_bytes(w, &args[0]);
        w_str(w, "' < '");
        w_bytes(w, &args[1]);
        let _ = write!(w, "' (first {} chars)\n", n);
    } else {
        w_str(w, "'");
        w_bytes(w, &args[0]);
        w_str(w, "' > '");
        w_bytes(w, &args[1]);
        let _ = write!(w, "' (first {} chars)\n", n);
    }
}

fn cmd_startswith<W: Write>(w: &mut W, args: &[Vec<u8>]) {
    if args.len() < 2 {
        w_str(w, "Usage: startswith <string> <prefix>\n");
        return;
    }
    let prefix_len = args[1].len();
    if strncmp(&args[0], &args[1], prefix_len) == 0 {
        w_str(w, "'");
        w_bytes(w, &args[0]);
        w_str(w, "' starts with '");
        w_bytes(w, &args[1]);
        w_str(w, "'\n");
    } else {
        w_str(w, "'");
        w_bytes(w, &args[0]);
        w_str(w, "' does not start with '");
        w_bytes(w, &args[1]);
        w_str(w, "'\n");
    }
}

fn cmd_match<W: Write>(w: &mut W, args: &[Vec<u8>]) {
    if args.len() < 2 {
        w_str(w, "Usage: match <pattern> <string1> [string2] ...\n");
        return;
    }
    w_str(w, "Matching pattern '");
    w_bytes(w, &args[0]);
    w_str(w, "':\n");
    let mut matches: i32 = 0;
    for i in 1..args.len() {
        if strcmp(&args[0], &args[i]) == 0 {
            w_str(w, "  '");
            w_bytes(w, &args[i]);
            w_str(w, "' - EXACT MATCH\n");
            matches += 1;
        } else if strstr_pos(&args[i], &args[0]).is_some() {
            w_str(w, "  '");
            w_bytes(w, &args[i]);
            w_str(w, "' - contains pattern\n");
            matches += 1;
        } else {
            w_str(w, "  '");
            w_bytes(w, &args[i]);
            w_str(w, "' - no match\n");
        }
    }
    let _ = write!(w, "Total matches: {}\n", matches);
}

// ----- System commands -----

fn cmd_help<W: Write>(w: &mut W) {
    w_str(w, "\n=== Command Interpreter Help ===\n");
    w_str(w, "User Management:\n");
    w_str(w, "  adduser <user> <pass> [level] - Add new user\n");
    w_str(w, "  login <user> <pass>            - Login as user\n");
    w_str(w, "  logout                         - Logout current user\n");
    w_str(w, "  whoami                         - Show current user\n");
    w_str(w, "  listusers                      - List all users\n");
    w_str(w, "\nFile Management:\n");
    w_str(w, "  createfile <name> [content]    - Create file\n");
    w_str(w, "  readfile <name>                - Read file\n");
    w_str(w, "  writefile <name> <content>     - Write to file\n");
    w_str(w, "  deletefile <name>              - Delete file\n");
    w_str(w, "  listfiles                      - List all files\n");
    w_str(w, "\nVariable Management:\n");
    w_str(w, "  set <name> <value>             - Set variable\n");
    w_str(w, "  get <name>                     - Get variable\n");
    w_str(w, "  unset <name>                   - Unset variable\n");
    w_str(w, "  listvars                       - List all variables\n");
    w_str(w, "\nString Operations:\n");
    w_str(w, "  compare <str1> <str2>          - Compare strings\n");
    w_str(w, "  compareN <str1> <str2> <n>     - Compare first N chars\n");
    w_str(w, "  startswith <str> <prefix>      - Check if starts with\n");
    w_str(w, "  match <pattern> <str> ...      - Match pattern\n");
    w_str(w, "\nSystem:\n");
    w_str(w, "  debug [on|off]                 - Toggle debug mode\n");
    w_str(w, "  verbose [on|off]               - Toggle verbose mode\n");
    w_str(w, "  status                         - Show system status\n");
    w_str(w, "  time                           - Show current time\n");
    w_str(w, "  help                           - Show this help\n");
    w_str(w, "  exit                           - Exit program\n");
}

fn cmd_debug<W: Write>(w: &mut W, state: &mut State, args: &[Vec<u8>]) {
    if args.is_empty() {
        let _ = write!(
            w,
            "Debug mode: {}\n",
            if state.debug_mode { "ON" } else { "OFF" }
        );
        return;
    }
    if args[0] == b"on" {
        state.debug_mode = true;
        w_str(w, "Debug mode enabled\n");
    } else if args[0] == b"off" {
        state.debug_mode = false;
        w_str(w, "Debug mode disabled\n");
    } else {
        w_str(w, "Usage: debug [on|off]\n");
    }
}

fn cmd_verbose<W: Write>(w: &mut W, state: &mut State, args: &[Vec<u8>]) {
    if args.is_empty() {
        let _ = write!(
            w,
            "Verbose mode: {}\n",
            if state.verbose_mode { "ON" } else { "OFF" }
        );
        return;
    }
    if args[0] == b"on" {
        state.verbose_mode = true;
        w_str(w, "Verbose mode enabled\n");
    } else if args[0] == b"off" {
        state.verbose_mode = false;
        w_str(w, "Verbose mode disabled\n");
    } else {
        w_str(w, "Usage: verbose [on|off]\n");
    }
}

fn cmd_status<W: Write>(w: &mut W, state: &State) {
    w_str(w, "\n=== System Status ===\n");
    let _ = write!(w, "Users: {}/{}\n", state.users.len(), MAX_USERS);
    let _ = write!(w, "Files: {}/{}\n", state.files.len(), MAX_FILES);
    let _ = write!(
        w,
        "Variables: {}/{}\n",
        state.variables.len(),
        MAX_VARIABLES
    );
    w_str(w, "Current user: ");
    if let Some(name) = state.current_user_name() {
        w_bytes(w, name);
    } else {
        w_str(w, "none");
    }
    w_str(w, "\n");
    let _ = write!(
        w,
        "Debug mode: {}\n",
        if state.debug_mode { "ON" } else { "OFF" }
    );
    let _ = write!(
        w,
        "Verbose mode: {}\n",
        if state.verbose_mode { "ON" } else { "OFF" }
    );
}

extern "C" {
    fn ctime(time: *const libc::time_t) -> *const libc::c_char;
}

fn cmd_time<W: Write>(w: &mut W) {
    // Match libc time(NULL) + ctime(&now) byte-for-byte by using libc directly.
    unsafe {
        let now: libc::time_t = libc::time(std::ptr::null_mut());
        let s_ptr = ctime(&now);
        if !s_ptr.is_null() {
            let cstr = std::ffi::CStr::from_ptr(s_ptr);
            let bytes = cstr.to_bytes();
            w_str(w, "Current time: ");
            w_bytes(w, bytes);
        }
    }
}

// Returns true if the program should exit.
fn process_command<W: Write>(w: &mut W, state: &mut State, input: &[u8]) -> bool {
    let (command, args) = parse_command(input);

    if command.is_empty() {
        return false;
    }

    if state.debug_mode {
        w_str(w, "[DEBUG] Command: '");
        w_bytes(w, &command);
        let _ = write!(w, "', Args: {}\n", args.len());
    }

    // Command routing — order matches the original C implementation.
    if command == b"adduser" {
        cmd_adduser(w, state, &args);
    } else if command == b"login" {
        cmd_login(w, state, &args);
    } else if command == b"logout" {
        cmd_logout(w, state);
    } else if command == b"whoami" {
        cmd_whoami(w, state);
    } else if command == b"listusers" || command == b"users" {
        cmd_listusers(w, state);
    } else if command == b"createfile" || command == b"touch" {
        cmd_createfile(w, state, &args);
    } else if command == b"readfile" || command == b"cat" {
        cmd_readfile(w, state, &args);
    } else if command == b"writefile" || command == b"write" {
        cmd_writefile(w, state, &args);
    } else if command == b"deletefile" || command == b"rm" {
        cmd_deletefile(w, state, &args);
    } else if command == b"listfiles" || command == b"ls" {
        cmd_listfiles(w, state);
    } else if command == b"set" {
        cmd_set(w, state, &args);
    } else if command == b"get" {
        cmd_get(w, state, &args);
    } else if command == b"unset" {
        cmd_unset(w, state, &args);
    } else if command == b"listvars" || command == b"vars" {
        cmd_listvars(w, state);
    } else if command == b"compare" || command == b"cmp" {
        cmd_compare(w, &args);
    } else if command == b"compareN" || command == b"cmpn" {
        cmd_compare_n(w, &args);
    } else if command == b"startswith" {
        cmd_startswith(w, &args);
    } else if command == b"match" {
        cmd_match(w, &args);
    } else if command == b"debug" {
        cmd_debug(w, state, &args);
    } else if command == b"verbose" {
        cmd_verbose(w, state, &args);
    } else if command == b"status" {
        cmd_status(w, state);
    } else if command == b"time" {
        cmd_time(w);
    } else if command == b"help" || command == b"?" {
        cmd_help(w);
    } else if command == b"exit" || command == b"quit" {
        w_str(w, "Goodbye!\n");
        return true;
    } else if strncmp(&command, b"add", 3) == 0 {
        w_str(w, "Did you mean 'adduser'?\n");
    } else if strncmp(&command, b"log", 3) == 0 {
        w_str(w, "Did you mean 'login' or 'logout'?\n");
    } else if strncmp(&command, b"list", 4) == 0 {
        w_str(w, "Did you mean 'listusers', 'listfiles', or 'listvars'?\n");
    } else if strncmp(&command, b"create", 6) == 0 {
        w_str(w, "Did you mean 'createfile'?\n");
    } else if strncmp(&command, b"read", 4) == 0 {
        w_str(w, "Did you mean 'readfile'?\n");
    } else if strncmp(&command, b"write", 5) == 0 {
        w_str(w, "Did you mean 'writefile'?\n");
    } else if strncmp(&command, b"delete", 6) == 0 {
        w_str(w, "Did you mean 'deletefile'?\n");
    } else {
        w_str(w, "Unknown command: '");
        w_bytes(w, &command);
        w_str(w, "'. Type 'help' for available commands.\n");
    }

    false
}

fn main() {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();

    let stdout = io::stdout();
    let stdout_lock = stdout.lock();
    let mut out = BufWriter::new(stdout_lock);

    let mut state = State::new();

    w_str(&mut out, "|----------------------------------------|\n");
    w_str(&mut out, "|   COMMAND INTERPRETER                  |\n");
    w_str(&mut out, "|   strcmp/strncmp demonstration         |\n");
    w_str(&mut out, "|----------------------------------------|\n");
    w_str(&mut out, "Type 'help' for available commands\n\n");

    loop {
        w_str(&mut out, "> ");

        let raw = match read_line_fgets(&mut stdin_lock, MAX_INPUT) {
            Some(l) => l,
            None => break,
        };

        // Strip the first '\n' if present (mirrors strcspn(input, "\n") = 0).
        let input: Vec<u8> = if let Some(pos) = raw.iter().position(|&b| b == b'\n') {
            raw[..pos].to_vec()
        } else {
            raw
        };

        if state.verbose_mode {
            w_str(&mut out, "[VERBOSE] Processing: '");
            w_bytes(&mut out, &input);
            w_str(&mut out, "'\n");
        }

        if process_command(&mut out, &mut state, &input) {
            let _ = out.flush();
            process::exit(0);
        }
    }

    let _ = out.flush();
}
