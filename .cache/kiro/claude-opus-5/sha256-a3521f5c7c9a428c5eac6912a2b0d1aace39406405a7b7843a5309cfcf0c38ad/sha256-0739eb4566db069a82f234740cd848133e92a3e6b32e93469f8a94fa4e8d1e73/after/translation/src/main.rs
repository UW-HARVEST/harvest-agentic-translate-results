/*
 * Rust translation of c_src/src/main.c (MIT Lincoln Laboratory, 2025).
 *
 * The original program is a toy command interpreter that demonstrates
 * strcmp/strncmp.  It contains several genuine C defects that are faithfully
 * reproduced here because the translation must be byte-identical:
 *
 *   1. `strcpy` of up-to-63-byte tokens into 32-byte struct fields overflows
 *      into the neighbouring fields of the same struct (and, in extreme cases,
 *      into the next array element).  To reproduce that exactly, the three
 *      global arrays are modelled as flat byte buffers using the real x86-64
 *      struct layouts, and field access goes through C-string / little-endian
 *      int helpers.
 *
 *   2. `process_command` leaves its local `command` buffer uninitialised when
 *      the input line contains no tokens, so a blank line re-executes the
 *      previous command name with `arg_count == 0`.  Because `process_command`
 *      is always called from the same stack depth, the stale value is the
 *      previous iteration's command; a blank first line sees a zeroed buffer.
 *      Modelled with persistent buffers that start out zeroed.
 *
 *   3. glibc's `strcmp`/`strncmp` return the difference of the first differing
 *      bytes taken as `unsigned char`, and that value is printed with `%d`.
 *
 *   4. `atoi` is `(int) strtol(...)`, so out-of-range input saturates to
 *      LONG_MAX/LONG_MIN and is then truncated to `int`.
 *
 *   5. `compareN` passes a possibly-negative `int` as `strncmp`'s `size_t`,
 *      which sign-extends into a huge length while still printing as negative.
 */

use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// Constants from the C source
// ---------------------------------------------------------------------------

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

// user_t { char name[32]; char password[32]; int permission_level; int logged_in; }
const USER_SZ: usize = 72;
const U_NAME: usize = 0;
const U_PASSWORD: usize = 32;
const U_PERMISSION: usize = 64;
const U_LOGGED_IN: usize = 68;

// file_t { char filename[64]; char content[512]; char owner[32]; int permissions; }
const FILE_SZ: usize = 612;
const F_FILENAME: usize = 0;
const F_CONTENT: usize = 64;
const F_OWNER: usize = 576;
const F_PERMISSIONS: usize = 608;

// variable_t { char name[32]; char value[128]; }
const VAR_SZ: usize = 160;
const V_NAME: usize = 0;
const V_VALUE: usize = 32;

/// Extra bytes appended to each emulated global array so that an overflowing
/// `strcpy` past the final element has somewhere (zero-filled) to land instead
/// of panicking.  In C this region is other BSS data; its exact contents are
/// unspecified behaviour either way.
const SLACK: usize = 2048;

// ---------------------------------------------------------------------------
// C library primitives, reimplemented over byte slices
// ---------------------------------------------------------------------------

/// Read the NUL-terminated string starting at `off`, without the terminator.
fn cstr(buf: &[u8], off: usize) -> Vec<u8> {
    if off >= buf.len() {
        return Vec::new();
    }
    let end = match buf[off..].iter().position(|&b| b == 0) {
        Some(p) => off + p,
        None => buf.len(),
    };
    buf[off..end].to_vec()
}

/// `strcpy(&buf[off], src)`: copies `src` plus a NUL terminator.  Writes are
/// clamped to the buffer, which only matters for the out-of-bounds cases that
/// are undefined behaviour in the original.
fn strcpy(buf: &mut [u8], off: usize, src: &[u8]) {
    let len = buf.len();
    for (i, &b) in src.iter().enumerate() {
        if off + i >= len {
            return;
        }
        buf[off + i] = b;
    }
    if off + src.len() < len {
        buf[off + src.len()] = 0;
    }
}

/// `strncpy(dst, src, n - 1)` followed by `dst[n - 1] = '\0'`, where `n` is
/// `dst.len()`.  Note that strncpy zero-pads, so the destination is fully
/// overwritten.
fn strncpy_field(dst: &mut [u8], src: &[u8]) {
    let cap = dst.len() - 1;
    let n = src.len().min(cap);
    dst[..n].copy_from_slice(&src[..n]);
    for b in dst[n..].iter_mut() {
        *b = 0;
    }
}

fn read_i32(buf: &[u8], off: usize) -> i32 {
    let mut bytes = [0u8; 4];
    for i in 0..4 {
        if off + i < buf.len() {
            bytes[i] = buf[off + i];
        }
    }
    i32::from_le_bytes(bytes)
}

fn write_i32(buf: &mut [u8], off: usize, value: i32) {
    let bytes = value.to_le_bytes();
    for i in 0..4 {
        if off + i < buf.len() {
            buf[off + i] = bytes[i];
        }
    }
}

fn byte_at(s: &[u8], i: usize) -> u8 {
    if i < s.len() {
        s[i]
    } else {
        0 // the implicit NUL terminator
    }
}

/// glibc `strcmp`: difference of the first differing bytes as `unsigned char`.
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let ca = byte_at(a, i);
        let cb = byte_at(b, i);
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

/// glibc `strncmp`.  `n` is a `size_t`; the loop always terminates because a
/// NUL is reached in both operands at or before their end.
fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        let ca = byte_at(a, i);
        let cb = byte_at(b, i);
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
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// `atoi` == `(int) strtol(s, NULL, 10)`, including LONG_MAX/LONG_MIN
/// saturation followed by truncation to `int`.
fn c_atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Magnitude limit: LONG_MAX, or |LONG_MIN| when negative.
    let limit: u64 = if negative {
        9_223_372_036_854_775_808
    } else {
        9_223_372_036_854_775_807
    };

    let mut acc: u64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as u64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) if v <= limit => acc = v,
                _ => overflow = true,
            }
        }
        i += 1;
    }

    let value: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };

    value as i32
}

/// `strtok(temp, " \t")` applied repeatedly: split on runs of the delimiters,
/// discarding empty fields.
fn strtok_all(s: &[u8]) -> Vec<&[u8]> {
    s.split(|&b| b == b' ' || b == b'\t')
        .filter(|t| !t.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// stdout: byte-oriented, buffered like a C stdio stream on a pipe
// ---------------------------------------------------------------------------

struct Out {
    w: std::io::BufWriter<std::io::Stdout>,
}

impl Out {
    fn new() -> Self {
        Out {
            w: std::io::BufWriter::with_capacity(1 << 16, std::io::stdout()),
        }
    }
    /// Write raw bytes (printf ignores write errors, and so do we).
    fn b(&mut self, bytes: &[u8]) {
        let _ = self.w.write_all(bytes);
    }
    fn s(&mut self, text: &str) {
        self.b(text.as_bytes());
    }
    fn d(&mut self, value: i32) {
        self.b(value.to_string().as_bytes());
    }
    fn flush(&mut self) {
        let _ = self.w.flush();
    }
}

// ---------------------------------------------------------------------------
// stdin: byte-at-a-time reader backing an exact `fgets`
// ---------------------------------------------------------------------------

struct In {
    src: std::io::Stdin,
    buf: Vec<u8>,
    pos: usize,
    len: usize,
    eof: bool,
}

impl In {
    fn new() -> Self {
        In {
            src: std::io::stdin(),
            buf: vec![0u8; 8192],
            pos: 0,
            len: 0,
            eof: false,
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        if self.pos == self.len {
            if self.eof {
                return None;
            }
            match self.src.read(&mut self.buf) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.pos = 0;
                    self.len = n;
                }
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Some(b)
    }

    /// `fgets(dst, dst.len(), stdin)`: at most `len - 1` bytes, stopping after
    /// a newline, NUL-terminated.  Returns false for the NULL result (EOF with
    /// nothing read).
    fn fgets(&mut self, dst: &mut [u8]) -> bool {
        let cap = dst.len() - 1;
        let mut written = 0usize;
        while written < cap {
            match self.next_byte() {
                Some(b) => {
                    dst[written] = b;
                    written += 1;
                    if b == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }
        if written == 0 {
            return false;
        }
        dst[written] = 0;
        true
    }
}

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

struct State {
    users: Vec<u8>,
    user_count: usize,
    /// Index of the user `current_user` points at; `None` means NULL.
    current_user: Option<usize>,

    files: Vec<u8>,
    file_count: usize,

    variables: Vec<u8>,
    variable_count: usize,

    debug_mode: i32,
    verbose_mode: i32,
}

impl State {
    fn new() -> Self {
        State {
            users: vec![0u8; MAX_USERS * USER_SZ + SLACK],
            user_count: 0,
            current_user: None,
            files: vec![0u8; MAX_FILES * FILE_SZ + SLACK],
            file_count: 0,
            variables: vec![0u8; MAX_VARIABLES * VAR_SZ + SLACK],
            variable_count: 0,
            debug_mode: 0,
            verbose_mode: 0,
        }
    }

    fn user_name(&self, i: usize) -> Vec<u8> {
        cstr(&self.users, i * USER_SZ + U_NAME)
    }
    fn user_password(&self, i: usize) -> Vec<u8> {
        cstr(&self.users, i * USER_SZ + U_PASSWORD)
    }
    fn user_permission(&self, i: usize) -> i32 {
        read_i32(&self.users, i * USER_SZ + U_PERMISSION)
    }
    fn user_logged_in(&self, i: usize) -> i32 {
        read_i32(&self.users, i * USER_SZ + U_LOGGED_IN)
    }

    fn file_name(&self, i: usize) -> Vec<u8> {
        cstr(&self.files, i * FILE_SZ + F_FILENAME)
    }
    fn file_content(&self, i: usize) -> Vec<u8> {
        cstr(&self.files, i * FILE_SZ + F_CONTENT)
    }
    fn file_owner(&self, i: usize) -> Vec<u8> {
        cstr(&self.files, i * FILE_SZ + F_OWNER)
    }
    fn file_permissions(&self, i: usize) -> i32 {
        read_i32(&self.files, i * FILE_SZ + F_PERMISSIONS)
    }

    fn var_name(&self, i: usize) -> Vec<u8> {
        cstr(&self.variables, i * VAR_SZ + V_NAME)
    }
    fn var_value(&self, i: usize) -> Vec<u8> {
        cstr(&self.variables, i * VAR_SZ + V_VALUE)
    }

    /// `current_user && current_user->logged_in`
    fn logged_in_user(&self) -> Option<usize> {
        match self.current_user {
            Some(i) if self.user_logged_in(i) != 0 => Some(i),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Command parsing
// ---------------------------------------------------------------------------

/// Mirrors `process_command`'s stack locals, which persist between calls
/// because the function is always invoked from the same frame in `main`.
struct Locals {
    command: [u8; MAX_COMMAND],
    args: [[u8; MAX_COMMAND]; MAX_ARGS],
    arg_count: usize,
}

impl Locals {
    fn new() -> Self {
        Locals {
            command: [0u8; MAX_COMMAND],
            args: [[0u8; MAX_COMMAND]; MAX_ARGS],
            arg_count: 0,
        }
    }
    fn cmd(&self) -> Vec<u8> {
        cstr(&self.command, 0)
    }
    fn arg(&self, i: usize) -> Vec<u8> {
        cstr(&self.args[i], 0)
    }
}

fn parse_command(input: &[u8], l: &mut Locals) {
    // char temp[MAX_INPUT]; strncpy(temp, input, MAX_INPUT - 1); temp[MAX_INPUT - 1] = '\0';
    let mut temp = [0u8; MAX_INPUT];
    let n = input.len().min(MAX_INPUT - 1);
    temp[..n].copy_from_slice(&input[..n]);
    temp[MAX_INPUT - 1] = 0;

    l.arg_count = 0;
    let temp_str = cstr(&temp, 0);
    let tokens = strtok_all(&temp_str);

    // `if (token)` -- when the line has no tokens, `cmd` is left untouched.
    if let Some(first) = tokens.first() {
        strncpy_field(&mut l.command, first);
        for tok in tokens.iter().skip(1) {
            if l.arg_count >= MAX_ARGS {
                break;
            }
            let idx = l.arg_count;
            strncpy_field(&mut l.args[idx], tok);
            l.arg_count += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// User management commands
// ---------------------------------------------------------------------------

fn cmd_adduser(st: &mut State, l: &Locals, out: &mut Out) {
    if l.arg_count < 2 {
        out.s("Usage: adduser <username> <password> [permission_level]\n");
        return;
    }

    if st.user_count >= MAX_USERS {
        out.s("Error: Maximum users reached\n");
        return;
    }

    let a0 = l.arg(0);
    for i in 0..st.user_count {
        if c_strcmp(&st.user_name(i), &a0) == 0 {
            out.s("Error: User '");
            out.b(&a0);
            out.s("' already exists\n");
            return;
        }
    }

    let base = st.user_count * USER_SZ;
    strcpy(&mut st.users, base + U_NAME, &a0);
    strcpy(&mut st.users, base + U_PASSWORD, &l.arg(1));
    let level = if l.arg_count >= 3 { c_atoi(&l.arg(2)) } else { 1 };
    write_i32(&mut st.users, base + U_PERMISSION, level);
    write_i32(&mut st.users, base + U_LOGGED_IN, 0);
    st.user_count += 1;

    out.s("User '");
    out.b(&a0);
    out.s("' added with permission level ");
    out.d(st.user_permission(st.user_count - 1));
    out.s("\n");
}

fn cmd_login(st: &mut State, l: &Locals, out: &mut Out) {
    if l.arg_count < 2 {
        out.s("Usage: login <username> <password>\n");
        return;
    }

    if let Some(cu) = st.logged_in_user() {
        out.s("Error: User '");
        out.b(&st.user_name(cu));
        out.s("' already logged in. Use 'logout' first.\n");
        return;
    }

    let a0 = l.arg(0);
    let a1 = l.arg(1);
    for i in 0..st.user_count {
        if c_strcmp(&st.user_name(i), &a0) == 0 {
            if c_strcmp(&st.user_password(i), &a1) == 0 {
                write_i32(&mut st.users, i * USER_SZ + U_LOGGED_IN, 1);
                st.current_user = Some(i);
                out.s("Login successful. Welcome, ");
                out.b(&st.user_name(i));
                out.s("!\n");
            } else {
                out.s("Error: Incorrect password\n");
            }
            return;
        }
    }

    out.s("Error: User not found\n");
}

fn cmd_logout(st: &mut State, out: &mut Out) {
    let cu = match st.logged_in_user() {
        Some(i) => i,
        None => {
            out.s("Error: No user logged in\n");
            return;
        }
    };

    out.s("Goodbye, ");
    out.b(&st.user_name(cu));
    out.s("!\n");
    write_i32(&mut st.users, cu * USER_SZ + U_LOGGED_IN, 0);
    st.current_user = None;
}

fn cmd_whoami(st: &State, out: &mut Out) {
    let cu = match st.logged_in_user() {
        Some(i) => i,
        None => {
            out.s("Not logged in\n");
            return;
        }
    };

    out.s("Current user: ");
    out.b(&st.user_name(cu));
    out.s("\n");
    out.s("Permission level: ");
    out.d(st.user_permission(cu));
    out.s("\n");
}

fn cmd_listusers(st: &State, out: &mut Out) {
    if st.user_count == 0 {
        out.s("No users registered\n");
        return;
    }

    out.s("Registered users:\n");
    for i in 0..st.user_count {
        out.s("  ");
        out.b(&st.user_name(i));
        out.s(" (level ");
        out.d(st.user_permission(i));
        out.s(") ");
        out.s(if st.user_logged_in(i) != 0 {
            "[logged in]"
        } else {
            ""
        });
        out.s("\n");
    }
}

// ---------------------------------------------------------------------------
// File management commands
// ---------------------------------------------------------------------------

fn cmd_createfile(st: &mut State, l: &Locals, out: &mut Out) {
    let cu = match st.logged_in_user() {
        Some(i) => i,
        None => {
            out.s("Error: Must be logged in\n");
            return;
        }
    };

    if l.arg_count < 1 {
        out.s("Usage: createfile <filename> [content]\n");
        return;
    }

    if st.file_count >= MAX_FILES {
        out.s("Error: Maximum files reached\n");
        return;
    }

    let a0 = l.arg(0);
    for i in 0..st.file_count {
        if c_strcmp(&st.file_name(i), &a0) == 0 {
            out.s("Error: File '");
            out.b(&a0);
            out.s("' already exists\n");
            return;
        }
    }

    let base = st.file_count * FILE_SZ;
    strcpy(&mut st.files, base + F_FILENAME, &a0);
    let owner = st.user_name(cu);
    strcpy(&mut st.files, base + F_OWNER, &owner);
    write_i32(&mut st.files, base + F_PERMISSIONS, 755);

    if l.arg_count >= 2 {
        strcpy(&mut st.files, base + F_CONTENT, &l.arg(1));
    } else {
        st.files[base + F_CONTENT] = 0;
    }

    st.file_count += 1;
    out.s("File '");
    out.b(&a0);
    out.s("' created\n");
}

fn cmd_readfile(st: &State, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Usage: readfile <filename>\n");
        return;
    }

    let a0 = l.arg(0);
    for i in 0..st.file_count {
        if c_strcmp(&st.file_name(i), &a0) == 0 {
            out.s("=== ");
            out.b(&st.file_name(i));
            out.s(" ===\n");
            out.s("Owner: ");
            out.b(&st.file_owner(i));
            out.s("\n");
            out.s("Permissions: ");
            out.d(st.file_permissions(i));
            out.s("\n");
            out.s("Content: ");
            out.b(&st.file_content(i));
            out.s("\n");
            return;
        }
    }

    out.s("Error: File '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_writefile(st: &mut State, l: &Locals, out: &mut Out) {
    let cu = match st.logged_in_user() {
        Some(i) => i,
        None => {
            out.s("Error: Must be logged in\n");
            return;
        }
    };

    if l.arg_count < 2 {
        out.s("Usage: writefile <filename> <content>\n");
        return;
    }

    let a0 = l.arg(0);
    for i in 0..st.file_count {
        if c_strcmp(&st.file_name(i), &a0) == 0 {
            let cu_name = st.user_name(cu);
            if c_strcmp(&st.file_owner(i), &cu_name) == 0 || st.user_permission(cu) >= 5 {
                strcpy(&mut st.files, i * FILE_SZ + F_CONTENT, &l.arg(1));
                out.s("File '");
                out.b(&a0);
                out.s("' updated\n");
            } else {
                out.s("Error: Permission denied\n");
            }
            return;
        }
    }

    out.s("Error: File '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_deletefile(st: &mut State, l: &Locals, out: &mut Out) {
    let cu = match st.logged_in_user() {
        Some(i) => i,
        None => {
            out.s("Error: Must be logged in\n");
            return;
        }
    };

    if l.arg_count < 1 {
        out.s("Usage: deletefile <filename>\n");
        return;
    }

    let a0 = l.arg(0);
    for i in 0..st.file_count {
        if c_strcmp(&st.file_name(i), &a0) == 0 {
            let cu_name = st.user_name(cu);
            if c_strcmp(&st.file_owner(i), &cu_name) == 0 || st.user_permission(cu) >= 9 {
                for j in i..st.file_count - 1 {
                    st.files
                        .copy_within((j + 1) * FILE_SZ..(j + 2) * FILE_SZ, j * FILE_SZ);
                }
                st.file_count -= 1;
                out.s("File '");
                out.b(&a0);
                out.s("' deleted\n");
            } else {
                out.s("Error: Permission denied\n");
            }
            return;
        }
    }

    out.s("Error: File '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_listfiles(st: &State, out: &mut Out) {
    if st.file_count == 0 {
        out.s("No files\n");
        return;
    }

    out.s("Files:\n");
    for i in 0..st.file_count {
        out.s("  ");
        out.b(&st.file_name(i));
        out.s(" (owner: ");
        out.b(&st.file_owner(i));
        out.s(", perm: ");
        out.d(st.file_permissions(i));
        out.s(")\n");
    }
}

// ---------------------------------------------------------------------------
// Variable commands
// ---------------------------------------------------------------------------

fn cmd_set(st: &mut State, l: &Locals, out: &mut Out) {
    if l.arg_count < 2 {
        out.s("Usage: set <name> <value>\n");
        return;
    }

    let a0 = l.arg(0);
    for i in 0..st.variable_count {
        if c_strcmp(&st.var_name(i), &a0) == 0 {
            strcpy(&mut st.variables, i * VAR_SZ + V_VALUE, &l.arg(1));
            out.s("Variable '");
            out.b(&a0);
            out.s("' updated\n");
            return;
        }
    }

    if st.variable_count >= MAX_VARIABLES {
        out.s("Error: Maximum variables reached\n");
        return;
    }

    let base = st.variable_count * VAR_SZ;
    strcpy(&mut st.variables, base + V_NAME, &a0);
    strcpy(&mut st.variables, base + V_VALUE, &l.arg(1));
    st.variable_count += 1;
    out.s("Variable '");
    out.b(&a0);
    out.s("' set\n");
}

fn cmd_get(st: &State, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Usage: get <name>\n");
        return;
    }

    let a0 = l.arg(0);
    for i in 0..st.variable_count {
        if c_strcmp(&st.var_name(i), &a0) == 0 {
            out.b(&st.var_name(i));
            out.s(" = ");
            out.b(&st.var_value(i));
            out.s("\n");
            return;
        }
    }

    out.s("Error: Variable '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_unset(st: &mut State, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Usage: unset <name>\n");
        return;
    }

    let a0 = l.arg(0);
    for i in 0..st.variable_count {
        if c_strcmp(&st.var_name(i), &a0) == 0 {
            for j in i..st.variable_count - 1 {
                st.variables
                    .copy_within((j + 1) * VAR_SZ..(j + 2) * VAR_SZ, j * VAR_SZ);
            }
            st.variable_count -= 1;
            out.s("Variable '");
            out.b(&a0);
            out.s("' unset\n");
            return;
        }
    }

    out.s("Error: Variable '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_listvars(st: &State, out: &mut Out) {
    if st.variable_count == 0 {
        out.s("No variables set\n");
        return;
    }

    out.s("Variables:\n");
    for i in 0..st.variable_count {
        out.s("  ");
        out.b(&st.var_name(i));
        out.s(" = ");
        out.b(&st.var_value(i));
        out.s("\n");
    }
}

// ---------------------------------------------------------------------------
// String comparison commands
// ---------------------------------------------------------------------------

fn cmd_compare(l: &Locals, out: &mut Out) {
    if l.arg_count < 2 {
        out.s("Usage: compare <string1> <string2>\n");
        return;
    }

    let a0 = l.arg(0);
    let a1 = l.arg(1);
    let result = c_strcmp(&a0, &a1);

    out.s("strcmp('");
    out.b(&a0);
    out.s("', '");
    out.b(&a1);
    out.s("') = ");
    out.d(result);
    out.s("\n");

    if result == 0 {
        out.s("Strings are equal\n");
    } else if result < 0 {
        out.s("'");
        out.b(&a0);
        out.s("' < '");
        out.b(&a1);
        out.s("'\n");
    } else {
        out.s("'");
        out.b(&a0);
        out.s("' > '");
        out.b(&a1);
        out.s("'\n");
    }
}

fn cmd_compare_n(l: &Locals, out: &mut Out) {
    if l.arg_count < 3 {
        out.s("Usage: compareN <string1> <string2> <n>\n");
        return;
    }

    let a0 = l.arg(0);
    let a1 = l.arg(1);
    let n = c_atoi(&l.arg(2));
    // `int` -> `size_t` sign-extends, so a negative n becomes enormous.
    let result = c_strncmp(&a0, &a1, n as isize as usize);

    out.s("strncmp('");
    out.b(&a0);
    out.s("', '");
    out.b(&a1);
    out.s("', ");
    out.d(n);
    out.s(") = ");
    out.d(result);
    out.s("\n");

    if result == 0 {
        out.s("First ");
        out.d(n);
        out.s(" characters are equal\n");
    } else if result < 0 {
        out.s("'");
        out.b(&a0);
        out.s("' < '");
        out.b(&a1);
        out.s("' (first ");
        out.d(n);
        out.s(" chars)\n");
    } else {
        out.s("'");
        out.b(&a0);
        out.s("' > '");
        out.b(&a1);
        out.s("' (first ");
        out.d(n);
        out.s(" chars)\n");
    }
}

fn cmd_startswith(l: &Locals, out: &mut Out) {
    if l.arg_count < 2 {
        out.s("Usage: startswith <string> <prefix>\n");
        return;
    }

    let a0 = l.arg(0);
    let a1 = l.arg(1);
    let prefix_len = a1.len();

    if c_strncmp(&a0, &a1, prefix_len) == 0 {
        out.s("'");
        out.b(&a0);
        out.s("' starts with '");
        out.b(&a1);
        out.s("'\n");
    } else {
        out.s("'");
        out.b(&a0);
        out.s("' does not start with '");
        out.b(&a1);
        out.s("'\n");
    }
}

fn cmd_match(l: &Locals, out: &mut Out) {
    if l.arg_count < 2 {
        out.s("Usage: match <pattern> <string1> [string2] ...\n");
        return;
    }

    let pattern = l.arg(0);
    out.s("Matching pattern '");
    out.b(&pattern);
    out.s("':\n");
    let mut matches = 0i32;

    for i in 1..l.arg_count {
        let ai = l.arg(i);
        if c_strcmp(&pattern, &ai) == 0 {
            out.s("  '");
            out.b(&ai);
            out.s("' - EXACT MATCH\n");
            matches += 1;
        } else if c_strstr(&ai, &pattern) {
            out.s("  '");
            out.b(&ai);
            out.s("' - contains pattern\n");
            matches += 1;
        } else {
            out.s("  '");
            out.b(&ai);
            out.s("' - no match\n");
        }
    }

    out.s("Total matches: ");
    out.d(matches);
    out.s("\n");
}

// ---------------------------------------------------------------------------
// System commands
// ---------------------------------------------------------------------------

fn cmd_help(out: &mut Out) {
    out.s("\n=== Command Interpreter Help ===\n");
    out.s("User Management:\n");
    out.s("  adduser <user> <pass> [level] - Add new user\n");
    out.s("  login <user> <pass>            - Login as user\n");
    out.s("  logout                         - Logout current user\n");
    out.s("  whoami                         - Show current user\n");
    out.s("  listusers                      - List all users\n");
    out.s("\nFile Management:\n");
    out.s("  createfile <name> [content]    - Create file\n");
    out.s("  readfile <name>                - Read file\n");
    out.s("  writefile <name> <content>     - Write to file\n");
    out.s("  deletefile <name>              - Delete file\n");
    out.s("  listfiles                      - List all files\n");
    out.s("\nVariable Management:\n");
    out.s("  set <name> <value>             - Set variable\n");
    out.s("  get <name>                     - Get variable\n");
    out.s("  unset <name>                   - Unset variable\n");
    out.s("  listvars                       - List all variables\n");
    out.s("\nString Operations:\n");
    out.s("  compare <str1> <str2>          - Compare strings\n");
    out.s("  compareN <str1> <str2> <n>     - Compare first N chars\n");
    out.s("  startswith <str> <prefix>      - Check if starts with\n");
    out.s("  match <pattern> <str> ...      - Match pattern\n");
    out.s("\nSystem:\n");
    out.s("  debug [on|off]                 - Toggle debug mode\n");
    out.s("  verbose [on|off]               - Toggle verbose mode\n");
    out.s("  status                         - Show system status\n");
    out.s("  time                           - Show current time\n");
    out.s("  help                           - Show this help\n");
    out.s("  exit                           - Exit program\n");
}

fn cmd_debug(st: &mut State, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Debug mode: ");
        out.s(if st.debug_mode != 0 { "ON" } else { "OFF" });
        out.s("\n");
        return;
    }

    let a0 = l.arg(0);
    if c_strcmp(&a0, b"on") == 0 {
        st.debug_mode = 1;
        out.s("Debug mode enabled\n");
    } else if c_strcmp(&a0, b"off") == 0 {
        st.debug_mode = 0;
        out.s("Debug mode disabled\n");
    } else {
        out.s("Usage: debug [on|off]\n");
    }
}

fn cmd_verbose(st: &mut State, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Verbose mode: ");
        out.s(if st.verbose_mode != 0 { "ON" } else { "OFF" });
        out.s("\n");
        return;
    }

    let a0 = l.arg(0);
    if c_strcmp(&a0, b"on") == 0 {
        st.verbose_mode = 1;
        out.s("Verbose mode enabled\n");
    } else if c_strcmp(&a0, b"off") == 0 {
        st.verbose_mode = 0;
        out.s("Verbose mode disabled\n");
    } else {
        out.s("Usage: verbose [on|off]\n");
    }
}

fn cmd_status(st: &State, out: &mut Out) {
    out.s("\n=== System Status ===\n");
    out.s("Users: ");
    out.d(st.user_count as i32);
    out.s("/");
    out.d(MAX_USERS as i32);
    out.s("\n");
    out.s("Files: ");
    out.d(st.file_count as i32);
    out.s("/");
    out.d(MAX_FILES as i32);
    out.s("\n");
    out.s("Variables: ");
    out.d(st.variable_count as i32);
    out.s("/");
    out.d(MAX_VARIABLES as i32);
    out.s("\n");
    out.s("Current user: ");
    match st.logged_in_user() {
        Some(i) => {
            let name = st.user_name(i);
            out.b(&name);
        }
        None => out.s("none"),
    }
    out.s("\n");
    out.s("Debug mode: ");
    out.s(if st.debug_mode != 0 { "ON" } else { "OFF" });
    out.s("\n");
    out.s("Verbose mode: ");
    out.s(if st.verbose_mode != 0 { "ON" } else { "OFF" });
    out.s("\n");
}

extern "C" {
    fn ctime(timep: *const libc::time_t) -> *const libc::c_char;
}

/// `time(NULL)` + `ctime()`.  Delegated to libc so the locale/timezone
/// formatting matches the C program exactly; `ctime` already ends in '\n',
/// which is why the C printf has no trailing newline of its own.
fn cmd_time(out: &mut Out) {
    let text: Vec<u8> = unsafe {
        let now = libc::time(std::ptr::null_mut());
        let p = ctime(&now);
        if p.is_null() {
            b"(null)".to_vec()
        } else {
            let mut v = Vec::new();
            let mut i = 0isize;
            loop {
                let c = *p.offset(i) as u8;
                if c == 0 {
                    break;
                }
                v.push(c);
                i += 1;
            }
            v
        }
    };
    out.s("Current time: ");
    out.b(&text);
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

fn process_command(input: &[u8], st: &mut State, l: &mut Locals, out: &mut Out) {
    parse_command(input, l);

    let command = l.cmd();
    if command.is_empty() {
        return;
    }

    if st.debug_mode != 0 {
        out.s("[DEBUG] Command: '");
        out.b(&command);
        out.s("', Args: ");
        out.d(l.arg_count as i32);
        out.s("\n");
    }

    let c = |lit: &[u8]| c_strcmp(&command, lit) == 0;
    let cn = |lit: &[u8], n: usize| c_strncmp(&command, lit, n) == 0;

    // User commands
    if c(b"adduser") {
        cmd_adduser(st, l, out);
    } else if c(b"login") {
        cmd_login(st, l, out);
    } else if c(b"logout") {
        cmd_logout(st, out);
    } else if c(b"whoami") {
        cmd_whoami(st, out);
    } else if c(b"listusers") || c(b"users") {
        cmd_listusers(st, out);
    }
    // File commands
    else if c(b"createfile") || c(b"touch") {
        cmd_createfile(st, l, out);
    } else if c(b"readfile") || c(b"cat") {
        cmd_readfile(st, l, out);
    } else if c(b"writefile") || c(b"write") {
        cmd_writefile(st, l, out);
    } else if c(b"deletefile") || c(b"rm") {
        cmd_deletefile(st, l, out);
    } else if c(b"listfiles") || c(b"ls") {
        cmd_listfiles(st, out);
    }
    // Variable commands
    else if c(b"set") {
        cmd_set(st, l, out);
    } else if c(b"get") {
        cmd_get(st, l, out);
    } else if c(b"unset") {
        cmd_unset(st, l, out);
    } else if c(b"listvars") || c(b"vars") {
        cmd_listvars(st, out);
    }
    // String comparison commands
    else if c(b"compare") || c(b"cmp") {
        cmd_compare(l, out);
    } else if c(b"compareN") || c(b"cmpn") {
        cmd_compare_n(l, out);
    } else if c(b"startswith") {
        cmd_startswith(l, out);
    } else if c(b"match") {
        cmd_match(l, out);
    }
    // System commands
    else if c(b"debug") {
        cmd_debug(st, l, out);
    } else if c(b"verbose") {
        cmd_verbose(st, l, out);
    } else if c(b"status") {
        cmd_status(st, out);
    } else if c(b"time") {
        cmd_time(out);
    } else if c(b"help") || c(b"?") {
        cmd_help(out);
    } else if c(b"exit") || c(b"quit") {
        out.s("Goodbye!\n");
        out.flush();
        std::process::exit(0);
    }
    // Partial matches via strncmp
    else if cn(b"add", 3) {
        out.s("Did you mean 'adduser'?\n");
    } else if cn(b"log", 3) {
        out.s("Did you mean 'login' or 'logout'?\n");
    } else if cn(b"list", 4) {
        out.s("Did you mean 'listusers', 'listfiles', or 'listvars'?\n");
    } else if cn(b"create", 6) {
        out.s("Did you mean 'createfile'?\n");
    } else if cn(b"read", 4) {
        out.s("Did you mean 'readfile'?\n");
    } else if cn(b"write", 5) {
        out.s("Did you mean 'writefile'?\n");
    } else if cn(b"delete", 6) {
        out.s("Did you mean 'deletefile'?\n");
    } else {
        out.s("Unknown command: '");
        out.b(&command);
        out.s("'. Type 'help' for available commands.\n");
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let mut out = Out::new();
    let mut inp = In::new();
    let mut st = State::new();
    let mut locals = Locals::new();

    out.s("|----------------------------------------|\n");
    out.s("|   COMMAND INTERPRETER                  |\n");
    out.s("|   strcmp/strncmp demonstration         |\n");
    out.s("|----------------------------------------|\n");
    out.s("Type 'help' for available commands\n\n");

    let mut input = [0u8; MAX_INPUT];

    loop {
        out.s("> ");

        if !inp.fgets(&mut input) {
            break;
        }

        // input[strcspn(input, "\n")] = 0;
        let span = input
            .iter()
            .position(|&b| b == b'\n' || b == 0)
            .unwrap_or(MAX_INPUT - 1);
        input[span] = 0;

        let line = cstr(&input, 0);

        if st.verbose_mode != 0 {
            out.s("[VERBOSE] Processing: '");
            out.b(&line);
            out.s("'\n");
        }

        process_command(&line, &mut st, &mut locals, &mut out);
    }

    out.flush();
}
