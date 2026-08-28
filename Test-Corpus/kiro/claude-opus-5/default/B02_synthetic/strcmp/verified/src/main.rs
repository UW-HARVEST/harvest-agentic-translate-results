// Rust translation of c_src/src/main.c
//
// The original C program is a toy command interpreter that demonstrates
// strcmp/strncmp. This translation reproduces its observable behaviour
// byte-for-byte, including the parts that are undefined behaviour in C.
//
// Faithfulness notes
// ------------------
// * `fgets` reading: 255 byte chunks, so a long line is split and each chunk
//   gets its own "> " prompt.
// * `strtok` tokenisation on ' ' and '\t' only, empty tokens skipped.
// * glibc `strcmp`/`strncmp` return the difference of the first differing
//   bytes taken as `unsigned char`, not a normalised -1/0/1.
// * C `atoi` == `(int) strtol(s, NULL, 10)`, i.e. saturating in `long` then
//   truncating to `int`.
// * The C code `strcpy`s tokens of up to 63 characters into `char[32]` struct
//   fields. That overflow is not an accident we may fix: it is observable.
//   All of the program's mutable state lives in consecutive BSS objects, so an
//   overflow of `users[9].password` walks into `user_count` and then
//   `current_user`, an overflow of `files[19].owner` walks into `file_count`,
//   and so on. The whole BSS window is therefore modelled as one flat byte
//   array using the exact offsets of the compiled C program, and every access
//   is bounds checked against the writable page range. An access outside it
//   raises SIGSEGV without flushing buffered stdout, exactly as the C program
//   dies.
// * stdout is emulated as glibc's fully buffered 4096 byte stream (the
//   `st_blksize` of a pipe or a regular file) so that the amount of output
//   that survives a crash matches too.

use std::io::Write;
use std::mem::ManuallyDrop;
use std::os::unix::io::FromRawFd;

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: i32 = 20;
const MAX_USERS: i32 = 10;
const MAX_VARIABLES: i32 = 20;

// ---------------------------------------------------------------------------
// libc bindings
// ---------------------------------------------------------------------------

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn raise(sig: i32) -> i32;
    fn time(tloc: *mut i64) -> i64;
    fn ctime(timep: *const i64) -> *const u8;
}

const SIGSEGV: i32 = 11;
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

/// Die the way the C program dies when it dereferences an address outside its
/// writable mapping: killed by SIGSEGV, with no Rust destructors run and, in
/// particular, no flush of the buffered stdout stream.
fn segv() -> ! {
    unsafe {
        signal(SIGSEGV, SIG_DFL);
        raise(SIGSEGV);
    }
    std::process::abort()
}

// ---------------------------------------------------------------------------
// BSS layout of the compiled C program (x86-64, non-PIE), from
// `nm -td c_src/build/driver`:
//
//   users            0x4070a0  size   720   rel      0
//   user_count       0x407370  size     4   rel    720
//   current_user     0x407378  size     8   rel    728
//   files            0x407380  size 12240   rel    736
//   file_count       0x40a350  size     4   rel  12976
//   variables        0x40a360  size  3200   rel  12992
//   variable_count   0x40afe0  size     4   rel  16192
//   debug_mode       0x40afe4  size     4   rel  16196
//   verbose_mode     0x40afe8  size     4   rel  16200
//
// The RW LOAD segment is 0x406de8 + 0x4208 (readelf -lW), so the mapped
// writable pages are [0x406000, 0x40b000). Relative to `users` that is
// [-4256, 16224).
// ---------------------------------------------------------------------------

const BASE_ADDR: i64 = 0x0040_70a0;
const MEM_LO: i64 = 0x0040_6000 - BASE_ADDR; // -4256
const MEM_HI: i64 = 0x0040_b000 - BASE_ADDR; // 16224

const G_USERS: i64 = 0;
const G_USER_COUNT: i64 = 720;
const G_CURRENT_USER: i64 = 728;
const G_FILES: i64 = 736;
const G_FILE_COUNT: i64 = 12976;
const G_VARIABLES: i64 = 12992;
const G_VARIABLE_COUNT: i64 = 16192;
const G_DEBUG_MODE: i64 = 16196;
const G_VERBOSE_MODE: i64 = 16200;

// typedef struct { char name[32]; char password[32]; int permission_level; int logged_in; } user_t;
const U_NAME: i64 = 0;
const U_PASS: i64 = 32;
const U_PERM: i64 = 64;
const U_LOGGED: i64 = 68;
const U_SZ: i64 = 72;

// typedef struct { char filename[64]; char content[512]; char owner[32]; int permissions; } file_t;
const F_NAME: i64 = 0;
const F_CONTENT: i64 = 64;
const F_OWNER: i64 = 576;
const F_PERM: i64 = 608;
const F_SZ: i64 = 612;

// typedef struct { char name[32]; char value[128]; } variable_t;
const V_NAME: i64 = 0;
const V_VALUE: i64 = 32;
const V_SZ: i64 = 160;

/// The program's writable global memory as one flat array.
struct Mem {
    buf: Vec<u8>,
}

impl Mem {
    fn new() -> Mem {
        Mem {
            buf: vec![0u8; (MEM_HI - MEM_LO) as usize],
        }
    }

    /// Translate a byte offset relative to `&users` into an index, faulting if
    /// the `len` byte access leaves the mapped writable pages.
    fn idx(&self, rel: i64, len: i64) -> usize {
        match rel.checked_add(len) {
            Some(end) if rel >= MEM_LO && end <= MEM_HI => (rel - MEM_LO) as usize,
            _ => segv(),
        }
    }

    fn read(&self, rel: i64, len: usize) -> &[u8] {
        let i = self.idx(rel, len as i64);
        &self.buf[i..i + len]
    }

    fn get_i32(&self, rel: i64) -> i32 {
        let s = self.read(rel, 4);
        i32::from_le_bytes([s[0], s[1], s[2], s[3]])
    }

    fn set_i32(&mut self, rel: i64, v: i32) {
        let i = self.idx(rel, 4);
        self.buf[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn get_u64(&self, rel: i64) -> u64 {
        let s = self.read(rel, 8);
        u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
    }

    fn set_u64(&mut self, rel: i64, v: u64) {
        let i = self.idx(rel, 8);
        self.buf[i..i + 8].copy_from_slice(&v.to_le_bytes());
    }

    /// Contents of the NUL terminated string at `rel`. Walking off the mapped
    /// pages while looking for the terminator faults, as it does in C.
    fn cstr(&self, rel: i64) -> Vec<u8> {
        let mut out = Vec::new();
        let mut r = rel;
        loop {
            let b = self.read(r, 1)[0];
            if b == 0 {
                return out;
            }
            out.push(b);
            r += 1;
        }
    }

    /// `strcpy(base + rel, src)` where `src` is the string contents (no NUL).
    fn strcpy(&mut self, rel: i64, src: &[u8]) {
        let i = self.idx(rel, src.len() as i64 + 1);
        self.buf[i..i + src.len()].copy_from_slice(src);
        self.buf[i + src.len()] = 0;
    }

    fn set_u8(&mut self, rel: i64, v: u8) {
        let i = self.idx(rel, 1);
        self.buf[i] = v;
    }

    /// Struct assignment, e.g. `files[j] = files[j + 1]`.
    fn copy(&mut self, dst: i64, src: i64, len: i64) {
        let s = self.idx(src, len);
        let d = self.idx(dst, len);
        self.buf.copy_within(s..s + len as usize, d);
    }
}

// ---------------------------------------------------------------------------
// C string helpers (operating on the fixed size stack buffers)
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

/// glibc `strcmp`: difference of the first differing bytes as `unsigned char`.
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

/// C `atoi` (glibc: `(int) strtol(s, NULL, 10)`, saturating in `long`).
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

// ---------------------------------------------------------------------------
// stdout: glibc's fully buffered stream
// ---------------------------------------------------------------------------

const IO_BUFSIZ: usize = 4096;

struct Out {
    fd: ManuallyDrop<std::fs::File>,
    buf: Vec<u8>,
}

impl Out {
    fn new() -> Out {
        Out {
            fd: ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(1) }),
            buf: Vec::with_capacity(IO_BUFSIZ),
        }
    }

    fn raw(&mut self, data: &[u8]) {
        let _ = self.fd.write_all(data);
    }

    fn drain(&mut self) {
        if !self.buf.is_empty() {
            let data = std::mem::take(&mut self.buf);
            self.raw(&data);
            self.buf = data;
            self.buf.clear();
        }
    }

    /// `_IO_new_file_xsputn`: fill the buffer; if anything is left over flush
    /// the (now full) buffer, write whole blocks straight through, and buffer
    /// the tail.
    fn b(&mut self, data: &[u8]) {
        let mut to_do = data;

        let space = IO_BUFSIZ - self.buf.len();
        if space > 0 {
            let n = space.min(to_do.len());
            self.buf.extend_from_slice(&to_do[..n]);
            to_do = &to_do[n..];
        }

        if !to_do.is_empty() {
            self.drain();
            let do_write = to_do.len() - to_do.len() % IO_BUFSIZ;
            if do_write > 0 {
                let head = to_do[..do_write].to_vec();
                self.raw(&head);
                to_do = &to_do[do_write..];
            }
            self.buf.extend_from_slice(to_do);
        }
    }

    fn s(&mut self, s: &str) {
        self.b(s.as_bytes());
    }

    fn d(&mut self, v: i32) {
        self.s(&v.to_string());
    }

    fn flush(&mut self) {
        self.drain();
    }
}

// ---------------------------------------------------------------------------
// interpreter state
// ---------------------------------------------------------------------------

type Args = [[u8; MAX_COMMAND]; MAX_ARGS];

struct App {
    mem: Mem,
    out: Out,
}

impl App {
    fn new() -> App {
        App {
            mem: Mem::new(),
            out: Out::new(),
        }
    }

    // --- global accessors -------------------------------------------------
    fn user_count(&self) -> i32 {
        self.mem.get_i32(G_USER_COUNT)
    }
    fn set_user_count(&mut self, v: i32) {
        self.mem.set_i32(G_USER_COUNT, v);
    }
    fn file_count(&self) -> i32 {
        self.mem.get_i32(G_FILE_COUNT)
    }
    fn set_file_count(&mut self, v: i32) {
        self.mem.set_i32(G_FILE_COUNT, v);
    }
    fn variable_count(&self) -> i32 {
        self.mem.get_i32(G_VARIABLE_COUNT)
    }
    fn set_variable_count(&mut self, v: i32) {
        self.mem.set_i32(G_VARIABLE_COUNT, v);
    }
    fn debug_mode(&self) -> i32 {
        self.mem.get_i32(G_DEBUG_MODE)
    }
    fn verbose_mode(&self) -> i32 {
        self.mem.get_i32(G_VERBOSE_MODE)
    }

    /// `&users[i]` as a stored pointer value.
    fn user_ptr(i: i64) -> u64 {
        (BASE_ADDR + G_USERS + i.wrapping_mul(U_SZ)) as u64
    }

    /// Offset of `users[i]`, indexed the way C does (`int` sign extended).
    fn user_at(i: i32) -> i64 {
        G_USERS + (i as i64).wrapping_mul(U_SZ)
    }
    fn file_at(i: i32) -> i64 {
        G_FILES + (i as i64).wrapping_mul(F_SZ)
    }
    fn var_at(i: i32) -> i64 {
        G_VARIABLES + (i as i64).wrapping_mul(V_SZ)
    }

    /// `current_user` as an offset relative to `&users`, or `None` for NULL.
    fn cu(&self) -> Option<i64> {
        let p = self.mem.get_u64(G_CURRENT_USER);
        if p == 0 {
            None
        } else {
            Some((p as i64).wrapping_sub(BASE_ADDR))
        }
    }

    fn set_cu(&mut self, v: u64) {
        self.mem.set_u64(G_CURRENT_USER, v);
    }

    /// `current_user && current_user->logged_in`
    fn cu_logged_in(&self) -> bool {
        match self.cu() {
            None => false,
            Some(r) => self.mem.get_i32(r + U_LOGGED) != 0,
        }
    }

    fn cu_name(&self) -> Vec<u8> {
        self.mem.cstr(self.cu().unwrap() + U_NAME)
    }

    // --- user management -------------------------------------------------
    fn cmd_adduser(&mut self, args: &Args, arg_count: i32) {
        if arg_count < 2 {
            self.out
                .s("Usage: adduser <username> <password> [permission_level]\n");
            return;
        }

        if self.user_count() >= MAX_USERS {
            self.out.s("Error: Maximum users reached\n");
            return;
        }

        let mut i = 0i32;
        while i < self.user_count() {
            if c_strcmp(&self.mem.cstr(Self::user_at(i) + U_NAME), &args[0]) == 0 {
                self.out.s("Error: User '");
                self.out.b(cstr(&args[0]));
                self.out.s("' already exists\n");
                return;
            }
            i += 1;
        }

        // Each of these statements re-reads `user_count`, which the two
        // strcpy calls may have overwritten by overflowing the struct.
        let name = cstr(&args[0]).to_vec();
        let uc = self.user_count();
        self.mem.strcpy(Self::user_at(uc) + U_NAME, &name);

        let pass = cstr(&args[1]).to_vec();
        let uc = self.user_count();
        self.mem.strcpy(Self::user_at(uc) + U_PASS, &pass);

        let level = if arg_count >= 3 { c_atoi(&args[2]) } else { 1 };
        let uc = self.user_count();
        self.mem.set_i32(Self::user_at(uc) + U_PERM, level);
        let uc = self.user_count();
        self.mem.set_i32(Self::user_at(uc) + U_LOGGED, 0);
        let uc = self.user_count();
        self.set_user_count(uc.wrapping_add(1));

        let shown = self
            .mem
            .get_i32(Self::user_at(self.user_count().wrapping_sub(1)) + U_PERM);
        self.out.s("User '");
        self.out.b(cstr(&args[0]));
        self.out.s("' added with permission level ");
        self.out.d(shown);
        self.out.s("\n");
    }

    fn cmd_login(&mut self, args: &Args, arg_count: i32) {
        if arg_count < 2 {
            self.out.s("Usage: login <username> <password>\n");
            return;
        }

        if self.cu_logged_in() {
            let name = self.cu_name();
            self.out.s("Error: User '");
            self.out.b(&name);
            self.out.s("' already logged in. Use 'logout' first.\n");
            return;
        }

        let mut i = 0i32;
        while i < self.user_count() {
            if c_strcmp(&self.mem.cstr(Self::user_at(i) + U_NAME), &args[0]) == 0 {
                if c_strcmp(&self.mem.cstr(Self::user_at(i) + U_PASS), &args[1]) == 0 {
                    self.mem.set_i32(Self::user_at(i) + U_LOGGED, 1);
                    self.set_cu(Self::user_ptr(i as i64));
                    let name = self.cu_name();
                    self.out.s("Login successful. Welcome, ");
                    self.out.b(&name);
                    self.out.s("!\n");
                    return;
                } else {
                    self.out.s("Error: Incorrect password\n");
                    return;
                }
            }
            i += 1;
        }

        self.out.s("Error: User not found\n");
    }

    fn cmd_logout(&mut self) {
        if !self.cu_logged_in() {
            self.out.s("Error: No user logged in\n");
            return;
        }

        let name = self.cu_name();
        self.out.s("Goodbye, ");
        self.out.b(&name);
        self.out.s("!\n");
        let r = self.cu().unwrap();
        self.mem.set_i32(r + U_LOGGED, 0);
        self.set_cu(0);
    }

    fn cmd_whoami(&mut self) {
        if !self.cu_logged_in() {
            self.out.s("Not logged in\n");
            return;
        }

        let name = self.cu_name();
        let level = self.mem.get_i32(self.cu().unwrap() + U_PERM);
        self.out.s("Current user: ");
        self.out.b(&name);
        self.out.s("\n");
        self.out.s("Permission level: ");
        self.out.d(level);
        self.out.s("\n");
    }

    fn cmd_listusers(&mut self) {
        if self.user_count() == 0 {
            self.out.s("No users registered\n");
            return;
        }

        self.out.s("Registered users:\n");
        let mut i = 0i32;
        while i < self.user_count() {
            let at = Self::user_at(i);
            let name = self.mem.cstr(at + U_NAME);
            let level = self.mem.get_i32(at + U_PERM);
            let logged = self.mem.get_i32(at + U_LOGGED) != 0;
            self.out.s("  ");
            self.out.b(&name);
            self.out.s(" (level ");
            self.out.d(level);
            self.out.s(") ");
            self.out.s(if logged { "[logged in]" } else { "" });
            self.out.s("\n");
            i += 1;
        }
    }

    // --- file management -------------------------------------------------
    fn cmd_createfile(&mut self, args: &Args, arg_count: i32) {
        if !self.cu_logged_in() {
            self.out.s("Error: Must be logged in\n");
            return;
        }

        if arg_count < 1 {
            self.out.s("Usage: createfile <filename> [content]\n");
            return;
        }

        if self.file_count() >= MAX_FILES {
            self.out.s("Error: Maximum files reached\n");
            return;
        }

        let mut i = 0i32;
        while i < self.file_count() {
            if c_strcmp(&self.mem.cstr(Self::file_at(i) + F_NAME), &args[0]) == 0 {
                self.out.s("Error: File '");
                self.out.b(cstr(&args[0]));
                self.out.s("' already exists\n");
                return;
            }
            i += 1;
        }

        let fname = cstr(&args[0]).to_vec();
        let fc = self.file_count();
        self.mem.strcpy(Self::file_at(fc) + F_NAME, &fname);

        let owner = self.cu_name();
        let fc = self.file_count();
        self.mem.strcpy(Self::file_at(fc) + F_OWNER, &owner);

        let fc = self.file_count();
        self.mem.set_i32(Self::file_at(fc) + F_PERM, 755);

        if arg_count >= 2 {
            let content = cstr(&args[1]).to_vec();
            let fc = self.file_count();
            self.mem.strcpy(Self::file_at(fc) + F_CONTENT, &content);
        } else {
            let fc = self.file_count();
            self.mem.set_u8(Self::file_at(fc) + F_CONTENT, 0);
        }

        let fc = self.file_count();
        self.set_file_count(fc.wrapping_add(1));
        self.out.s("File '");
        self.out.b(cstr(&args[0]));
        self.out.s("' created\n");
    }

    fn cmd_readfile(&mut self, args: &Args, arg_count: i32) {
        if arg_count < 1 {
            self.out.s("Usage: readfile <filename>\n");
            return;
        }

        let mut i = 0i32;
        while i < self.file_count() {
            let at = Self::file_at(i);
            if c_strcmp(&self.mem.cstr(at + F_NAME), &args[0]) == 0 {
                let fname = self.mem.cstr(at + F_NAME);
                let owner = self.mem.cstr(at + F_OWNER);
                let perm = self.mem.get_i32(at + F_PERM);
                let content = self.mem.cstr(at + F_CONTENT);
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
            i += 1;
        }

        self.out.s("Error: File '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_writefile(&mut self, args: &Args, arg_count: i32) {
        if !self.cu_logged_in() {
            self.out.s("Error: Must be logged in\n");
            return;
        }

        if arg_count < 2 {
            self.out.s("Usage: writefile <filename> <content>\n");
            return;
        }

        let mut i = 0i32;
        while i < self.file_count() {
            let at = Self::file_at(i);
            if c_strcmp(&self.mem.cstr(at + F_NAME), &args[0]) == 0 {
                let cu_name = self.cu_name();
                let cu_level = self.mem.get_i32(self.cu().unwrap() + U_PERM);
                if c_strcmp(&self.mem.cstr(at + F_OWNER), &cu_name) == 0 || cu_level >= 5 {
                    let content = cstr(&args[1]).to_vec();
                    self.mem.strcpy(at + F_CONTENT, &content);
                    self.out.s("File '");
                    self.out.b(cstr(&args[0]));
                    self.out.s("' updated\n");
                    return;
                } else {
                    self.out.s("Error: Permission denied\n");
                    return;
                }
            }
            i += 1;
        }

        self.out.s("Error: File '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_deletefile(&mut self, args: &Args, arg_count: i32) {
        if !self.cu_logged_in() {
            self.out.s("Error: Must be logged in\n");
            return;
        }

        if arg_count < 1 {
            self.out.s("Usage: deletefile <filename>\n");
            return;
        }

        let mut i = 0i32;
        while i < self.file_count() {
            let at = Self::file_at(i);
            if c_strcmp(&self.mem.cstr(at + F_NAME), &args[0]) == 0 {
                let cu_name = self.cu_name();
                let cu_level = self.mem.get_i32(self.cu().unwrap() + U_PERM);
                if c_strcmp(&self.mem.cstr(at + F_OWNER), &cu_name) == 0 || cu_level >= 9 {
                    let mut j = i;
                    while j < self.file_count().wrapping_sub(1) {
                        self.mem
                            .copy(Self::file_at(j), Self::file_at(j.wrapping_add(1)), F_SZ);
                        j += 1;
                    }
                    let fc = self.file_count();
                    self.set_file_count(fc.wrapping_sub(1));
                    self.out.s("File '");
                    self.out.b(cstr(&args[0]));
                    self.out.s("' deleted\n");
                    return;
                } else {
                    self.out.s("Error: Permission denied\n");
                    return;
                }
            }
            i += 1;
        }

        self.out.s("Error: File '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_listfiles(&mut self) {
        if self.file_count() == 0 {
            self.out.s("No files\n");
            return;
        }

        self.out.s("Files:\n");
        let mut i = 0i32;
        while i < self.file_count() {
            let at = Self::file_at(i);
            let fname = self.mem.cstr(at + F_NAME);
            let owner = self.mem.cstr(at + F_OWNER);
            let perm = self.mem.get_i32(at + F_PERM);
            self.out.s("  ");
            self.out.b(&fname);
            self.out.s(" (owner: ");
            self.out.b(&owner);
            self.out.s(", perm: ");
            self.out.d(perm);
            self.out.s(")\n");
            i += 1;
        }
    }

    // --- variables -------------------------------------------------------
    fn cmd_set(&mut self, args: &Args, arg_count: i32) {
        if arg_count < 2 {
            self.out.s("Usage: set <name> <value>\n");
            return;
        }

        let mut i = 0i32;
        while i < self.variable_count() {
            let at = Self::var_at(i);
            if c_strcmp(&self.mem.cstr(at + V_NAME), &args[0]) == 0 {
                let value = cstr(&args[1]).to_vec();
                self.mem.strcpy(at + V_VALUE, &value);
                self.out.s("Variable '");
                self.out.b(cstr(&args[0]));
                self.out.s("' updated\n");
                return;
            }
            i += 1;
        }

        if self.variable_count() >= MAX_VARIABLES {
            self.out.s("Error: Maximum variables reached\n");
            return;
        }

        let name = cstr(&args[0]).to_vec();
        let vc = self.variable_count();
        self.mem.strcpy(Self::var_at(vc) + V_NAME, &name);

        let value = cstr(&args[1]).to_vec();
        let vc = self.variable_count();
        self.mem.strcpy(Self::var_at(vc) + V_VALUE, &value);

        let vc = self.variable_count();
        self.set_variable_count(vc.wrapping_add(1));
        self.out.s("Variable '");
        self.out.b(cstr(&args[0]));
        self.out.s("' set\n");
    }

    fn cmd_get(&mut self, args: &Args, arg_count: i32) {
        if arg_count < 1 {
            self.out.s("Usage: get <name>\n");
            return;
        }

        let mut i = 0i32;
        while i < self.variable_count() {
            let at = Self::var_at(i);
            if c_strcmp(&self.mem.cstr(at + V_NAME), &args[0]) == 0 {
                let name = self.mem.cstr(at + V_NAME);
                let value = self.mem.cstr(at + V_VALUE);
                self.out.b(&name);
                self.out.s(" = ");
                self.out.b(&value);
                self.out.s("\n");
                return;
            }
            i += 1;
        }

        self.out.s("Error: Variable '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_unset(&mut self, args: &Args, arg_count: i32) {
        if arg_count < 1 {
            self.out.s("Usage: unset <name>\n");
            return;
        }

        let mut i = 0i32;
        while i < self.variable_count() {
            let at = Self::var_at(i);
            if c_strcmp(&self.mem.cstr(at + V_NAME), &args[0]) == 0 {
                let mut j = i;
                while j < self.variable_count().wrapping_sub(1) {
                    self.mem
                        .copy(Self::var_at(j), Self::var_at(j.wrapping_add(1)), V_SZ);
                    j += 1;
                }
                let vc = self.variable_count();
                self.set_variable_count(vc.wrapping_sub(1));
                self.out.s("Variable '");
                self.out.b(cstr(&args[0]));
                self.out.s("' unset\n");
                return;
            }
            i += 1;
        }

        self.out.s("Error: Variable '");
        self.out.b(cstr(&args[0]));
        self.out.s("' not found\n");
    }

    fn cmd_listvars(&mut self) {
        if self.variable_count() == 0 {
            self.out.s("No variables set\n");
            return;
        }

        self.out.s("Variables:\n");
        let mut i = 0i32;
        while i < self.variable_count() {
            let at = Self::var_at(i);
            let name = self.mem.cstr(at + V_NAME);
            let value = self.mem.cstr(at + V_VALUE);
            self.out.s("  ");
            self.out.b(&name);
            self.out.s(" = ");
            self.out.b(&value);
            self.out.s("\n");
            i += 1;
        }
    }

    // --- string comparison ----------------------------------------------
    fn cmd_compare(&mut self, args: &Args, arg_count: i32) {
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

    fn cmd_compare_n(&mut self, args: &Args, arg_count: i32) {
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

    fn cmd_startswith(&mut self, args: &Args, arg_count: i32) {
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

    fn cmd_match(&mut self, args: &Args, arg_count: i32) {
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

    fn cmd_debug(&mut self, args: &Args, arg_count: i32) {
        if arg_count < 1 {
            let on = self.debug_mode() != 0;
            self.out.s("Debug mode: ");
            self.out.s(if on { "ON" } else { "OFF" });
            self.out.s("\n");
            return;
        }

        if c_strcmp(&args[0], b"on") == 0 {
            self.mem.set_i32(G_DEBUG_MODE, 1);
            self.out.s("Debug mode enabled\n");
        } else if c_strcmp(&args[0], b"off") == 0 {
            self.mem.set_i32(G_DEBUG_MODE, 0);
            self.out.s("Debug mode disabled\n");
        } else {
            self.out.s("Usage: debug [on|off]\n");
        }
    }

    fn cmd_verbose(&mut self, args: &Args, arg_count: i32) {
        if arg_count < 1 {
            let on = self.verbose_mode() != 0;
            self.out.s("Verbose mode: ");
            self.out.s(if on { "ON" } else { "OFF" });
            self.out.s("\n");
            return;
        }

        if c_strcmp(&args[0], b"on") == 0 {
            self.mem.set_i32(G_VERBOSE_MODE, 1);
            self.out.s("Verbose mode enabled\n");
        } else if c_strcmp(&args[0], b"off") == 0 {
            self.mem.set_i32(G_VERBOSE_MODE, 0);
            self.out.s("Verbose mode disabled\n");
        } else {
            self.out.s("Usage: verbose [on|off]\n");
        }
    }

    fn cmd_status(&mut self) {
        self.out.s("\n=== System Status ===\n");
        let uc = self.user_count();
        self.out.s("Users: ");
        self.out.d(uc);
        self.out.s("/");
        self.out.d(MAX_USERS);
        self.out.s("\n");
        let fc = self.file_count();
        self.out.s("Files: ");
        self.out.d(fc);
        self.out.s("/");
        self.out.d(MAX_FILES);
        self.out.s("\n");
        let vc = self.variable_count();
        self.out.s("Variables: ");
        self.out.d(vc);
        self.out.s("/");
        self.out.d(MAX_VARIABLES);
        self.out.s("\n");
        self.out.s("Current user: ");
        if self.cu_logged_in() {
            let name = self.cu_name();
            self.out.b(&name);
        } else {
            self.out.s("none");
        }
        self.out.s("\n");
        let dm = self.debug_mode() != 0;
        self.out.s("Debug mode: ");
        self.out.s(if dm { "ON" } else { "OFF" });
        self.out.s("\n");
        let vm = self.verbose_mode() != 0;
        self.out.s("Verbose mode: ");
        self.out.s(if vm { "ON" } else { "OFF" });
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
        let mut args: Args = [[0u8; MAX_COMMAND]; MAX_ARGS];
        let mut arg_count: i32 = 0;

        parse_command(input, &mut command, &mut args, &mut arg_count);

        if c_strlen(&command) == 0 {
            return;
        }

        if self.debug_mode() != 0 {
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

fn parse_command(input: &[u8], cmd: &mut [u8; MAX_COMMAND], args: &mut Args, arg_count: &mut i32) {
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

/// Unbuffered-at-our-level reader over fd 0 with an internal buffer, used to
/// implement `fgets`.
struct In {
    f: ManuallyDrop<std::fs::File>,
    buf: [u8; 4096],
    pos: usize,
    len: usize,
    eof: bool,
}

impl In {
    fn new() -> In {
        In {
            f: ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(0) }),
            buf: [0u8; 4096],
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
            match std::io::Read::read(&mut *self.f, &mut self.buf) {
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
}

/// `fgets(buf, buf.len(), stdin)`: reads at most `buf.len() - 1` bytes,
/// stopping after a newline, and NUL terminates. Returns false when nothing
/// could be read (EOF/error), matching a NULL return.
fn fgets(r: &mut In, buf: &mut [u8]) -> bool {
    let limit = buf.len() - 1;
    let mut i = 0usize;
    while i < limit {
        match r.next_byte() {
            None => break,
            Some(byte) => {
                buf[i] = byte;
                i += 1;
                if byte == b'\n' {
                    break;
                }
            }
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
    // The C program leaves SIGPIPE at its default disposition; Rust's runtime
    // ignores it. Restore the C behaviour so a closed stdout kills us the same
    // way.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }

    let mut app = App::new();

    app.out.s("|----------------------------------------|\n");
    app.out.s("|   COMMAND INTERPRETER                  |\n");
    app.out.s("|   strcmp/strncmp demonstration         |\n");
    app.out.s("|----------------------------------------|\n");
    app.out.s("Type 'help' for available commands\n\n");

    let mut reader = In::new();
    let mut input = [0u8; MAX_INPUT];

    loop {
        app.out.s("> ");

        if !fgets(&mut reader, &mut input) {
            break;
        }

        strip_newline(&mut input);

        if app.verbose_mode() != 0 {
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
