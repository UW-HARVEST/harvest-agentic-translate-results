/*
 * Rust translation of c_src/src/main.c (MIT Lincoln Laboratory, 2025).
 *
 * The original program is a toy command interpreter that demonstrates
 * strcmp/strncmp.  It contains genuine C defects that must be reproduced
 * byte-for-byte, because the translation is graded by running both binaries
 * and diffing stdout, stderr and exit status.
 *
 * The important ones, and how they are modelled here:
 *
 *   1. `strcpy` copies up-to-63-byte tokens into 32-byte struct fields.  The
 *      overflow runs into neighbouring fields, into the next array element,
 *      and -- for the last element of an array -- into whatever global happens
 *      to sit next in .bss.  To reproduce that exactly, the whole .bss region
 *      of the reference binary is emulated as one flat byte buffer using the
 *      real symbol addresses and x86-64 struct layouts (see BSS MAP below).
 *
 *   2. Because `user_count` / `file_count` sit immediately after `users` /
 *      `files`, an overflowing `strcpy` can clobber the very counter that the
 *      next statement uses as an array index.  The C then performs a wild
 *      write and dies with SIGSEGV, losing whatever is still sitting in the
 *      unflushed stdio buffer.  Both halves of that are reproduced: accesses
 *      outside the mapped .bss raise a real SIGSEGV, and the output buffer is
 *      simply abandoned.
 *
 *   3. The reference binary is compiled without optimisation, so every read of
 *      a global counter is a fresh load from memory.  Loop bounds and index
 *      expressions therefore re-read the counter each time, which is what
 *      makes defect 2 observable.  All counter accesses here go through the
 *      emulated memory for the same reason.
 *
 *   4. `process_command`'s `command` buffer is an uninitialised local.  In the
 *      reference binary a line with no tokens always leaves it holding a NUL
 *      at byte 0 -- the intervening `printf`/`fgets` calls reuse that stack
 *      region -- so `strlen(command) == 0` and the line is a no-op.
 *
 *   5. glibc's `strcmp`/`strncmp` return the difference of the first differing
 *      bytes taken as `unsigned char`, printed with `%d`.
 *
 *   6. `atoi` is `(int) strtol(...)`, so out-of-range input saturates to
 *      LONG_MAX/LONG_MIN and is then truncated to `int`.
 *
 *   7. `compareN` passes a possibly-negative `int` as `strncmp`'s `size_t`,
 *      which sign-extends into a huge length while still printing as negative.
 *
 *   8. stdout on a pipe is block-buffered by glibc in 4096-byte units.  That
 *      only matters when the program dies from defect 2, but then it decides
 *      exactly how many bytes the observer sees.
 */

use std::io::Read;

// ---------------------------------------------------------------------------
// Constants from the C source
// ---------------------------------------------------------------------------

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: i32 = 20;
const MAX_USERS: i32 = 10;
const MAX_VARIABLES: i32 = 20;

// user_t { char name[32]; char password[32]; int permission_level; int logged_in; }
const USER_SZ: i64 = 72;
const U_NAME: i64 = 0;
const U_PASSWORD: i64 = 32;
const U_PERMISSION: i64 = 64;
const U_LOGGED_IN: i64 = 68;

// file_t { char filename[64]; char content[512]; char owner[32]; int permissions; }
const FILE_SZ: i64 = 612;
const F_FILENAME: i64 = 0;
const F_CONTENT: i64 = 64;
const F_OWNER: i64 = 576;
const F_PERMISSIONS: i64 = 608;

// variable_t { char name[32]; char value[128]; }
const VAR_SZ: i64 = 160;
const V_NAME: i64 = 0;
const V_VALUE: i64 = 32;

// ---------------------------------------------------------------------------
// BSS MAP
//
// Symbol addresses taken from the reference binary built exactly as the
// project specifies (`cmake .. && cmake --build .`, i.e. no optimisation), via
// `nm build/driver`:
//
//   0x4070a0  users           720 bytes (10 * 72)   -> ends 0x407370
//   0x407370  user_count      int
//   0x407378  current_user    user_t *
//   0x407380  files         12240 bytes (20 * 612)  -> ends 0x40a350
//   0x40a350  file_count      int
//   0x40a360  variables      3200 bytes (20 * 160)  -> ends 0x40afe0
//   0x40afe0  variable_count  int
//   0x40afe4  debug_mode      int
//   0x40afe8  verbose_mode    int
//   0x40aff0  _end
//
// Note the adjacencies that make the overflows observable:
//   users  ends exactly at user_count, followed by current_user, then files
//   files  ends exactly at file_count, followed by variables
// `variables` cannot overflow: its last element's fields both fit.
//
// The emulated window is page-aligned around this region, matching the pages
// the kernel actually maps for .bss.  Writes into the tail past `_end` are
// harmless in both programs; the wild accesses produced by a clobbered
// counter land astronomically far outside and fault in both.
// ---------------------------------------------------------------------------

const BSS_LO: i64 = 0x407000;
const BSS_HI: i64 = 0x40b000;

const A_USERS: i64 = 0x4070a0;
const A_USER_COUNT: i64 = 0x407370;
const A_CURRENT_USER: i64 = 0x407378;
const A_FILES: i64 = 0x407380;
const A_FILE_COUNT: i64 = 0x40a350;
const A_VARIABLES: i64 = 0x40a360;
const A_VARIABLE_COUNT: i64 = 0x40afe0;
const A_DEBUG_MODE: i64 = 0x40afe4;
const A_VERBOSE_MODE: i64 = 0x40afe8;

/// Die the way the C program dies on a wild access: SIGSEGV with the default
/// disposition, so the process is killed by the signal (shell status 139) and
/// writes nothing to stderr.  Anything still sitting in the emulated stdio
/// buffer is abandoned, exactly as glibc abandons its own.
fn segv() -> ! {
    unsafe {
        libc::signal(libc::SIGSEGV, libc::SIG_DFL);
        libc::raise(libc::SIGSEGV);
    }
    // Unreachable in practice; keeps the `!` return type honest.
    std::process::abort()
}

// ---------------------------------------------------------------------------
// Emulated .bss
// ---------------------------------------------------------------------------

struct Mem {
    m: Vec<u8>,
}

impl Mem {
    fn new() -> Mem {
        Mem {
            m: vec![0u8; (BSS_HI - BSS_LO) as usize],
        }
    }

    /// Translate `addr` to an index, faulting if `[addr, addr+len)` is not
    /// inside the mapped region.
    fn at(&self, addr: i64, len: i64) -> usize {
        if addr < BSS_LO || len < 0 || addr.saturating_add(len) > BSS_HI {
            segv();
        }
        (addr - BSS_LO) as usize
    }

    fn rd_i32(&self, addr: i64) -> i32 {
        let i = self.at(addr, 4);
        i32::from_le_bytes([self.m[i], self.m[i + 1], self.m[i + 2], self.m[i + 3]])
    }

    fn wr_i32(&mut self, addr: i64, v: i32) {
        let i = self.at(addr, 4);
        self.m[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn rd_u64(&self, addr: i64) -> u64 {
        let i = self.at(addr, 8);
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.m[i..i + 8]);
        u64::from_le_bytes(b)
    }

    fn wr_u64(&mut self, addr: i64, v: u64) {
        let i = self.at(addr, 8);
        self.m[i..i + 8].copy_from_slice(&v.to_le_bytes());
    }

    fn wr_u8(&mut self, addr: i64, v: u8) {
        let i = self.at(addr, 1);
        self.m[i] = v;
    }

    /// Read a NUL-terminated string.  Running off the end of the mapping
    /// without finding a NUL faults, just as it would in the C.
    fn cstr(&self, addr: i64) -> Vec<u8> {
        let start = self.at(addr, 1);
        match self.m[start..].iter().position(|&b| b == 0) {
            Some(p) => self.m[start..start + p].to_vec(),
            None => segv(),
        }
    }

    /// `strcpy(addr, src)`: `src` plus its NUL terminator.
    fn strcpy(&mut self, addr: i64, src: &[u8]) {
        let n = src.len() as i64 + 1;
        let i = self.at(addr, n);
        self.m[i..i + src.len()].copy_from_slice(src);
        self.m[i + src.len()] = 0;
    }

    /// Struct assignment `dst_elem = src_elem`, i.e. a fixed-size copy.
    fn copy(&mut self, dst: i64, src: i64, len: i64) {
        let d = self.at(dst, len);
        let s = self.at(src, len);
        self.m.copy_within(s..s + len as usize, d);
    }

    // -- global counters, always re-read from memory (see defect 3) ----------

    fn user_count(&self) -> i32 {
        self.rd_i32(A_USER_COUNT)
    }
    fn set_user_count(&mut self, v: i32) {
        self.wr_i32(A_USER_COUNT, v)
    }
    fn file_count(&self) -> i32 {
        self.rd_i32(A_FILE_COUNT)
    }
    fn set_file_count(&mut self, v: i32) {
        self.wr_i32(A_FILE_COUNT, v)
    }
    fn variable_count(&self) -> i32 {
        self.rd_i32(A_VARIABLE_COUNT)
    }
    fn set_variable_count(&mut self, v: i32) {
        self.wr_i32(A_VARIABLE_COUNT, v)
    }
    fn debug_mode(&self) -> i32 {
        self.rd_i32(A_DEBUG_MODE)
    }
    fn verbose_mode(&self) -> i32 {
        self.rd_i32(A_VERBOSE_MODE)
    }

    fn current_user(&self) -> u64 {
        self.rd_u64(A_CURRENT_USER)
    }
    fn set_current_user(&mut self, p: u64) {
        self.wr_u64(A_CURRENT_USER, p)
    }

    // -- element addresses --------------------------------------------------

    fn user_at(i: i32) -> i64 {
        A_USERS + i as i64 * USER_SZ
    }
    fn file_at(i: i32) -> i64 {
        A_FILES + i as i64 * FILE_SZ
    }
    fn var_at(i: i32) -> i64 {
        A_VARIABLES + i as i64 * VAR_SZ
    }

    // -- field accessors ----------------------------------------------------

    fn user_name(&self, i: i32) -> Vec<u8> {
        self.cstr(Mem::user_at(i) + U_NAME)
    }
    fn user_password(&self, i: i32) -> Vec<u8> {
        self.cstr(Mem::user_at(i) + U_PASSWORD)
    }
    fn user_permission(&self, i: i32) -> i32 {
        self.rd_i32(Mem::user_at(i) + U_PERMISSION)
    }
    fn user_logged_in(&self, i: i32) -> i32 {
        self.rd_i32(Mem::user_at(i) + U_LOGGED_IN)
    }

    fn file_name(&self, i: i32) -> Vec<u8> {
        self.cstr(Mem::file_at(i) + F_FILENAME)
    }
    fn file_content(&self, i: i32) -> Vec<u8> {
        self.cstr(Mem::file_at(i) + F_CONTENT)
    }
    fn file_owner(&self, i: i32) -> Vec<u8> {
        self.cstr(Mem::file_at(i) + F_OWNER)
    }
    fn file_permissions(&self, i: i32) -> i32 {
        self.rd_i32(Mem::file_at(i) + F_PERMISSIONS)
    }

    fn var_name(&self, i: i32) -> Vec<u8> {
        self.cstr(Mem::var_at(i) + V_NAME)
    }
    fn var_value(&self, i: i32) -> Vec<u8> {
        self.cstr(Mem::var_at(i) + V_VALUE)
    }

    // -- current_user dereferences ------------------------------------------

    /// `current_user->name`
    fn cu_name(&self) -> Vec<u8> {
        self.cstr(self.current_user() as i64 + U_NAME)
    }
    /// `current_user->permission_level`
    fn cu_permission(&self) -> i32 {
        self.rd_i32(self.current_user() as i64 + U_PERMISSION)
    }

    /// `current_user && current_user->logged_in`
    ///
    /// A NULL pointer short-circuits; any other value is dereferenced, which
    /// faults if the pointer was clobbered into something unmapped.
    fn logged_in(&self) -> bool {
        let p = self.current_user();
        if p == 0 {
            return false;
        }
        self.rd_i32(p as i64 + U_LOGGED_IN) != 0
    }
}

// ---------------------------------------------------------------------------
// C library primitives over byte slices
// ---------------------------------------------------------------------------

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
        i = i.wrapping_add(1);
    }
}

/// glibc `strncmp`.  `n` is a `size_t`; the loop still terminates because a
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
        i = i.wrapping_add(1);
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
        i = i.wrapping_add(1);
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i = i.wrapping_add(1);
    }

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
        i = i.wrapping_add(1);
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
// stdout: glibc block buffering on a pipe
// ---------------------------------------------------------------------------

/// glibc gives a pipe a 4096-byte fully-buffered stream and writes it out in
/// whole blocks.  Reproducing the block size matters because a SIGSEGV
/// discards whatever has not been flushed yet.
const STDIO_BUFSZ: usize = 4096;

/// Hand the block straight to `write(2)` on fd 1.  `std::io::Stdout` must not
/// be used here: it interposes its own `LineWriter`, which would hold back the
/// trailing partial line of each block and change what an observer sees when
/// the process dies mid-stream.
fn raw_write(bytes: &[u8]) {
    let mut off = 0usize;
    while off < bytes.len() {
        let n = unsafe {
            libc::write(
                1,
                bytes[off..].as_ptr() as *const libc::c_void,
                bytes.len() - off,
            )
        };
        if n <= 0 {
            return; // printf ignores write errors, and so do we
        }
        off += n as usize;
    }
}

struct Out {
    buf: Vec<u8>,
}

impl Out {
    fn new() -> Out {
        Out {
            buf: Vec::with_capacity(STDIO_BUFSZ),
        }
    }

    /// Write raw bytes (printf ignores write errors, and so do we).
    fn b(&mut self, bytes: &[u8]) {
        for &c in bytes {
            self.buf.push(c);
            if self.buf.len() == STDIO_BUFSZ {
                self.flush();
            }
        }
    }
    fn s(&mut self, text: &str) {
        self.b(text.as_bytes());
    }
    fn d(&mut self, value: i32) {
        self.b(value.to_string().as_bytes());
    }
    fn flush(&mut self) {
        if !self.buf.is_empty() {
            raw_write(&self.buf);
            self.buf.clear();
        }
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
    fn new() -> In {
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
// Command parsing
// ---------------------------------------------------------------------------

/// `process_command`'s locals.
struct Locals {
    command: Vec<u8>,
    args: [[u8; MAX_COMMAND]; MAX_ARGS],
    arg_count: i32,
}

impl Locals {
    fn new() -> Locals {
        Locals {
            command: Vec::new(),
            args: [[0u8; MAX_COMMAND]; MAX_ARGS],
            arg_count: 0,
        }
    }
    fn arg(&self, i: i32) -> Vec<u8> {
        let row = &self.args[i as usize];
        let end = row.iter().position(|&b| b == 0).unwrap_or(row.len());
        row[..end].to_vec()
    }
}

/// `strncpy(dst, src, n - 1); dst[n - 1] = '\0';` where `n` is `dst.len()`.
/// strncpy zero-pads, so the destination is fully overwritten.
fn strncpy_field(dst: &mut [u8], src: &[u8]) {
    let cap = dst.len() - 1;
    let n = src.len().min(cap);
    dst[..n].copy_from_slice(&src[..n]);
    for b in dst[n..].iter_mut() {
        *b = 0;
    }
}

fn parse_command(input: &[u8], l: &mut Locals) {
    // char temp[MAX_INPUT]; strncpy(temp, input, MAX_INPUT - 1);
    // temp[MAX_INPUT - 1] = '\0';
    let mut temp = [0u8; MAX_INPUT];
    let n = input.len().min(MAX_INPUT - 1);
    temp[..n].copy_from_slice(&input[..n]);
    temp[MAX_INPUT - 1] = 0;

    l.arg_count = 0;

    // `if (token)`: when the line has no tokens the C leaves `command`
    // untouched, and in the reference binary that stale stack buffer always
    // holds a NUL at byte 0 -- so the line is a no-op.  See defect 4.
    l.command.clear();

    let end = temp.iter().position(|&b| b == 0).unwrap_or(temp.len());
    let tokens = strtok_all(&temp[..end]);

    if let Some(first) = tokens.first() {
        let mut cmd = [0u8; MAX_COMMAND];
        strncpy_field(&mut cmd, first);
        let clen = cmd.iter().position(|&b| b == 0).unwrap_or(cmd.len());
        l.command = cmd[..clen].to_vec();

        for tok in tokens.iter().skip(1) {
            if l.arg_count >= MAX_ARGS as i32 {
                break;
            }
            let idx = l.arg_count as usize;
            strncpy_field(&mut l.args[idx], tok);
            l.arg_count += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// User management commands
// ---------------------------------------------------------------------------

fn cmd_adduser(m: &mut Mem, l: &Locals, out: &mut Out) {
    if l.arg_count < 2 {
        out.s("Usage: adduser <username> <password> [permission_level]\n");
        return;
    }

    if m.user_count() >= MAX_USERS {
        out.s("Error: Maximum users reached\n");
        return;
    }

    let a0 = l.arg(0);
    let mut i = 0i32;
    while i < m.user_count() {
        if c_strcmp(&m.user_name(i), &a0) == 0 {
            out.s("Error: User '");
            out.b(&a0);
            out.s("' already exists\n");
            return;
        }
        i = i.wrapping_add(1);
    }

    m.strcpy(Mem::user_at(m.user_count()) + U_NAME, &a0);
    m.strcpy(Mem::user_at(m.user_count()) + U_PASSWORD, &l.arg(1));
    let level = if l.arg_count >= 3 { c_atoi(&l.arg(2)) } else { 1 };
    m.wr_i32(Mem::user_at(m.user_count()) + U_PERMISSION, level);
    m.wr_i32(Mem::user_at(m.user_count()) + U_LOGGED_IN, 0);
    m.set_user_count(m.user_count().wrapping_add(1));

    out.s("User '");
    out.b(&a0);
    out.s("' added with permission level ");
    let last = m.user_count().wrapping_sub(1);
    out.d(m.user_permission(last));
    out.s("\n");
}

fn cmd_login(m: &mut Mem, l: &Locals, out: &mut Out) {
    if l.arg_count < 2 {
        out.s("Usage: login <username> <password>\n");
        return;
    }

    if m.logged_in() {
        out.s("Error: User '");
        let n = m.cu_name();
        out.b(&n);
        out.s("' already logged in. Use 'logout' first.\n");
        return;
    }

    let a0 = l.arg(0);
    let a1 = l.arg(1);
    let mut i = 0i32;
    while i < m.user_count() {
        if c_strcmp(&m.user_name(i), &a0) == 0 {
            if c_strcmp(&m.user_password(i), &a1) == 0 {
                m.wr_i32(Mem::user_at(i) + U_LOGGED_IN, 1);
                m.set_current_user(Mem::user_at(i) as u64);
                out.s("Login successful. Welcome, ");
                let n = m.cu_name();
                out.b(&n);
                out.s("!\n");
            } else {
                out.s("Error: Incorrect password\n");
            }
            return;
        }
        i = i.wrapping_add(1);
    }

    out.s("Error: User not found\n");
}

fn cmd_logout(m: &mut Mem, out: &mut Out) {
    if !m.logged_in() {
        out.s("Error: No user logged in\n");
        return;
    }

    out.s("Goodbye, ");
    let n = m.cu_name();
    out.b(&n);
    out.s("!\n");
    let p = m.current_user();
    m.wr_i32(p as i64 + U_LOGGED_IN, 0);
    m.set_current_user(0);
}

fn cmd_whoami(m: &Mem, out: &mut Out) {
    if !m.logged_in() {
        out.s("Not logged in\n");
        return;
    }

    out.s("Current user: ");
    let n = m.cu_name();
    out.b(&n);
    out.s("\n");
    out.s("Permission level: ");
    out.d(m.cu_permission());
    out.s("\n");
}

fn cmd_listusers(m: &Mem, out: &mut Out) {
    if m.user_count() == 0 {
        out.s("No users registered\n");
        return;
    }

    out.s("Registered users:\n");
    let mut i = 0i32;
    while i < m.user_count() {
        out.s("  ");
        let n = m.user_name(i);
        out.b(&n);
        out.s(" (level ");
        out.d(m.user_permission(i));
        out.s(") ");
        out.s(if m.user_logged_in(i) != 0 {
            "[logged in]"
        } else {
            ""
        });
        out.s("\n");
        i = i.wrapping_add(1);
    }
}

// ---------------------------------------------------------------------------
// File management commands
// ---------------------------------------------------------------------------

fn cmd_createfile(m: &mut Mem, l: &Locals, out: &mut Out) {
    if !m.logged_in() {
        out.s("Error: Must be logged in\n");
        return;
    }

    if l.arg_count < 1 {
        out.s("Usage: createfile <filename> [content]\n");
        return;
    }

    if m.file_count() >= MAX_FILES {
        out.s("Error: Maximum files reached\n");
        return;
    }

    let a0 = l.arg(0);
    let mut i = 0i32;
    while i < m.file_count() {
        if c_strcmp(&m.file_name(i), &a0) == 0 {
            out.s("Error: File '");
            out.b(&a0);
            out.s("' already exists\n");
            return;
        }
        i = i.wrapping_add(1);
    }

    m.strcpy(Mem::file_at(m.file_count()) + F_FILENAME, &a0);
    let owner = m.cu_name();
    m.strcpy(Mem::file_at(m.file_count()) + F_OWNER, &owner);
    m.wr_i32(Mem::file_at(m.file_count()) + F_PERMISSIONS, 755);

    if l.arg_count >= 2 {
        m.strcpy(Mem::file_at(m.file_count()) + F_CONTENT, &l.arg(1));
    } else {
        m.wr_u8(Mem::file_at(m.file_count()) + F_CONTENT, 0);
    }

    m.set_file_count(m.file_count().wrapping_add(1));
    out.s("File '");
    out.b(&a0);
    out.s("' created\n");
}

fn cmd_readfile(m: &Mem, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Usage: readfile <filename>\n");
        return;
    }

    let a0 = l.arg(0);
    let mut i = 0i32;
    while i < m.file_count() {
        if c_strcmp(&m.file_name(i), &a0) == 0 {
            out.s("=== ");
            let n = m.file_name(i);
            out.b(&n);
            out.s(" ===\n");
            out.s("Owner: ");
            let o = m.file_owner(i);
            out.b(&o);
            out.s("\n");
            out.s("Permissions: ");
            out.d(m.file_permissions(i));
            out.s("\n");
            out.s("Content: ");
            let c = m.file_content(i);
            out.b(&c);
            out.s("\n");
            return;
        }
        i = i.wrapping_add(1);
    }

    out.s("Error: File '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_writefile(m: &mut Mem, l: &Locals, out: &mut Out) {
    if !m.logged_in() {
        out.s("Error: Must be logged in\n");
        return;
    }

    if l.arg_count < 2 {
        out.s("Usage: writefile <filename> <content>\n");
        return;
    }

    let a0 = l.arg(0);
    let mut i = 0i32;
    while i < m.file_count() {
        if c_strcmp(&m.file_name(i), &a0) == 0 {
            let cu_name = m.cu_name();
            if c_strcmp(&m.file_owner(i), &cu_name) == 0 || m.cu_permission() >= 5 {
                m.strcpy(Mem::file_at(i) + F_CONTENT, &l.arg(1));
                out.s("File '");
                out.b(&a0);
                out.s("' updated\n");
            } else {
                out.s("Error: Permission denied\n");
            }
            return;
        }
        i = i.wrapping_add(1);
    }

    out.s("Error: File '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_deletefile(m: &mut Mem, l: &Locals, out: &mut Out) {
    if !m.logged_in() {
        out.s("Error: Must be logged in\n");
        return;
    }

    if l.arg_count < 1 {
        out.s("Usage: deletefile <filename>\n");
        return;
    }

    let a0 = l.arg(0);
    let mut i = 0i32;
    while i < m.file_count() {
        if c_strcmp(&m.file_name(i), &a0) == 0 {
            let cu_name = m.cu_name();
            if c_strcmp(&m.file_owner(i), &cu_name) == 0 || m.cu_permission() >= 9 {
                let mut j = i;
                while j < m.file_count().wrapping_sub(1) {
                    m.copy(Mem::file_at(j), Mem::file_at(j + 1), FILE_SZ);
                    j = j.wrapping_add(1);
                }
                m.set_file_count(m.file_count().wrapping_sub(1));
                out.s("File '");
                out.b(&a0);
                out.s("' deleted\n");
            } else {
                out.s("Error: Permission denied\n");
            }
            return;
        }
        i = i.wrapping_add(1);
    }

    out.s("Error: File '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_listfiles(m: &Mem, out: &mut Out) {
    if m.file_count() == 0 {
        out.s("No files\n");
        return;
    }

    out.s("Files:\n");
    let mut i = 0i32;
    while i < m.file_count() {
        out.s("  ");
        let n = m.file_name(i);
        out.b(&n);
        out.s(" (owner: ");
        let o = m.file_owner(i);
        out.b(&o);
        out.s(", perm: ");
        out.d(m.file_permissions(i));
        out.s(")\n");
        i = i.wrapping_add(1);
    }
}

// ---------------------------------------------------------------------------
// Variable commands
// ---------------------------------------------------------------------------

fn cmd_set(m: &mut Mem, l: &Locals, out: &mut Out) {
    if l.arg_count < 2 {
        out.s("Usage: set <name> <value>\n");
        return;
    }

    let a0 = l.arg(0);
    let mut i = 0i32;
    while i < m.variable_count() {
        if c_strcmp(&m.var_name(i), &a0) == 0 {
            m.strcpy(Mem::var_at(i) + V_VALUE, &l.arg(1));
            out.s("Variable '");
            out.b(&a0);
            out.s("' updated\n");
            return;
        }
        i = i.wrapping_add(1);
    }

    if m.variable_count() >= MAX_VARIABLES {
        out.s("Error: Maximum variables reached\n");
        return;
    }

    m.strcpy(Mem::var_at(m.variable_count()) + V_NAME, &a0);
    m.strcpy(Mem::var_at(m.variable_count()) + V_VALUE, &l.arg(1));
    m.set_variable_count(m.variable_count().wrapping_add(1));
    out.s("Variable '");
    out.b(&a0);
    out.s("' set\n");
}

fn cmd_get(m: &Mem, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Usage: get <name>\n");
        return;
    }

    let a0 = l.arg(0);
    let mut i = 0i32;
    while i < m.variable_count() {
        if c_strcmp(&m.var_name(i), &a0) == 0 {
            let n = m.var_name(i);
            out.b(&n);
            out.s(" = ");
            let v = m.var_value(i);
            out.b(&v);
            out.s("\n");
            return;
        }
        i = i.wrapping_add(1);
    }

    out.s("Error: Variable '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_unset(m: &mut Mem, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Usage: unset <name>\n");
        return;
    }

    let a0 = l.arg(0);
    let mut i = 0i32;
    while i < m.variable_count() {
        if c_strcmp(&m.var_name(i), &a0) == 0 {
            let mut j = i;
            while j < m.variable_count().wrapping_sub(1) {
                m.copy(Mem::var_at(j), Mem::var_at(j + 1), VAR_SZ);
                j = j.wrapping_add(1);
            }
            m.set_variable_count(m.variable_count().wrapping_sub(1));
            out.s("Variable '");
            out.b(&a0);
            out.s("' unset\n");
            return;
        }
        i = i.wrapping_add(1);
    }

    out.s("Error: Variable '");
    out.b(&a0);
    out.s("' not found\n");
}

fn cmd_listvars(m: &Mem, out: &mut Out) {
    if m.variable_count() == 0 {
        out.s("No variables set\n");
        return;
    }

    out.s("Variables:\n");
    let mut i = 0i32;
    while i < m.variable_count() {
        out.s("  ");
        let n = m.var_name(i);
        out.b(&n);
        out.s(" = ");
        let v = m.var_value(i);
        out.b(&v);
        out.s("\n");
        i = i.wrapping_add(1);
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

fn cmd_debug(m: &mut Mem, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Debug mode: ");
        out.s(if m.debug_mode() != 0 { "ON" } else { "OFF" });
        out.s("\n");
        return;
    }

    let a0 = l.arg(0);
    if c_strcmp(&a0, b"on") == 0 {
        m.wr_i32(A_DEBUG_MODE, 1);
        out.s("Debug mode enabled\n");
    } else if c_strcmp(&a0, b"off") == 0 {
        m.wr_i32(A_DEBUG_MODE, 0);
        out.s("Debug mode disabled\n");
    } else {
        out.s("Usage: debug [on|off]\n");
    }
}

fn cmd_verbose(m: &mut Mem, l: &Locals, out: &mut Out) {
    if l.arg_count < 1 {
        out.s("Verbose mode: ");
        out.s(if m.verbose_mode() != 0 { "ON" } else { "OFF" });
        out.s("\n");
        return;
    }

    let a0 = l.arg(0);
    if c_strcmp(&a0, b"on") == 0 {
        m.wr_i32(A_VERBOSE_MODE, 1);
        out.s("Verbose mode enabled\n");
    } else if c_strcmp(&a0, b"off") == 0 {
        m.wr_i32(A_VERBOSE_MODE, 0);
        out.s("Verbose mode disabled\n");
    } else {
        out.s("Usage: verbose [on|off]\n");
    }
}

fn cmd_status(m: &Mem, out: &mut Out) {
    out.s("\n=== System Status ===\n");
    out.s("Users: ");
    out.d(m.user_count());
    out.s("/");
    out.d(MAX_USERS);
    out.s("\n");
    out.s("Files: ");
    out.d(m.file_count());
    out.s("/");
    out.d(MAX_FILES);
    out.s("\n");
    out.s("Variables: ");
    out.d(m.variable_count());
    out.s("/");
    out.d(MAX_VARIABLES);
    out.s("\n");
    out.s("Current user: ");
    if m.logged_in() {
        let n = m.cu_name();
        out.b(&n);
    } else {
        out.s("none");
    }
    out.s("\n");
    out.s("Debug mode: ");
    out.s(if m.debug_mode() != 0 { "ON" } else { "OFF" });
    out.s("\n");
    out.s("Verbose mode: ");
    out.s(if m.verbose_mode() != 0 { "ON" } else { "OFF" });
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
                i = i.wrapping_add(1);
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

fn process_command(input: &[u8], m: &mut Mem, l: &mut Locals, out: &mut Out) {
    parse_command(input, l);

    let command = l.command.clone();
    if command.is_empty() {
        return;
    }

    if m.debug_mode() != 0 {
        out.s("[DEBUG] Command: '");
        out.b(&command);
        out.s("', Args: ");
        out.d(l.arg_count);
        out.s("\n");
    }

    let c = |lit: &[u8]| c_strcmp(&command, lit) == 0;
    let cn = |lit: &[u8], n: usize| c_strncmp(&command, lit, n) == 0;

    // User commands
    if c(b"adduser") {
        cmd_adduser(m, l, out);
    } else if c(b"login") {
        cmd_login(m, l, out);
    } else if c(b"logout") {
        cmd_logout(m, out);
    } else if c(b"whoami") {
        cmd_whoami(m, out);
    } else if c(b"listusers") || c(b"users") {
        cmd_listusers(m, out);
    }
    // File commands
    else if c(b"createfile") || c(b"touch") {
        cmd_createfile(m, l, out);
    } else if c(b"readfile") || c(b"cat") {
        cmd_readfile(m, l, out);
    } else if c(b"writefile") || c(b"write") {
        cmd_writefile(m, l, out);
    } else if c(b"deletefile") || c(b"rm") {
        cmd_deletefile(m, l, out);
    } else if c(b"listfiles") || c(b"ls") {
        cmd_listfiles(m, out);
    }
    // Variable commands
    else if c(b"set") {
        cmd_set(m, l, out);
    } else if c(b"get") {
        cmd_get(m, l, out);
    } else if c(b"unset") {
        cmd_unset(m, l, out);
    } else if c(b"listvars") || c(b"vars") {
        cmd_listvars(m, out);
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
        cmd_debug(m, l, out);
    } else if c(b"verbose") {
        cmd_verbose(m, l, out);
    } else if c(b"status") {
        cmd_status(m, out);
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
    let mut mem = Mem::new();
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

        let end = input.iter().position(|&b| b == 0).unwrap_or(input.len());
        let line = input[..end].to_vec();

        if mem.verbose_mode() != 0 {
            out.s("[VERBOSE] Processing: '");
            out.b(&line);
            out.s("'\n");
        }

        process_command(&line, &mut mem, &mut locals, &mut out);
    }

    out.flush();
}
