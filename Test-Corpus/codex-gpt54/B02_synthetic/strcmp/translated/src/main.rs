use std::io::{self, Read, Write};

unsafe extern "C" {
    fn ctime(timer: *const libc::time_t) -> *mut libc::c_char;
}

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
    current_user: Option<usize>,
    files: Vec<FileEntry>,
    variables: Vec<Variable>,
    debug_mode: bool,
    verbose_mode: bool,
}

impl State {
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

    fn current_user(&self) -> Option<&User> {
        self.current_user
            .and_then(|idx| self.users.get(idx))
            .filter(|user| user.logged_in)
    }

    fn current_user_index(&self) -> Option<usize> {
        self.current_user
            .filter(|&idx| self.users.get(idx).map(|user| user.logged_in).unwrap_or(false))
    }
}

fn truncated_token(bytes: &[u8]) -> Vec<u8> {
    bytes[..bytes.len().min(MAX_COMMAND - 1)].to_vec()
}

fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let ac = a.get(i).copied().unwrap_or(0);
        let bc = b.get(i).copied().unwrap_or(0);
        if ac != bc {
            return ac as i32 - bc as i32;
        }
        if ac == 0 {
            return 0;
        }
        i += 1;
    }
}

fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    if n == 0 {
        return 0;
    }

    let mut i = 0usize;
    while i < n {
        let ac = a.get(i).copied().unwrap_or(0);
        let bc = b.get(i).copied().unwrap_or(0);
        if ac != bc {
            return ac as i32 - bc as i32;
        }
        if ac == 0 {
            return 0;
        }
        i += 1;
    }

    0
}

fn c_strstr(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }

    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn c_atoi(bytes: &[u8]) -> i32 {
    let mut idx = 0usize;
    while idx < bytes.len() && matches!(bytes[idx], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        idx += 1;
    }

    let mut sign = 1i32;
    if idx < bytes.len() {
        if bytes[idx] == b'-' {
            sign = -1;
            idx += 1;
        } else if bytes[idx] == b'+' {
            idx += 1;
        }
    }

    let mut value = 0i32;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        value = value.saturating_mul(10).saturating_add((bytes[idx] - b'0') as i32);
        idx += 1;
    }

    value.saturating_mul(sign)
}

fn write_bytes(out: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    out.write_all(bytes)
}

fn write_line(out: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    out.write_all(bytes)?;
    out.write_all(b"\n")
}

fn write_c_quoted_line(
    out: &mut impl Write,
    prefix: &[u8],
    a: &[u8],
    middle: &[u8],
    b: &[u8],
    suffix: &[u8],
) -> io::Result<()> {
    out.write_all(prefix)?;
    out.write_all(a)?;
    out.write_all(middle)?;
    out.write_all(b)?;
    out.write_all(suffix)
}

fn parse_command(input: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    let temp = input[..input.len().min(MAX_INPUT - 1)].to_vec();
    let mut tokens = Vec::new();
    let mut start = 0usize;

    while start < temp.len() {
        while start < temp.len() && (temp[start] == b' ' || temp[start] == b'\t') {
            start += 1;
        }
        if start >= temp.len() {
            break;
        }

        let mut end = start;
        while end < temp.len() && temp[end] != b' ' && temp[end] != b'\t' {
            end += 1;
        }

        tokens.push(truncated_token(&temp[start..end]));
        start = end;
    }

    if tokens.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let command = tokens.remove(0);
    let args = tokens.into_iter().take(MAX_ARGS).collect();
    (command, args)
}

fn cmd_adduser(state: &mut State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.len() < 2 {
        return write_line(out, b"Usage: adduser <username> <password> [permission_level]");
    }

    if state.users.len() >= MAX_USERS {
        return write_line(out, b"Error: Maximum users reached");
    }

    for user in &state.users {
        if c_strcmp(&user.name, &args[0]) == 0 {
            write_bytes(out, b"Error: User '")?;
            write_bytes(out, &args[0])?;
            return write_line(out, b"' already exists");
        }
    }

    let permission_level = if args.len() >= 3 { c_atoi(&args[2]) } else { 1 };
    state.users.push(User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level,
        logged_in: false,
    });

    write_bytes(out, b"User '")?;
    write_bytes(out, &args[0])?;
    write_bytes(out, b"' added with permission level ")?;
    write_line(out, permission_level.to_string().as_bytes())
}

fn cmd_login(state: &mut State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.len() < 2 {
        return write_line(out, b"Usage: login <username> <password>");
    }

    if let Some(current_user) = state.current_user() {
        write_bytes(out, b"Error: User '")?;
        write_bytes(out, &current_user.name)?;
        return write_line(out, b"' already logged in. Use 'logout' first.");
    }

    for idx in 0..state.users.len() {
        if c_strcmp(&state.users[idx].name, &args[0]) == 0 {
            if c_strcmp(&state.users[idx].password, &args[1]) == 0 {
                state.users[idx].logged_in = true;
                state.current_user = Some(idx);
                write_bytes(out, b"Login successful. Welcome, ")?;
                write_bytes(out, &state.users[idx].name)?;
                return write_line(out, b"!");
            } else {
                return write_line(out, b"Error: Incorrect password");
            }
        }
    }

    write_line(out, b"Error: User not found")
}

fn cmd_logout(state: &mut State, out: &mut impl Write) -> io::Result<()> {
    let Some(idx) = state.current_user_index() else {
        return write_line(out, b"Error: No user logged in");
    };

    write_bytes(out, b"Goodbye, ")?;
    write_bytes(out, &state.users[idx].name)?;
    write_line(out, b"!")?;
    state.users[idx].logged_in = false;
    state.current_user = None;
    Ok(())
}

fn cmd_whoami(state: &State, out: &mut impl Write) -> io::Result<()> {
    let Some(user) = state.current_user() else {
        return write_line(out, b"Not logged in");
    };

    write_bytes(out, b"Current user: ")?;
    write_bytes(out, &user.name)?;
    write_line(out, b"")?;
    write_bytes(out, b"Permission level: ")?;
    write_line(out, user.permission_level.to_string().as_bytes())
}

fn cmd_listusers(state: &State, out: &mut impl Write) -> io::Result<()> {
    if state.users.is_empty() {
        return write_line(out, b"No users registered");
    }

    write_line(out, b"Registered users:")?;
    for user in &state.users {
        write_bytes(out, b"  ")?;
        write_bytes(out, &user.name)?;
        write_bytes(out, b" (level ")?;
        write_bytes(out, user.permission_level.to_string().as_bytes())?;
        write_bytes(out, b") ")?;
        if user.logged_in {
            write_bytes(out, b"[logged in]")?;
        }
        write_line(out, b"")?;
    }
    Ok(())
}

fn cmd_createfile(state: &mut State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    let Some(current_idx) = state.current_user_index() else {
        return write_line(out, b"Error: Must be logged in");
    };

    if args.is_empty() {
        return write_line(out, b"Usage: createfile <filename> [content]");
    }

    if state.files.len() >= MAX_FILES {
        return write_line(out, b"Error: Maximum files reached");
    }

    for file in &state.files {
        if c_strcmp(&file.filename, &args[0]) == 0 {
            write_bytes(out, b"Error: File '")?;
            write_bytes(out, &args[0])?;
            return write_line(out, b"' already exists");
        }
    }

    state.files.push(FileEntry {
        filename: args[0].clone(),
        content: args.get(1).cloned().unwrap_or_default(),
        owner: state.users[current_idx].name.clone(),
        permissions: 755,
    });

    write_bytes(out, b"File '")?;
    write_bytes(out, &args[0])?;
    write_line(out, b"' created")
}

fn cmd_readfile(state: &State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.is_empty() {
        return write_line(out, b"Usage: readfile <filename>");
    }

    for file in &state.files {
        if c_strcmp(&file.filename, &args[0]) == 0 {
            write_bytes(out, b"=== ")?;
            write_bytes(out, &file.filename)?;
            write_line(out, b" ===")?;
            write_bytes(out, b"Owner: ")?;
            write_bytes(out, &file.owner)?;
            write_line(out, b"")?;
            write_bytes(out, b"Permissions: ")?;
            write_line(out, file.permissions.to_string().as_bytes())?;
            write_bytes(out, b"Content: ")?;
            write_bytes(out, &file.content)?;
            return write_line(out, b"");
        }
    }

    write_bytes(out, b"Error: File '")?;
    write_bytes(out, &args[0])?;
    write_line(out, b"' not found")
}

fn cmd_writefile(state: &mut State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    let Some(current_idx) = state.current_user_index() else {
        return write_line(out, b"Error: Must be logged in");
    };

    if args.len() < 2 {
        return write_line(out, b"Usage: writefile <filename> <content>");
    }

    let current_name = state.users[current_idx].name.clone();
    let current_permission = state.users[current_idx].permission_level;

    for file in &mut state.files {
        if c_strcmp(&file.filename, &args[0]) == 0 {
            if c_strcmp(&file.owner, &current_name) == 0 || current_permission >= 5 {
                file.content = args[1].clone();
                write_bytes(out, b"File '")?;
                write_bytes(out, &args[0])?;
                return write_line(out, b"' updated");
            } else {
                return write_line(out, b"Error: Permission denied");
            }
        }
    }

    write_bytes(out, b"Error: File '")?;
    write_bytes(out, &args[0])?;
    write_line(out, b"' not found")
}

fn cmd_deletefile(state: &mut State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    let Some(current_idx) = state.current_user_index() else {
        return write_line(out, b"Error: Must be logged in");
    };

    if args.is_empty() {
        return write_line(out, b"Usage: deletefile <filename>");
    }

    let current_name = state.users[current_idx].name.clone();
    let current_permission = state.users[current_idx].permission_level;

    for idx in 0..state.files.len() {
        if c_strcmp(&state.files[idx].filename, &args[0]) == 0 {
            if c_strcmp(&state.files[idx].owner, &current_name) == 0 || current_permission >= 9 {
                state.files.remove(idx);
                write_bytes(out, b"File '")?;
                write_bytes(out, &args[0])?;
                return write_line(out, b"' deleted");
            } else {
                return write_line(out, b"Error: Permission denied");
            }
        }
    }

    write_bytes(out, b"Error: File '")?;
    write_bytes(out, &args[0])?;
    write_line(out, b"' not found")
}

fn cmd_listfiles(state: &State, out: &mut impl Write) -> io::Result<()> {
    if state.files.is_empty() {
        return write_line(out, b"No files");
    }

    write_line(out, b"Files:")?;
    for file in &state.files {
        write_bytes(out, b"  ")?;
        write_bytes(out, &file.filename)?;
        write_bytes(out, b" (owner: ")?;
        write_bytes(out, &file.owner)?;
        write_bytes(out, b", perm: ")?;
        write_bytes(out, file.permissions.to_string().as_bytes())?;
        write_line(out, b")")?;
    }
    Ok(())
}

fn cmd_set(state: &mut State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.len() < 2 {
        return write_line(out, b"Usage: set <name> <value>");
    }

    for variable in &mut state.variables {
        if c_strcmp(&variable.name, &args[0]) == 0 {
            variable.value = args[1].clone();
            write_bytes(out, b"Variable '")?;
            write_bytes(out, &args[0])?;
            return write_line(out, b"' updated");
        }
    }

    if state.variables.len() >= MAX_VARIABLES {
        return write_line(out, b"Error: Maximum variables reached");
    }

    state.variables.push(Variable {
        name: args[0].clone(),
        value: args[1].clone(),
    });
    write_bytes(out, b"Variable '")?;
    write_bytes(out, &args[0])?;
    write_line(out, b"' set")
}

fn cmd_get(state: &State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.is_empty() {
        return write_line(out, b"Usage: get <name>");
    }

    for variable in &state.variables {
        if c_strcmp(&variable.name, &args[0]) == 0 {
            write_bytes(out, &variable.name)?;
            write_bytes(out, b" = ")?;
            write_bytes(out, &variable.value)?;
            return write_line(out, b"");
        }
    }

    write_bytes(out, b"Error: Variable '")?;
    write_bytes(out, &args[0])?;
    write_line(out, b"' not found")
}

fn cmd_unset(state: &mut State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.is_empty() {
        return write_line(out, b"Usage: unset <name>");
    }

    for idx in 0..state.variables.len() {
        if c_strcmp(&state.variables[idx].name, &args[0]) == 0 {
            state.variables.remove(idx);
            write_bytes(out, b"Variable '")?;
            write_bytes(out, &args[0])?;
            return write_line(out, b"' unset");
        }
    }

    write_bytes(out, b"Error: Variable '")?;
    write_bytes(out, &args[0])?;
    write_line(out, b"' not found")
}

fn cmd_listvars(state: &State, out: &mut impl Write) -> io::Result<()> {
    if state.variables.is_empty() {
        return write_line(out, b"No variables set");
    }

    write_line(out, b"Variables:")?;
    for variable in &state.variables {
        write_bytes(out, b"  ")?;
        write_bytes(out, &variable.name)?;
        write_bytes(out, b" = ")?;
        write_bytes(out, &variable.value)?;
        write_line(out, b"")?;
    }
    Ok(())
}

fn cmd_compare(args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.len() < 2 {
        return write_line(out, b"Usage: compare <string1> <string2>");
    }

    let result = c_strcmp(&args[0], &args[1]);
    write_c_quoted_line(
        out,
        b"strcmp('",
        &args[0],
        b"', '",
        &args[1],
        format!("') = {}\n", result).as_bytes(),
    )?;

    if result == 0 {
        write_line(out, b"Strings are equal")
    } else if result < 0 {
        write_bytes(out, b"'")?;
        write_bytes(out, &args[0])?;
        write_bytes(out, b"' < '")?;
        write_bytes(out, &args[1])?;
        write_line(out, b"'")
    } else {
        write_bytes(out, b"'")?;
        write_bytes(out, &args[0])?;
        write_bytes(out, b"' > '")?;
        write_bytes(out, &args[1])?;
        write_line(out, b"'")
    }
}

fn cmd_compare_n(args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.len() < 3 {
        return write_line(out, b"Usage: compareN <string1> <string2> <n>");
    }

    let n = c_atoi(&args[2]);
    let result = c_strncmp(&args[0], &args[1], n as usize);
    write_c_quoted_line(
        out,
        b"strncmp('",
        &args[0],
        b"', '",
        &args[1],
        format!("', {}) = {}\n", n, result).as_bytes(),
    )?;

    if result == 0 {
        write_line(out, format!("First {} characters are equal", n).as_bytes())
    } else if result < 0 {
        write_bytes(out, b"'")?;
        write_bytes(out, &args[0])?;
        write_bytes(out, b"' < '")?;
        write_bytes(out, &args[1])?;
        write_line(out, format!("' (first {} chars)", n).as_bytes())
    } else {
        write_bytes(out, b"'")?;
        write_bytes(out, &args[0])?;
        write_bytes(out, b"' > '")?;
        write_bytes(out, &args[1])?;
        write_line(out, format!("' (first {} chars)", n).as_bytes())
    }
}

fn cmd_startswith(args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.len() < 2 {
        return write_line(out, b"Usage: startswith <string> <prefix>");
    }

    let prefix_len = args[1].len();
    if c_strncmp(&args[0], &args[1], prefix_len) == 0 {
        write_bytes(out, b"'")?;
        write_bytes(out, &args[0])?;
        write_bytes(out, b"' starts with '")?;
        write_bytes(out, &args[1])?;
        write_line(out, b"'")
    } else {
        write_bytes(out, b"'")?;
        write_bytes(out, &args[0])?;
        write_bytes(out, b"' does not start with '")?;
        write_bytes(out, &args[1])?;
        write_line(out, b"'")
    }
}

fn cmd_match(args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.len() < 2 {
        return write_line(out, b"Usage: match <pattern> <string1> [string2] ...");
    }

    write_bytes(out, b"Matching pattern '")?;
    write_bytes(out, &args[0])?;
    write_line(out, b"':")?;

    let mut matches = 0i32;
    for arg in &args[1..] {
        write_bytes(out, b"  '")?;
        write_bytes(out, arg)?;
        if c_strcmp(&args[0], arg) == 0 {
            matches += 1;
            write_line(out, b"' - EXACT MATCH")?;
        } else if c_strstr(arg, &args[0]) {
            matches += 1;
            write_line(out, b"' - contains pattern")?;
        } else {
            write_line(out, b"' - no match")?;
        }
    }

    write_line(out, format!("Total matches: {}", matches).as_bytes())
}

fn cmd_help(out: &mut impl Write) -> io::Result<()> {
    write_line(out, b"")?;
    write_line(out, b"=== Command Interpreter Help ===")?;
    write_line(out, b"User Management:")?;
    write_line(out, b"  adduser <user> <pass> [level] - Add new user")?;
    write_line(out, b"  login <user> <pass>            - Login as user")?;
    write_line(out, b"  logout                         - Logout current user")?;
    write_line(out, b"  whoami                         - Show current user")?;
    write_line(out, b"  listusers                      - List all users")?;
    write_line(out, b"")?;
    write_line(out, b"File Management:")?;
    write_line(out, b"  createfile <name> [content]    - Create file")?;
    write_line(out, b"  readfile <name>                - Read file")?;
    write_line(out, b"  writefile <name> <content>     - Write to file")?;
    write_line(out, b"  deletefile <name>              - Delete file")?;
    write_line(out, b"  listfiles                      - List all files")?;
    write_line(out, b"")?;
    write_line(out, b"Variable Management:")?;
    write_line(out, b"  set <name> <value>             - Set variable")?;
    write_line(out, b"  get <name>                     - Get variable")?;
    write_line(out, b"  unset <name>                   - Unset variable")?;
    write_line(out, b"  listvars                       - List all variables")?;
    write_line(out, b"")?;
    write_line(out, b"String Operations:")?;
    write_line(out, b"  compare <str1> <str2>          - Compare strings")?;
    write_line(out, b"  compareN <str1> <str2> <n>     - Compare first N chars")?;
    write_line(out, b"  startswith <str> <prefix>      - Check if starts with")?;
    write_line(out, b"  match <pattern> <str> ...      - Match pattern")?;
    write_line(out, b"")?;
    write_line(out, b"System:")?;
    write_line(out, b"  debug [on|off]                 - Toggle debug mode")?;
    write_line(out, b"  verbose [on|off]               - Toggle verbose mode")?;
    write_line(out, b"  status                         - Show system status")?;
    write_line(out, b"  time                           - Show current time")?;
    write_line(out, b"  help                           - Show this help")?;
    write_line(out, b"  exit                           - Exit program")
}

fn cmd_debug(state: &mut State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.is_empty() {
        return write_line(
            out,
            if state.debug_mode {
                b"Debug mode: ON"
            } else {
                b"Debug mode: OFF"
            },
        );
    }

    if c_strcmp(&args[0], b"on") == 0 {
        state.debug_mode = true;
        write_line(out, b"Debug mode enabled")
    } else if c_strcmp(&args[0], b"off") == 0 {
        state.debug_mode = false;
        write_line(out, b"Debug mode disabled")
    } else {
        write_line(out, b"Usage: debug [on|off]")
    }
}

fn cmd_verbose(state: &mut State, args: &[Vec<u8>], out: &mut impl Write) -> io::Result<()> {
    if args.is_empty() {
        return write_line(
            out,
            if state.verbose_mode {
                b"Verbose mode: ON"
            } else {
                b"Verbose mode: OFF"
            },
        );
    }

    if c_strcmp(&args[0], b"on") == 0 {
        state.verbose_mode = true;
        write_line(out, b"Verbose mode enabled")
    } else if c_strcmp(&args[0], b"off") == 0 {
        state.verbose_mode = false;
        write_line(out, b"Verbose mode disabled")
    } else {
        write_line(out, b"Usage: verbose [on|off]")
    }
}

fn cmd_status(state: &State, out: &mut impl Write) -> io::Result<()> {
    write_line(out, b"")?;
    write_line(out, b"=== System Status ===")?;
    write_line(
        out,
        format!("Users: {}/{}", state.users.len(), MAX_USERS).as_bytes(),
    )?;
    write_line(
        out,
        format!("Files: {}/{}", state.files.len(), MAX_FILES).as_bytes(),
    )?;
    write_line(
        out,
        format!("Variables: {}/{}", state.variables.len(), MAX_VARIABLES).as_bytes(),
    )?;
    write_bytes(out, b"Current user: ")?;
    if let Some(user) = state.current_user() {
        write_bytes(out, &user.name)?;
    } else {
        write_bytes(out, b"none")?;
    }
    write_line(out, b"")?;
    write_line(
        out,
        if state.debug_mode {
            b"Debug mode: ON"
        } else {
            b"Debug mode: OFF"
        },
    )?;
    write_line(
        out,
        if state.verbose_mode {
            b"Verbose mode: ON"
        } else {
            b"Verbose mode: OFF"
        },
    )
}

fn cmd_time(out: &mut impl Write) -> io::Result<()> {
    let mut now: libc::time_t = 0;
    unsafe {
        libc::time(&mut now);
        let time_str = ctime(&now);
        if !time_str.is_null() {
            write_bytes(out, b"Current time: ")?;
            let bytes = std::ffi::CStr::from_ptr(time_str).to_bytes();
            write_bytes(out, bytes)?;
        }
    }
    Ok(())
}

fn process_command(state: &mut State, input: &[u8], out: &mut impl Write) -> io::Result<bool> {
    let (command, args) = parse_command(input);

    if command.is_empty() {
        return Ok(true);
    }

    if state.debug_mode {
        write_line(
            out,
            format!(
                "[DEBUG] Command: '{}', Args: {}",
                String::from_utf8_lossy(&command),
                args.len()
            )
            .as_bytes(),
        )?;
    }

    if c_strcmp(&command, b"adduser") == 0 {
        cmd_adduser(state, &args, out)?;
    } else if c_strcmp(&command, b"login") == 0 {
        cmd_login(state, &args, out)?;
    } else if c_strcmp(&command, b"logout") == 0 {
        cmd_logout(state, out)?;
    } else if c_strcmp(&command, b"whoami") == 0 {
        cmd_whoami(state, out)?;
    } else if c_strcmp(&command, b"listusers") == 0 || c_strcmp(&command, b"users") == 0 {
        cmd_listusers(state, out)?;
    } else if c_strcmp(&command, b"createfile") == 0 || c_strcmp(&command, b"touch") == 0 {
        cmd_createfile(state, &args, out)?;
    } else if c_strcmp(&command, b"readfile") == 0 || c_strcmp(&command, b"cat") == 0 {
        cmd_readfile(state, &args, out)?;
    } else if c_strcmp(&command, b"writefile") == 0 || c_strcmp(&command, b"write") == 0 {
        cmd_writefile(state, &args, out)?;
    } else if c_strcmp(&command, b"deletefile") == 0 || c_strcmp(&command, b"rm") == 0 {
        cmd_deletefile(state, &args, out)?;
    } else if c_strcmp(&command, b"listfiles") == 0 || c_strcmp(&command, b"ls") == 0 {
        cmd_listfiles(state, out)?;
    } else if c_strcmp(&command, b"set") == 0 {
        cmd_set(state, &args, out)?;
    } else if c_strcmp(&command, b"get") == 0 {
        cmd_get(state, &args, out)?;
    } else if c_strcmp(&command, b"unset") == 0 {
        cmd_unset(state, &args, out)?;
    } else if c_strcmp(&command, b"listvars") == 0 || c_strcmp(&command, b"vars") == 0 {
        cmd_listvars(state, out)?;
    } else if c_strcmp(&command, b"compare") == 0 || c_strcmp(&command, b"cmp") == 0 {
        cmd_compare(&args, out)?;
    } else if c_strcmp(&command, b"compareN") == 0 || c_strcmp(&command, b"cmpn") == 0 {
        cmd_compare_n(&args, out)?;
    } else if c_strcmp(&command, b"startswith") == 0 {
        cmd_startswith(&args, out)?;
    } else if c_strcmp(&command, b"match") == 0 {
        cmd_match(&args, out)?;
    } else if c_strcmp(&command, b"debug") == 0 {
        cmd_debug(state, &args, out)?;
    } else if c_strcmp(&command, b"verbose") == 0 {
        cmd_verbose(state, &args, out)?;
    } else if c_strcmp(&command, b"status") == 0 {
        cmd_status(state, out)?;
    } else if c_strcmp(&command, b"time") == 0 {
        cmd_time(out)?;
    } else if c_strcmp(&command, b"help") == 0 || c_strcmp(&command, b"?") == 0 {
        cmd_help(out)?;
    } else if c_strcmp(&command, b"exit") == 0 || c_strcmp(&command, b"quit") == 0 {
        write_line(out, b"Goodbye!")?;
        return Ok(false);
    } else if c_strncmp(&command, b"add", 3) == 0 {
        write_line(out, b"Did you mean 'adduser'?")?;
    } else if c_strncmp(&command, b"log", 3) == 0 {
        write_line(out, b"Did you mean 'login' or 'logout'?")?;
    } else if c_strncmp(&command, b"list", 4) == 0 {
        write_line(out, b"Did you mean 'listusers', 'listfiles', or 'listvars'?")?;
    } else if c_strncmp(&command, b"create", 6) == 0 {
        write_line(out, b"Did you mean 'createfile'?")?;
    } else if c_strncmp(&command, b"read", 4) == 0 {
        write_line(out, b"Did you mean 'readfile'?")?;
    } else if c_strncmp(&command, b"write", 5) == 0 {
        write_line(out, b"Did you mean 'writefile'?")?;
    } else if c_strncmp(&command, b"delete", 6) == 0 {
        write_line(out, b"Did you mean 'deletefile'?")?;
    } else {
        write_bytes(out, b"Unknown command: '")?;
        write_bytes(out, &command)?;
        write_line(out, b"'. Type 'help' for available commands.")?;
    }

    Ok(true)
}

fn read_fgets_like(reader: &mut impl Read, max_input: usize) -> io::Result<Option<Vec<u8>>> {
    let mut buf = Vec::with_capacity(max_input.saturating_sub(1));
    let mut byte = [0u8; 1];

    while buf.len() < max_input.saturating_sub(1) {
        match reader.read(&mut byte)? {
            0 => {
                if buf.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(buf));
            }
            _ => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    return Ok(Some(buf));
                }
            }
        }
    }

    Ok(Some(buf))
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let mut state = State::new();

    write_line(&mut out, b"|----------------------------------------|")?;
    write_line(&mut out, b"|   COMMAND INTERPRETER                  |")?;
    write_line(&mut out, b"|   strcmp/strncmp demonstration         |")?;
    write_line(&mut out, b"|----------------------------------------|")?;
    write_line(&mut out, b"Type 'help' for available commands")?;
    write_line(&mut out, b"")?;

    while let Some(mut line) = read_fgets_like(&mut input, MAX_INPUT)? {
        write_bytes(&mut out, b"> ")?;
        out.flush()?;

        if let Some(pos) = line.iter().position(|&b| b == b'\n') {
            line[pos] = 0;
            line.truncate(pos);
        }

        if state.verbose_mode {
            write_bytes(&mut out, b"[VERBOSE] Processing: '")?;
            write_bytes(&mut out, &line)?;
            write_line(&mut out, b"'")?;
        }

        if !process_command(&mut state, &line, &mut out)? {
            break;
        }
    }

    out.flush()
}
