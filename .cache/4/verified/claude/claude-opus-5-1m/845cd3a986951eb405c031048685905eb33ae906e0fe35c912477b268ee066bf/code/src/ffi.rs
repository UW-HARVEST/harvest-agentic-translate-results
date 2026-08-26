//! C-ABI surface of the translation.
//!
//! Every non-`static` function of `c_src/src/{tokenizer,analyzer,main}.c` is
//! re-exported here under its original symbol name with the original C
//! signature.  The file-scope `static` variables of those translation units
//! become the process-global singletons below.
//!
//! Layout note: `token_t`, `tokenizer_ops_t` and `analysis_result_t` are
//! mirrored as `#[repr(C)]` structs so that values crossing this boundary are
//! bit-compatible with the C build.

use core::cell::UnsafeCell;
use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::analyzer::{AnalysisResult, Analyzer};
use crate::cio::{err, strncat, strstr, up_to_nul, In, Out};
use crate::driver;
use crate::tokenizer::{Token, TokenType, Tokenizer, MAX_TOKEN_LENGTH};
use std::io::Write;

// ---------------------------------------------------------------------------
// C types
// ---------------------------------------------------------------------------

/// `token_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CToken {
    pub ttype: c_int,
    pub value: [c_char; MAX_TOKEN_LENGTH],
    pub length: usize,
    pub line: c_int,
    pub column: c_int,
}

impl CToken {
    fn zeroed() -> CToken {
        CToken {
            ttype: 0,
            value: [0; MAX_TOKEN_LENGTH],
            length: 0,
            line: 0,
            column: 0,
        }
    }

    fn from_token(t: &Token) -> CToken {
        let mut ct = CToken::zeroed();
        ct.ttype = t.ttype as c_int;
        // `create_token` never produces more than MAX_TOKEN_LENGTH - 1 bytes,
        // and the C code always NUL-terminates at `token.length`.
        let n = if t.value.len() < MAX_TOKEN_LENGTH - 1 {
            t.value.len()
        } else {
            MAX_TOKEN_LENGTH - 1
        };
        for i in 0..n {
            ct.value[i] = t.value[i] as c_char;
        }
        ct.length = t.length;
        ct.line = t.line;
        ct.column = t.column;
        ct
    }

    /// The `token.value` C string.
    fn value_bytes(&self) -> Vec<u8> {
        let raw: &[u8] =
            unsafe { core::slice::from_raw_parts(self.value.as_ptr() as *const u8, self.value.len()) };
        up_to_nul(raw).to_vec()
    }
}

/// `tokenizer_ops_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CTokenizerOps {
    pub next_token: Option<extern "C" fn() -> CToken>,
    pub peek_token: Option<extern "C" fn() -> CToken>,
    pub reset: Option<extern "C" fn()>,
    pub load_text: Option<extern "C" fn(*const c_char) -> c_int>,
    pub get_stats: Option<extern "C" fn(*mut usize, *mut usize, *mut usize)>,
}

impl CTokenizerOps {
    const fn null() -> CTokenizerOps {
        CTokenizerOps {
            next_token: None,
            peek_token: None,
            reset: None,
            load_text: None,
            get_stats: None,
        }
    }
}

/// The C code calls `tokenizer_ops.<member>(...)` unconditionally, so a
/// `tokenizer_ops_t` with a NULL member faults on the first indirect call
/// (SIGSEGV, with whatever was still buffered in `stdout` discarded).
///
/// Reproduce that instead of panicking, so both builds die the same way with the
/// same output.
#[inline(never)]
fn null_ops_member() -> ! {
    unsafe {
        ptr::read_volatile(ptr::null::<u8>());
    }
    // Not reached: the volatile read above faults.
    std::process::abort();
}

/// `analysis_result_t`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CAnalysisResult {
    pub word_count: usize,
    pub number_count: usize,
    pub keyword_count: usize,
    pub operator_count: usize,
    pub comment_count: usize,
    pub string_count: usize,
    pub line_count: usize,
    pub char_count: usize,
}

impl From<AnalysisResult> for CAnalysisResult {
    fn from(r: AnalysisResult) -> CAnalysisResult {
        CAnalysisResult {
            word_count: r.word_count,
            number_count: r.number_count,
            keyword_count: r.keyword_count,
            operator_count: r.operator_count,
            comment_count: r.comment_count,
            string_count: r.string_count,
            line_count: r.line_count,
            char_count: r.char_count,
        }
    }
}

impl From<CAnalysisResult> for AnalysisResult {
    fn from(r: CAnalysisResult) -> AnalysisResult {
        AnalysisResult {
            word_count: r.word_count,
            number_count: r.number_count,
            keyword_count: r.keyword_count,
            operator_count: r.operator_count,
            comment_count: r.comment_count,
            string_count: r.string_count,
            line_count: r.line_count,
            char_count: r.char_count,
        }
    }
}

// ---------------------------------------------------------------------------
// Process-global state (the C `static` variables)
// ---------------------------------------------------------------------------

struct Global<T>(UnsafeCell<Option<T>>);

// The C program is single threaded and accesses its statics without any
// synchronisation; the same contract applies to callers of this library.
unsafe impl<T> Sync for Global<T> {}

impl<T> Global<T> {
    const fn new() -> Global<T> {
        Global(UnsafeCell::new(None))
    }

    #[allow(clippy::mut_from_ref)]
    fn get(&self, init: impl FnOnce() -> T) -> &mut T {
        unsafe {
            let slot = &mut *self.0.get();
            if slot.is_none() {
                *slot = Some(init());
            }
            slot.as_mut().unwrap()
        }
    }
}

struct Plain<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Plain<T> {}

impl<T> Plain<T> {
    const fn new(v: T) -> Plain<T> {
        Plain(UnsafeCell::new(v))
    }

    #[allow(clippy::mut_from_ref)]
    fn get(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

static TOKENIZER: Global<Tokenizer> = Global::new();
static ANALYZER: Global<Analyzer> = Global::new();
static OUT: Global<Out> = Global::new();
static STDIN: Global<In> = Global::new();
/// `static tokenizer_ops_t tokenizer_ops;` from analyzer.c
static OPS: Plain<CTokenizerOps> = Plain::new(CTokenizerOps::null());

fn tokenizer() -> &'static mut Tokenizer {
    TOKENIZER.get(Tokenizer::new)
}

fn analyzer() -> &'static mut Analyzer {
    ANALYZER.get(Analyzer::new)
}

fn out() -> &'static mut Out {
    OUT.get(|| {
        // C flushes its stdout buffer when the process exits.
        unsafe {
            atexit(flush_out_at_exit);
        }
        Out::new()
    })
}

fn stdin_() -> &'static mut In {
    STDIN.get(In::new)
}

extern "C" {
    fn atexit(cb: extern "C" fn()) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
}

extern "C" fn flush_out_at_exit() {
    out().flush_all();
}

/// Not part of the C API: lets a test drain the emulated `stdout` buffer at a
/// point of its choosing (the C build is drained with `fflush`).
#[no_mangle]
pub extern "C" fn text_analyzer_flush_stdout() {
    out().flush_all();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The bytes of a C string, or `None` for a NULL pointer.
unsafe fn cstr<'a>(p: *const c_char) -> Option<&'a [u8]> {
    if p.is_null() {
        return None;
    }
    let mut len = 0usize;
    while *p.add(len) != 0 {
        len += 1;
    }
    Some(core::slice::from_raw_parts(p as *const u8, len))
}

// ---------------------------------------------------------------------------
// tokenizer.c
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn tokenizer_next_token() -> CToken {
    CToken::from_token(&tokenizer().next_token())
}

#[no_mangle]
pub extern "C" fn tokenizer_peek_token() -> CToken {
    CToken::from_token(&tokenizer().peek_token())
}

#[no_mangle]
pub extern "C" fn tokenizer_reset() {
    tokenizer().reset();
}

#[no_mangle]
pub extern "C" fn tokenizer_load_text(text: *const c_char) -> c_int {
    let bytes = match unsafe { cstr(text) } {
        None => return -1,
        Some(b) => b,
    };
    tokenizer().load_text(bytes)
}

#[no_mangle]
pub extern "C" fn tokenizer_get_stats(lines: *mut usize, tokens: *mut usize, chars: *mut usize) {
    let (l, t, c) = tokenizer().get_stats();
    unsafe {
        if !lines.is_null() {
            *lines = l;
        }
        if !tokens.is_null() {
            *tokens = t;
        }
        if !chars.is_null() {
            *chars = c;
        }
    }
}

#[no_mangle]
pub extern "C" fn get_tokenizer_ops() -> CTokenizerOps {
    CTokenizerOps {
        next_token: Some(tokenizer_next_token),
        peek_token: Some(tokenizer_peek_token),
        reset: Some(tokenizer_reset),
        load_text: Some(tokenizer_load_text),
        get_stats: Some(tokenizer_get_stats),
    }
}

// ---------------------------------------------------------------------------
// analyzer.c
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn analyzer_init(ops: CTokenizerOps) {
    *OPS.get() = ops;
    analyzer().init();
}

#[no_mangle]
pub extern "C" fn analyze_text(text: *const c_char) -> CAnalysisResult {
    let mut result = AnalysisResult::default();

    if !analyzer().is_initialized() {
        err(b"Error: Analyzer not initialized\n");
        return result.into();
    }

    let ops = *OPS.get();

    // Load text using function pointer
    let load_text = match ops.load_text {
        Some(f) => f,
        None => null_ops_member(),
    };
    if load_text(text) != 0 {
        err(b"Error: Failed to load text\n");
        return result.into();
    }

    // Process all tokens using function pointers
    let next_token = match ops.next_token {
        Some(f) => f,
        None => null_ops_member(),
    };
    loop {
        let token = next_token();
        if token.ttype == TokenType::Eof as c_int {
            break;
        }
        let value = token.value_bytes();
        analyzer().account_token(token.ttype, &value, &mut result);
    }

    // Get final statistics using function pointer
    let get_stats = match ops.get_stats {
        Some(f) => f,
        None => null_ops_member(),
    };
    let mut lines: usize = 0;
    let mut tokens: usize = 0;
    let mut chars: usize = 0;
    get_stats(&mut lines, &mut tokens, &mut chars);

    result.line_count = lines;
    result.char_count = chars;
    let _ = tokens;

    result.into()
}

#[no_mangle]
pub extern "C" fn print_token_distribution() {
    analyzer().print_token_distribution(out());
}

#[no_mangle]
pub extern "C" fn calculate_complexity_score() -> c_int {
    analyzer().calculate_complexity_score()
}

#[no_mangle]
pub extern "C" fn find_patterns(pattern: *const c_char) {
    let pattern = match unsafe { cstr(pattern) } {
        None => return,
        Some(p) => p,
    };
    if !analyzer().is_initialized() {
        return;
    }

    {
        let o = out();
        o.puts("\n=== Searching for pattern: '");
        o.put(pattern);
        o.puts("' ===\n");
    }

    let ops = *OPS.get();

    // Reset tokenizer using function pointer
    let reset = match ops.reset {
        Some(f) => f,
        None => null_ops_member(),
    };
    reset();

    let mut count: c_int = 0;
    let next_token = match ops.next_token {
        Some(f) => f,
        None => null_ops_member(),
    };

    loop {
        let token = next_token();
        if token.ttype == TokenType::Eof as c_int {
            break;
        }
        let value = token.value_bytes();
        if strstr(&value, pattern) {
            let o = out();
            let _ = write!(o, "Line {}, Column {}: ", token.line, token.column);
            o.put(&value);
            o.puts("\n");
            count += 1;
        }
    }

    let _ = write!(out(), "Found {} occurrences\n", count);
}

// ---------------------------------------------------------------------------
// main.c
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn print_menu() {
    driver::print_menu(out());
}

#[no_mangle]
pub extern "C" fn print_analysis_result(result: CAnalysisResult) {
    driver::print_analysis_result(out(), AnalysisResult::from(result));
}

#[no_mangle]
pub extern "C" fn interactive_tokenizer(ops: CTokenizerOps) {
    out().puts("\nEnter text (empty line to stop):\n");

    let mut input: Vec<u8> = Vec::new();

    while let Some(line) = stdin_().fgets(256, out()) {
        if line[0] == b'\n' {
            break;
        }
        let room = driver::MAX_INPUT_SIZE
            .saturating_sub(input.len())
            .saturating_sub(1);
        strncat(&mut input, &line, room);
    }

    let load_text = match ops.load_text {
        Some(f) => f,
        None => null_ops_member(),
    };
    let mut c_input = input.clone();
    c_input.push(0);
    if load_text(c_input.as_ptr() as *const c_char) != 0 {
        out().puts("Failed to load text\n");
        return;
    }

    out().puts("\n=== Tokens ===\n");

    let token_type_names = driver::TOKEN_TYPE_NAMES;

    let mut count: c_int = 0;
    let next_token = match ops.next_token {
        Some(f) => f,
        None => null_ops_member(),
    };

    loop {
        let token = next_token();
        if token.ttype == TokenType::Eof as c_int {
            break;
        }

        // The C code indexes a 12-element table with `token.type`.
        let name = if token.ttype >= 0 && (token.ttype as usize) < token_type_names.len() {
            token_type_names[token.ttype as usize]
        } else {
            ""
        };

        let value = token.value_bytes();
        let o = out();
        let _ = write!(o, "[{}] '", name);
        o.put(&value);
        let _ = write!(o, "' (L{}:C{})\n", token.line, token.column);
        count += 1;

        if count > 100 {
            o.puts("... (truncated, too many tokens)\n");
            break;
        }
    }
}

#[no_mangle]
pub extern "C" fn read_file(filename: *const c_char) -> *mut c_char {
    // `fopen(NULL, "r")` fails, and glibc's `%s` prints a NULL pointer as
    // "(null)".
    let name = match unsafe { cstr(filename) } {
        None => {
            err(b"Error: Could not open file '(null)'\n");
            return ptr::null_mut();
        }
        Some(n) => n,
    };

    let content = match driver::read_file(name) {
        None => return ptr::null_mut(),
        Some(c) => c,
    };

    let buf = unsafe { malloc(content.len() + 1) } as *mut c_char;
    if buf.is_null() {
        err(b"Error: Memory allocation failed\n");
        return ptr::null_mut();
    }
    unsafe {
        ptr::copy_nonoverlapping(content.as_ptr(), buf as *mut u8, content.len());
        *buf.add(content.len()) = 0;
    }
    buf
}

/// `int main(void)` of c_src/src/main.c.
///
/// Not exported when the library is compiled as a test harness, because that
/// harness brings its own `main` (see `[lib] test = false` in Cargo.toml).
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    // Get tokenizer operations (function pointers)
    let ops = get_tokenizer_ops();
    *OPS.get() = ops;

    driver::run(out(), stdin_(), tokenizer(), analyzer());
    0
}
