// Rust translation of c_src/src/main.c
//
// The original C program is a toy command interpreter that demonstrates
// strcmp/strncmp. This translation reproduces its observable behaviour
// byte-for-byte, including:
//   * `fgets`-based line reading (255 byte chunks, long lines are split and
//     each chunk gets its own "> " prompt)
//   * `strtok` tokenisation on ' ' and '\t' only
//   * glibc `strcmp`/`strncmp` return values (difference of unsigned chars)
//   * C `atoi` semantics
//   * the fixed-size, un-bounds-checked string buffers of the original
//     structs: the C code uses `strcpy` into `char[32]` fields with tokens of
//     up to 63 characters, so a long name spills into the neighbouring field.
//     The tables are therefore modelled as flat byte arrays with the exact
//     x86-64 struct layout so that this overflow behaviour is preserved.

use std::io::{self, BufRead, BufReader, Write};

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

// ---------------------------------------------------------------------------
// struct layouts (x86-64)
// ---------------------------------------------------------------------------

// typedef struct { char name[32]; char password[32]; int permission_level; int logged_in; } user_t;
const U_NAME: usize = 0;
const U_PASS: usize = 32;
const U_PERM: usize = 64;
const U_LOGGED: usize = 68;
const U_SZ: usize = 72;

// typedef struct { char filename[64]; char content[512]; char owner[32]; int permissions; } file_t;
const F_NAME: usize = 0;
const F_CONTENT: usize = 64;
const F_OWNER: usize = 576;
const F_PERM: usize = 608;
const F_SZ: usize = 612;

// typedef struct { char name[32]; char value[128]; } variable_t;
const V_NAME: usize = 0;
const V_VALUE: usize = 32;
const V_SZ: usize = 160;

// A `strcpy` of a 63 character token into the last element of a table can run
// a few bytes past the end of the table; in C that lands in unrelated padding
// of the BSS segment. Model that with slack bytes.
const SLACK: usize = 64;

// ---------------------------------------------------------------------------
// C string helpers
// ---------------------------------------------------------------------------

/// Contents of a NUL terminated C string starting at the front of `buf`.
fn cstr(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == 0) {
        Some(i) => &buf[..i],
        None => buf,
    }
}

fn c_strlen(buf: &[u8]) -> usize {
    cstr(buf).len()
}

/// `strncpy(dst, src, n)` where `src` is the string contents (no NUL).
/// Copies at most `n` bytes and zero pads the remainder of the `n` bytes.
fn c_strncpy(dst: &mut [u8], src: &[u8], n: usize) {
    let copy = src.len().min(n);
    dst[..copy].copy_from_slice(&src[..copy]);
    for b in dst[copy..n].iter_mut() {
        *b = 0;
    }
}

/// `strcpy(&dst[off], src)` where `src` is the string contents (no NUL).
fn c_strcpy(dst: &mut [u8], off: usize, src: &[u8]) {
    dst[off..off + src.len()].copy_from_slice(src);
    dst[off + src.len()] = 0;
}

/// glibc `strcmp`: difference of the first differing unsigned chars.
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let a = cstr(a);
    let b = cstr(b);
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return a[i] as i32 - b[i] as i32;
        }
    }
    if a.len() == b.len() {
        0
    } else if a.len() < b.len() {
        0 - b[n] as i32
    } else {
        a[n] as i32
    }
}

/// glibc `strncmp`.
fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    let a = cstr(a);
    let b = cstr(b);
    let mut i = 0usize;
    while i < n {
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

/// `strstr(haystack, needle) != NULL`
fn c_strstr(haystack: &[u8], needle: &[u8]) -> bool {
    let h = cstr(haystack);
    let n = cstr(needle);
    if n.is_empty() {
        return true;
    }
    if n.len() > h.len() {
        return false;
    }
    (0..=h.len() - n.len()).any(|i| &h[i..i + n.len()] == n)
}

/// C `atoi` (glibc: `(int)strtol(s, NULL, 10)`, saturating in `long`).
fn c_atoi(s: &[u8]) -> i32 {
    let s = cstr(s);
    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut acc: i64 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        i += 1;
    }
    if saturated {
        return if neg { i64::MIN as i32 } else { i64::MAX as i32 };
    }
    let v: i64 = if neg { -acc } else { acc };
    v as i32
}

fn get_i32(buf: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

fn set_i32(buf: &mut [u8], off: usize, v: i32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

// ---------------------------------------------------------------------------
// output
// ---------------------------------------------------------------------------

struct Out {
    w: io::BufWriter<io::Stdout>,
}

impl Out {
    fn new() -> Out {
        Out {
            w: io::BufWriter::new(io::stdout()),
        }
    }
    fn b(&mut self, s: &[u8]) {
        let _ = self.w.write_all(s);
    }
    fn s(&mut self, s: &str) {
        self.b(s.as_bytes());
    }
    fn d(&mut self, v: i32) {
        self.s(&v.to_string());
    }
    fn flush(&mut self) {
        let _ = self.w.flush();
    }
}

// ---------------------------------------------------------------------------
// interpreter state
// ---------------------------------------------------------------------------

struct App {
    users: Vec<u8>,
    user_count: i32,
    current_user: Option<usize>,

    files: Vec<u8>,
    file_count: i32,

    variables: Vec<u8>,
    variable_count: i32,

    debug_mode: i32,
    verbose_mode: i32,

    out: Out,
}

impl App {
    fn new() -> App {
        App {
            users: vec![0u8; MAX_USERS * U_SZ + SLACK],
            user_count: 0,
            current_user: None,
            files: vec![0u8; MAX_FILES * F_SZ + SLACK],
            file_count: 0,
            variables: vec![0u8; MAX_VARIABLES * V_SZ + SLACK],
            variable_count: 0,
            debug_mode: 0,
            verbose_mode: 0,
            out: Out::new(),
        }
    }

    // --- field accessors -------------------------------------------------
    fn user_field(&self, i: usize, off: usize) -> &[u8] {
        cstr(&self.users[i * U_SZ + off..])
    }
    fn file_field(&self, i: usize, off: usize) -> &[u8] {
        cstr(&self.files[i * F_SZ + off..])
    }
    fn var_field(&self, i: usize, off: usize) -> &[u8] {
        cstr(&self.variables[i * V_SZ + off..])
    }

    fn logged_in(&self) -> bool {
        // `current_user && current_user->logged_in`
        match self.current_user {
            Some(i) => get_i32(&self.users, i * U_SZ + U_LOGGED) != 0,
            None => false,
        }
    }

    // --- user management -------------------------------------------------
    fn cmd_adduser(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 2 {
            self.out
                .s("Usage: adduser <username> <password> [permission_level]\n");
            return;
        }

        if self.user_count as usize >= MAX_USERS {
            self.out.s("Error: Maximum users reached\n");
            return;
        }

        for i in 0..self.user_count as usize {
            if c_strcmp(self.user_field(i, U_NAME), &args[0]) == 0 {
                self.out.s("Error: User '");
                self.out.b(cstr(&args[0]));
                self.out.s("' already exists\n");
                return;
            }
        }

        let idx = self.user_count as usize;
        let name = cstr(&args[0]).to_vec();
        let pass = cstr(&args[1]).to_vec();
        c_strcpy(&mut self.users, idx * U_SZ + U_NAME, &name);
        c_strcpy(&mut self.users, idx * U_SZ + U_PASS, &pass);
        let level = if arg_count >= 3 { c_atoi(&args[2]) } else { 1 };
        set_i32(&mut self.users, idx * U_SZ + U_PERM, level);
        set_i32(&mut self.users, idx * U_SZ + U_LOGGED, 0);
        self.user_count += 1;

        let level = get_i32(&self.users, (self.user_count as usize - 1) * U_SZ + U_PERM);
        self.out.s("User '");
        self.out.b(cstr(&args[0]));
        self.out.s("' added with permission level ");
        self.out.d(level);
        self.out.s("\n");
    }

    fn cmd_login(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 2 {
            self.out.s("Usage: login <username> <password>\n");
            return;
        }

        if self.logged_in() {
            let cu = self.current_user.unwrap();
            let name = self.user_field(cu, U_NAME).to_vec();
            self.out.s("Error: User '");
            self.out.b(&name);
            self.out.s("' already logged in. Use 'logout' first.\n");
            return;
        }

        for i in 0..self.user_count as usize {
            if c_strcmp(self.user_field(i, U_NAME), &args[0]) == 0 {
                if c_strcmp(self.user_field(i, U_PASS), &args[1]) == 0 {
                    set_i32(&mut self.users, i * U_SZ + U_LOGGED, 1);
                    self.current_user = Some(i);
                    let name = self.user_field(i, U_NAME).to_vec();
                    self.out.s("Login successful. Welcome, ");
                    self.out.b(&name);
                    self.out.s("!\n");
                    return;
                } else {
                    self.out.s("Error: Incorrect password\n");
                    return;
                }
            }
        }

        self.out.s("Error: User not found\n");
    }

    fn cmd_logout(&mut self) {
        if !self.logged_in() {
            self.out.s("Error: No user logged in\n");
            return;
        }

        let cu = self.current_user.unwrap();
        let name = self.user_field(cu, U_NAME).to_vec();
        self.out.s("Goodbye, ");
        self.out.b(&name);
        self.out.s("!\n");
        set_i32(&mut self.users, cu * U_SZ + U_LOGGED, 0);
        self.current_user = None;
    }

    fn cmd_whoami(&mut self) {
        if !self.logged_in() {
            self.out.s("Not logged in\n");
            return;
        }

        let cu = self.current_user.unwrap();
        let name = self.user_field(cu, U_NAME).to_vec();
        let level = get_i32(&self.users, cu * U_SZ + U_PERM);
        self.out.s("Current user: ");
        self.out.b(&name);
        self.out.s("\n");
        self.out.s("Permission level: ");
        self.out.d(level);
        self.out.s("\n");
    }

    fn cmd_listusers(&mut self) {
        if self.user_count == 0 {
            self.out.s("No users registered\n");
            return;
        }

        self.out.s("Registered users:\n");
        for i in 0..self.user_count as usize {
            let name = self.user_field(i, U_NAME).to_vec();
            let level = get_i32(&self.users, i * U_SZ + U_PERM);
            let logged = get_i32(&self.users, i * U_SZ + U_LOGGED) != 0;
            self.out.s("  ");
            self.out.b(&name);
            self.out.s(" (level ");
            self.out.d(level);
            self.out.s(") ");
            self.out.s(if logged { "[logged in]" } else { "" });
            self.out.s("\n");
        }
    }

    // --- file management -------------------------------------------------
    fn cmd_createfile(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if !self.logged_in() {
            self.out.s("Error: Must be logged in\n");
            return;
        }

        if arg_count < 1 {
            self.out.s("Usage: createfile <filename> [content]\n");
            return;
        }

        if self.file_count as usize >= MAX_FILES {
            self.out.s("Error: Maximum files reached\n");
            return;
        }

        for i in 0..self.file_count as usize {
            if c_strcmp(self.file_field(i, F_NAME), &args[0]) == 0 {
                self.out.s("Error: File '");
                self.out.b(cstr(&args[0]));
                self.out.s("' already exists\n");
                return;
            }
        }

        let idx = self.file_count as usize;
        let fname = cstr(&args[0]).to_vec();
        let owner = self.user_field(self.current_user.unwrap(), U_NAME).to_vec();
        c_strcpy(&mut self.files, idx * F_SZ + F_NAME, &fname);
        c_strcpy(&mut self.files, idx * F_SZ + F_OWNER, &owner);
        set_i32(&mut self.files, idx * F_SZ + F_PERM, 755);

        if arg_count >= 2 {
            let content = cstr(&args[1]).to_vec();
            c_strcpy(&mut self.files, idx * F_SZ + F_CONTENT, &content);
        } else {
            self.files[idx * F_SZ + F_CONTENT] = 0;
        }

        self.file_count += 1;
        self.out.s("File '");
        self.out.b(cstr(&args[0]));
        self.out.s("' created\n");
    }

    fn cmd_readfile(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 1 {
            self.out.s("Usage: readfile <filename>\n");
            return;
        }

        for i in 0..self.file_count as usize {
            if c_strcmp(self.file_field(i, F_NAME), &args[0]) == 0 {
                let fname = self.file_field(i, F_NAME).to_vec();
                let owner = self.file_field(i, F_OWNER).to_vec();
                let content = self.file_field(i, F_CONTENT).to_vec();
                let perm = get_i32(&self.files, i * F_SZ + F_PERM);
                self.out.s("=== ");
                self.out.b(&fname);
                self.out.s(" ===\n");
                self.out.s("Owner: ");
                self.out.b(&owner);
                self.out.s("\n");
                self.out.s("Permissions: ");
                self.out.d(perm);
                self.out.s("\n");
                self.out.s("Content: ");
                self.out.b(&content);
                self.out.s("\n");
                return;
            }
        }

        self.out.s("Error: File '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_writefile(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if !self.logged_in() {
            self.out.s("Error: Must be logged in\n");
            return;
        }

        if arg_count < 2 {
            self.out.s("Usage: writefile <filename> <content>\n");
            return;
        }

        let cu = self.current_user.unwrap();
        for i in 0..self.file_count as usize {
            if c_strcmp(self.file_field(i, F_NAME), &args[0]) == 0 {
                let cu_name = self.user_field(cu, U_NAME).to_vec();
                let cu_level = get_i32(&self.users, cu * U_SZ + U_PERM);
                if c_strcmp(self.file_field(i, F_OWNER), &cu_name) == 0 || cu_level >= 5 {
                    let content = cstr(&args[1]).to_vec();
                    c_strcpy(&mut self.files, i * F_SZ + F_CONTENT, &content);
                    self.out.s("File '");
                    self.out.b(cstr(&args[0]));
                    self.out.s("' updated\n");
                    return;
                } else {
                    self.out.s("Error: Permission denied\n");
                    return;
                }
            }
        }

        self.out.s("Error: File '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_deletefile(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if !self.logged_in() {
            self.out.s("Error: Must be logged in\n");
            return;
        }

        if arg_count < 1 {
            self.out.s("Usage: deletefile <filename>\n");
            return;
        }

        let cu = self.current_user.unwrap();
        for i in 0..self.file_count as usize {
            if c_strcmp(self.file_field(i, F_NAME), &args[0]) == 0 {
                let cu_name = self.user_field(cu, U_NAME).to_vec();
                let cu_level = get_i32(&self.users, cu * U_SZ + U_PERM);
                if c_strcmp(self.file_field(i, F_OWNER), &cu_name) == 0 || cu_level >= 9 {
                    // Shift remaining files
                    for j in i..(self.file_count as usize - 1) {
                        self.files
                            .copy_within((j + 1) * F_SZ..(j + 2) * F_SZ, j * F_SZ);
                    }
                    self.file_count -= 1;
                    self.out.s("File '");
                    self.out.b(cstr(&args[0]));
                    self.out.s("' deleted\n");
                    return;
                } else {
                    self.out.s("Error: Permission denied\n");
                    return;
                }
            }
        }

        self.out.s("Error: File '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_listfiles(&mut self) {
        if self.file_count == 0 {
            self.out.s("No files\n");
            return;
        }

        self.out.s("Files:\n");
        for i in 0..self.file_count as usize {
            let fname = self.file_field(i, F_NAME).to_vec();
            let owner = self.file_field(i, F_OWNER).to_vec();
            let perm = get_i32(&self.files, i * F_SZ + F_PERM);
            self.out.s("  ");
            self.out.b(&fname);
            self.out.s(" (owner: ");
            self.out.b(&owner);
            self.out.s(", perm: ");
            self.out.d(perm);
            self.out.s(")\n");
        }
    }

    // --- variables -------------------------------------------------------
    fn cmd_set(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 2 {
            self.out.s("Usage: set <name> <value>\n");
            return;
        }

        for i in 0..self.variable_count as usize {
            if c_strcmp(self.var_field(i, V_NAME), &args[0]) == 0 {
                let value = cstr(&args[1]).to_vec();
                c_strcpy(&mut self.variables, i * V_SZ + V_VALUE, &value);
                self.out.s("Variable '");
                self.out.b(cstr(&args[0]));
                self.out.s("' updated\n");
                return;
            }
        }

        if self.variable_count as usize >= MAX_VARIABLES {
            self.out.s("Error: Maximum variables reached\n");
            return;
        }

        let idx = self.variable_count as usize;
        let name = cstr(&args[0]).to_vec();
        let value = cstr(&args[1]).to_vec();
        c_strcpy(&mut self.variables, idx * V_SZ + V_NAME, &name);
        c_strcpy(&mut self.variables, idx * V_SZ + V_VALUE, &value);
        self.variable_count += 1;
        self.out.s("Variable '");
        self.out.b(cstr(&args[0]));
        self.out.s("' set\n");
    }

    fn cmd_get(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 1 {
            self.out.s("Usage: get <name>\n");
            return;
        }

        for i in 0..self.variable_count as usize {
            if c_strcmp(self.var_field(i, V_NAME), &args[0]) == 0 {
                let name = self.var_field(i, V_NAME).to_vec();
                let value = self.var_field(i, V_VALUE).to_vec();
                self.out.b(&name);
                self.out.s(" = ");
                self.out.b(&value);
                self.out.s("\n");
                return;
            }
        }

        self.out.s("Error: Variable '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_unset(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 1 {
            self.out.s("Usage: unset <name>\n");
            return;
        }

        for i in 0..self.variable_count as usize {
            if c_strcmp(self.var_field(i, V_NAME), &args[0]) == 0 {
                for j in i..(self.variable_count as usize - 1) {
                    self.variables
                        .copy_within((j + 1) * V_SZ..(j + 2) * V_SZ, j * V_SZ);
                }
                self.variable_count -= 1;
                self.out.s("Variable '");
                self.out.b(cstr(&args[0]));
                self.out.s("' unset\n");
                return;
            }
        }

        self.out.s("Error: Variable '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_listvars(&mut self) {
        if self.variable_count == 0 {
            self.out.s("No variables set\n");
            return;
        }

        self.out.s("Variables:\n");
        for i in 0..self.variable_count as usize {
            let name = self.var_field(i, V_NAME).to_vec();
            let value = self.var_field(i, V_VALUE).to_vec();
            self.out.s("  ");
            self.out.b(&name);
            self.out.s(" = ");
            self.out.b(&value);
            self.out.s("\n");
        }
    }

    // --- string comparison ----------------------------------------------
    fn cmd_compare(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 2 {
            self.out.s("Usage: compare <string1> <string2>\n");
            return;
        }

        let result = c_strcmp(&args[0], &args[1]);
        let a = cstr(&args[0]).to_vec();
        let b = cstr(&args[1]).to_vec();

        self.out.s("strcmp('");
        self.out.b(&a);
        self.out.s("', '");
        self.out.b(&b);
        self.out.s("') = ");
        self.out.d(result);
        self.out.s("\n");

        if result == 0 {
            self.out.s("Strings are equal\n");
        } else if result < 0 {
            self.out.s("'");
            self.out.b(&a);
            self.out.s("' < '");
            self.out.b(&b);
            self.out.s("'\n");
        } else {
            self.out.s("'");
            self.out.b(&a);
            self.out.s("' > '");
            self.out.b(&b);
            self.out.s("'\n");
        }
    }

    fn cmd_compare_n(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 3 {
            self.out.s("Usage: compareN <string1> <string2> <n>\n");
            return;
        }

        let n = c_atoi(&args[2]);
        // `int` -> `size_t` conversion sign extends, as in the C code.
        let result = c_strncmp(&args[0], &args[1], n as i64 as u64 as usize);
        let a = cstr(&args[0]).to_vec();
        let b = cstr(&args[1]).to_vec();

        self.out.s("strncmp('");
        self.out.b(&a);
        self.out.s("', '");
        self.out.b(&b);
        self.out.s("', ");
        self.out.d(n);
        self.out.s(") = ");
        self.out.d(result);
        self.out.s("\n");

        if result == 0 {
            self.out.s("First ");
            self.out.d(n);
            self.out.s(" characters are equal\n");
        } else if result < 0 {
            self.out.s("'");
            self.out.b(&a);
            self.out.s("' < '");
            self.out.b(&b);
            self.out.s("' (first ");
            self.out.d(n);
            self.out.s(" chars)\n");
        } else {
            self.out.s("'");
            self.out.b(&a);
            self.out.s("' > '");
            self.out.b(&b);
            self.out.s("' (first ");
            self.out.d(n);
            self.out.s(" chars)\n");
        }
    }

    fn cmd_startswith(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 2 {
            self.out.s("Usage: startswith <string> <prefix>\n");
            return;
        }

        let prefix_len = c_strlen(&args[1]);
        let a = cstr(&args[0]).to_vec();
        let b = cstr(&args[1]).to_vec();

        if c_strncmp(&args[0], &args[1], prefix_len) == 0 {
            self.out.s("'");
            self.out.b(&a);
            self.out.s("' starts with '");
            self.out.b(&b);
            self.out.s("'\n");
        } else {
            self.out.s("'");
            self.out.b(&a);
            self.out.s("' does not start with '");
            self.out.b(&b);
            self.out.s("'\n");
        }
    }

    fn cmd_match(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 2 {
            self.out.s("Usage: match <pattern> <string1> [string2] ...\n");
            return;
        }

        let pattern = cstr(&args[0]).to_vec();
        self.out.s("Matching pattern '");
        self.out.b(&pattern);
        self.out.s("':\n");
        let mut matches = 0i32;

        for i in 1..arg_count as usize {
            let s = cstr(&args[i]).to_vec();
            if c_strcmp(&args[0], &args[i]) == 0 {
                self.out.s("  '");
                self.out.b(&s);
                self.out.s("' - EXACT MATCH\n");
                matches += 1;
            } else if c_strstr(&args[i], &args[0]) {
                self.out.s("  '");
                self.out.b(&s);
                self.out.s("' - contains pattern\n");
                matches += 1;
            } else {
                self.out.s("  '");
                self.out.b(&s);
                self.out.s("' - no match\n");
            }
        }

        self.out.s("Total matches: ");
        self.out.d(matches);
        self.out.s("\n");
    }

    // --- system ----------------------------------------------------------
    fn cmd_help(&mut self) {
        self.out.s("\n=== Command Interpreter Help ===\n");
        self.out.s("User Management:\n");
        self.out.s("  adduser <user> <pass> [level] - Add new user\n");
        self.out.s("  login <user> <pass>            - Login as user\n");
        self.out.s("  logout                         - Logout current user\n");
        self.out.s("  whoami                         - Show current user\n");
        self.out.s("  listusers                      - List all users\n");
        self.out.s("\nFile Management:\n");
        self.out.s("  createfile <name> [content]    - Create file\n");
        self.out.s("  readfile <name>                - Read file\n");
        self.out.s("  writefile <name> <content>     - Write to file\n");
        self.out.s("  deletefile <name>              - Delete file\n");
        self.out.s("  listfiles                      - List all files\n");
        self.out.s("\nVariable Management:\n");
        self.out.s("  set <name> <value>             - Set variable\n");
        self.out.s("  get <name>                     - Get variable\n");
        self.out.s("  unset <name>                   - Unset variable\n");
        self.out.s("  listvars                       - List all variables\n");
        self.out.s("\nString Operations:\n");
        self.out.s("  compare <str1> <str2>          - Compare strings\n");
        self.out.s("  compareN <str1> <str2> <n>     - Compare first N chars\n");
        self.out.s("  startswith <str> <prefix>      - Check if starts with\n");
        self.out.s("  match <pattern> <str> ...      - Match pattern\n");
        self.out.s("\nSystem:\n");
        self.out.s("  debug [on|off]                 - Toggle debug mode\n");
        self.out.s("  verbose [on|off]               - Toggle verbose mode\n");
        self.out.s("  status                         - Show system status\n");
        self.out.s("  time                           - Show current time\n");
        self.out.s("  help                           - Show this help\n");
        self.out.s("  exit                           - Exit program\n");
    }

    fn cmd_debug(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 1 {
            self.out.s("Debug mode: ");
            self.out.s(if self.debug_mode != 0 { "ON" } else { "OFF" });
            self.out.s("\n");
            return;
        }

        if c_strcmp(&args[0], b"on") == 0 {
            self.debug_mode = 1;
            self.out.s("Debug mode enabled\n");
        } else if c_strcmp(&args[0], b"off") == 0 {
            self.debug_mode = 0;
            self.out.s("Debug mode disabled\n");
        } else {
            self.out.s("Usage: debug [on|off]\n");
        }
    }

    fn cmd_verbose(&mut self, args: &[[u8; MAX_COMMAND]; MAX_ARGS], arg_count: i32) {
        if arg_count < 1 {
            self.out.s("Verbose mode: ");
            self.out.s(if self.verbose_mode != 0 { "ON" } else { "OFF" });
            self.out.s("\n");
            return;
        }

        if c_strcmp(&args[0], b"on") == 0 {
            self.verbose_mode = 1;
            self.out.s("Verbose mode enabled\n");
        } else if c_strcmp(&args[0], b"off") == 0 {
            self.verbose_mode = 0;
            self.out.s("Verbose mode disabled\n");
        } else {
            self.out.s("Usage: verbose [on|off]\n");
        }
    }

    fn cmd_status(&mut self) {
        self.out.s("\n=== System Status ===\n");
        self.out.s("Users: ");
        self.out.d(self.user_count);
        self.out.s("/");
        self.out.d(MAX_USERS as i32);
        self.out.s("\n");
        self.out.s("Files: ");
        self.out.d(self.file_count);
        self.out.s("/");
        self.out.d(MAX_FILES as i32);
        self.out.s("\n");
        self.out.s("Variables: ");
        self.out.d(self.variable_count);
        self.out.s("/");
        self.out.d(MAX_VARIABLES as i32);
        self.out.s("\n");
        self.out.s("Current user: ");
        if self.logged_in() {
            let name = self.user_field(self.current_user.unwrap(), U_NAME).to_vec();
            self.out.b(&name);
        } else {
            self.out.s("none");
        }
        self.out.s("\n");
        self.out.s("Debug mode: ");
        self.out.s(if self.debug_mode != 0 { "ON" } else { "OFF" });
        self.out.s("\n");
        self.out.s("Verbose mode: ");
        self.out.s(if self.verbose_mode != 0 { "ON" } else { "OFF" });
        self.out.s("\n");
    }

    fn cmd_time(&mut self) {
        // `time(NULL)` / `ctime(&now)`, delegated to libc so that the local
        // timezone handling and formatting match the C program exactly.
        let text = libc_ctime_now();
        self.out.s("Current time: ");
        self.out.b(&text);
    }

    // --- dispatch --------------------------------------------------------
    fn process_command(&mut self, input: &[u8]) {
        let mut command = [0u8; MAX_COMMAND];
        let mut args = [[0u8; MAX_COMMAND]; MAX_ARGS];
        let mut arg_count: i32 = 0;

        parse_command(input, &mut command, &mut args, &mut arg_count);

        if c_strlen(&command) == 0 {
            return;
        }

        if self.debug_mode != 0 {
            self.out.s("[DEBUG] Command: '");
            self.out.b(cstr(&command));
            self.out.s("', Args: ");
            self.out.d(arg_count);
            self.out.s("\n");
        }

        let c = &command;
        if c_strcmp(c, b"adduser") == 0 {
            self.cmd_adduser(&args, arg_count);
        } else if c_strcmp(c, b"login") == 0 {
            self.cmd_login(&args, arg_count);
        } else if c_strcmp(c, b"logout") == 0 {
            self.cmd_logout();
        } else if c_strcmp(c, b"whoami") == 0 {
            self.cmd_whoami();
        } else if c_strcmp(c, b"listusers") == 0 || c_strcmp(c, b"users") == 0 {
            self.cmd_listusers();
        }
        // File commands
        else if c_strcmp(c, b"createfile") == 0 || c_strcmp(c, b"touch") == 0 {
            self.cmd_createfile(&args, arg_count);
        } else if c_strcmp(c, b"readfile") == 0 || c_strcmp(c, b"cat") == 0 {
            self.cmd_readfile(&args, arg_count);
        } else if c_strcmp(c, b"writefile") == 0 || c_strcmp(c, b"write") == 0 {
            self.cmd_writefile(&args, arg_count);
        } else if c_strcmp(c, b"deletefile") == 0 || c_strcmp(c, b"rm") == 0 {
            self.cmd_deletefile(&args, arg_count);
        } else if c_strcmp(c, b"listfiles") == 0 || c_strcmp(c, b"ls") == 0 {
            self.cmd_listfiles();
        }
        // Variable commands
        else if c_strcmp(c, b"set") == 0 {
            self.cmd_set(&args, arg_count);
        } else if c_strcmp(c, b"get") == 0 {
            self.cmd_get(&args, arg_count);
        } else if c_strcmp(c, b"unset") == 0 {
            self.cmd_unset(&args, arg_count);
        } else if c_strcmp(c, b"listvars") == 0 || c_strcmp(c, b"vars") == 0 {
            self.cmd_listvars();
        }
        // String comparison commands
        else if c_strcmp(c, b"compare") == 0 || c_strcmp(c, b"cmp") == 0 {
            self.cmd_compare(&args, arg_count);
        } else if c_strcmp(c, b"compareN") == 0 || c_strcmp(c, b"cmpn") == 0 {
            self.cmd_compare_n(&args, arg_count);
        } else if c_strcmp(c, b"startswith") == 0 {
            self.cmd_startswith(&args, arg_count);
        } else if c_strcmp(c, b"match") == 0 {
            self.cmd_match(&args, arg_count);
        }
        // System commands
        else if c_strcmp(c, b"debug") == 0 {
            self.cmd_debug(&args, arg_count);
        } else if c_strcmp(c, b"verbose") == 0 {
            self.cmd_verbose(&args, arg_count);
        } else if c_strcmp(c, b"status") == 0 {
            self.cmd_status();
        } else if c_strcmp(c, b"time") == 0 {
            self.cmd_time();
        } else if c_strcmp(c, b"help") == 0 || c_strcmp(c, b"?") == 0 {
            self.cmd_help();
        } else if c_strcmp(c, b"exit") == 0 || c_strcmp(c, b"quit") == 0 {
            self.out.s("Goodbye!\n");
            self.out.flush();
            std::process::exit(0);
        }
        // Check for partial matches using strncmp
        else if c_strncmp(c, b"add", 3) == 0 {
            self.out.s("Did you mean 'adduser'?\n");
        } else if c_strncmp(c, b"log", 3) == 0 {
            self.out.s("Did you mean 'login' or 'logout'?\n");
        } else if c_strncmp(c, b"list", 4) == 0 {
            self.out
                .s("Did you mean 'listusers', 'listfiles', or 'listvars'?\n");
        } else if c_strncmp(c, b"create", 6) == 0 {
            self.out.s("Did you mean 'createfile'?\n");
        } else if c_strncmp(c, b"read", 4) == 0 {
            self.out.s("Did you mean 'readfile'?\n");
        } else if c_strncmp(c, b"write", 5) == 0 {
            self.out.s("Did you mean 'writefile'?\n");
        } else if c_strncmp(c, b"delete", 6) == 0 {
            self.out.s("Did you mean 'deletefile'?\n");
        } else {
            self.out.s("Unknown command: '");
            self.out.b(cstr(c));
            self.out.s("'. Type 'help' for available commands.\n");
        }
    }
}

// ---------------------------------------------------------------------------
// parsing
// ---------------------------------------------------------------------------

/// `strtok`-style split on ' ' and '\t' (empty tokens are skipped).
fn tokenize(s: &[u8]) -> Vec<&[u8]> {
    s.split(|&b| b == b' ' || b == b'\t')
        .filter(|t| !t.is_empty())
        .collect()
}

fn parse_command(
    input: &[u8],
    cmd: &mut [u8; MAX_COMMAND],
    args: &mut [[u8; MAX_COMMAND]; MAX_ARGS],
    arg_count: &mut i32,
) {
    let mut temp = [0u8; MAX_INPUT];
    c_strncpy(&mut temp, cstr(input), MAX_INPUT - 1);
    temp[MAX_INPUT - 1] = 0;

    *arg_count = 0;
    let tokens = tokenize(cstr(&temp));

    if let Some(first) = tokens.first() {
        c_strncpy(cmd, first, MAX_COMMAND - 1);
        cmd[MAX_COMMAND - 1] = 0;

        for token in tokens.iter().skip(1) {
            if *arg_count as usize >= MAX_ARGS {
                break;
            }
            let slot = &mut args[*arg_count as usize];
            c_strncpy(slot, token, MAX_COMMAND - 1);
            slot[MAX_COMMAND - 1] = 0;
            *arg_count += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// stdin: fgets
// ---------------------------------------------------------------------------

/// `fgets(buf, buf.len(), stdin)`: reads at most `buf.len() - 1` bytes,
/// stopping after a newline, and NUL terminates. Returns false when nothing
/// could be read (EOF/error), matching a NULL return.
fn fgets<R: BufRead>(r: &mut R, buf: &mut [u8]) -> bool {
    let limit = buf.len() - 1;
    let mut i = 0usize;
    while i < limit {
        let byte = {
            let available = match r.fill_buf() {
                Ok(b) => b,
                Err(_) => break,
            };
            if available.is_empty() {
                break;
            }
            available[0]
        };
        r.consume(1);
        buf[i] = byte;
        i += 1;
        if byte == b'\n' {
            break;
        }
    }
    if i == 0 {
        return false;
    }
    buf[i] = 0;
    true
}

/// `input[strcspn(input, "\n")] = 0;`
fn strip_newline(buf: &mut [u8]) {
    let span = cstr(buf)
        .iter()
        .position(|&b| b == b'\n')
        .unwrap_or_else(|| c_strlen(buf));
    buf[span] = 0;
}

// ---------------------------------------------------------------------------
// libc time
// ---------------------------------------------------------------------------

extern "C" {
    fn time(tloc: *mut i64) -> i64;
    fn ctime(timep: *const i64) -> *const u8;
}

fn libc_ctime_now() -> Vec<u8> {
    unsafe {
        let now: i64 = time(std::ptr::null_mut());
        let p = ctime(&now as *const i64);
        if p.is_null() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut i = 0isize;
        loop {
            let b = *p.offset(i);
            if b == 0 {
                break;
            }
            out.push(b);
            i += 1;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let mut app = App::new();

    app.out.s("|----------------------------------------|\n");
    app.out.s("|   COMMAND INTERPRETER                  |\n");
    app.out.s("|   strcmp/strncmp demonstration         |\n");
    app.out.s("|----------------------------------------|\n");
    app.out.s("Type 'help' for available commands\n\n");

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut input = [0u8; MAX_INPUT];

    loop {
        app.out.s("> ");

        if !fgets(&mut reader, &mut input) {
            break;
        }

        strip_newline(&mut input);

        if app.verbose_mode != 0 {
            let line = cstr(&input).to_vec();
            app.out.s("[VERBOSE] Processing: '");
            app.out.b(&line);
            app.out.s("'\n");
        }

        let line = cstr(&input).to_vec();
        app.process_command(&line);
    }

    app.out.flush();
}
