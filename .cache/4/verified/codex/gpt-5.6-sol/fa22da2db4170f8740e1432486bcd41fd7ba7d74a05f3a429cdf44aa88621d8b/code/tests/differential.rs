use std::collections::BTreeSet;
use std::env;
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use libloading::Library;

const MAX_TOKEN_LENGTH: usize = 256;
const MAX_BUFFER_SIZE: usize = 8192;

#[repr(C)]
#[derive(Clone, Copy)]
struct Token {
    token_type: c_int,
    value: [c_char; MAX_TOKEN_LENGTH],
    length: usize,
    line: c_int,
    column: c_int,
}

impl Token {
    fn new(token_type: c_int) -> Self {
        Self {
            token_type,
            value: [0; MAX_TOKEN_LENGTH],
            length: 0,
            line: 1,
            column: 1,
        }
    }

    fn bytes(&self) -> Vec<u8> {
        self.value[..self.length.min(MAX_TOKEN_LENGTH)]
            .iter()
            .map(|&byte| byte as u8)
            .collect()
    }
}

type NextTokenFn = unsafe extern "C" fn() -> Token;
type PeekTokenFn = unsafe extern "C" fn() -> Token;
type ResetFn = unsafe extern "C" fn();
type LoadTextFn = unsafe extern "C" fn(*const c_char) -> c_int;
type GetStatsFn = unsafe extern "C" fn(*mut usize, *mut usize, *mut usize);

#[repr(C)]
#[derive(Clone, Copy)]
struct TokenizerOps {
    next_token: Option<NextTokenFn>,
    peek_token: Option<PeekTokenFn>,
    reset: Option<ResetFn>,
    load_text: Option<LoadTextFn>,
    get_stats: Option<GetStatsFn>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct AnalysisResult {
    word_count: usize,
    number_count: usize,
    keyword_count: usize,
    operator_count: usize,
    comment_count: usize,
    string_count: usize,
    line_count: usize,
    char_count: usize,
}

type GetTokenizerOps = unsafe extern "C" fn() -> TokenizerOps;
type TokenizerNext = unsafe extern "C" fn() -> Token;
type TokenizerPeek = unsafe extern "C" fn() -> Token;
type TokenizerReset = unsafe extern "C" fn();
type TokenizerLoad = unsafe extern "C" fn(*const c_char) -> c_int;
type TokenizerStats = unsafe extern "C" fn(*mut usize, *mut usize, *mut usize);
type AnalyzerInit = unsafe extern "C" fn(TokenizerOps);
type AnalyzeText = unsafe extern "C" fn(*const c_char) -> AnalysisResult;
type VoidFn = unsafe extern "C" fn();
type FindPatterns = unsafe extern "C" fn(*const c_char);

struct Api {
    _library: Library,
    get_ops: GetTokenizerOps,
    next: TokenizerNext,
    peek: TokenizerPeek,
    reset: TokenizerReset,
    load: TokenizerLoad,
    stats: TokenizerStats,
    init: AnalyzerInit,
    analyze: AnalyzeText,
    distribution: VoidFn,
    complexity: unsafe extern "C" fn() -> c_int,
    find: FindPatterns,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = Library::new(path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $type:ty) => {
                *library
                    .get::<$type>(concat!($name, "\0").as_bytes())
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name))
            };
        }
        Self {
            get_ops: symbol!("get_tokenizer_ops", GetTokenizerOps),
            next: symbol!("tokenizer_next_token", TokenizerNext),
            peek: symbol!("tokenizer_peek_token", TokenizerPeek),
            reset: symbol!("tokenizer_reset", TokenizerReset),
            load: symbol!("tokenizer_load_text", TokenizerLoad),
            stats: symbol!("tokenizer_get_stats", TokenizerStats),
            init: symbol!("analyzer_init", AnalyzerInit),
            analyze: symbol!("analyze_text", AnalyzeText),
            distribution: symbol!("print_token_distribution", VoidFn),
            complexity: symbol!(
                "calculate_complexity_score",
                unsafe extern "C" fn() -> c_int
            ),
            find: symbol!("find_patterns", FindPatterns),
            _library: library,
        }
    }
}

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn capture_fd<T>(fd: c_int, operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        fflush(std::ptr::null_mut());
        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved = dup(fd);
        assert!(saved >= 0);
        assert_eq!(dup2(fds[1], fd), fd);
        close(fds[1]);

        let result = operation();
        fflush(std::ptr::null_mut());
        assert_eq!(dup2(saved, fd), fd);
        close(saved);

        let mut output = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let count = read(fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        close(fds[0]);
        (result, output)
    }
}

fn c_string(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("test input must not contain NUL")
}

fn assert_token(row: usize, c: Token, rust: Token) {
    assert_eq!(c.token_type, rust.token_type, "CONFIGS.md row {row}: type");
    assert_eq!(c.length, rust.length, "CONFIGS.md row {row}: length");
    assert_eq!(c.line, rust.line, "CONFIGS.md row {row}: line");
    assert_eq!(c.column, rust.column, "CONFIGS.md row {row}: column");
    assert_eq!(c.bytes(), rust.bytes(), "CONFIGS.md row {row}: value");
}

unsafe fn load_both(c: &Api, rust: &Api, bytes: &[u8], row: usize) {
    let text = c_string(bytes);
    assert_eq!(
        (c.load)(text.as_ptr()),
        (rust.load)(text.as_ptr()),
        "CONFIGS.md row {row}: load"
    );
}

unsafe fn next_both(c: &Api, rust: &Api, row: usize) -> (Token, Token) {
    let c_token = (c.next)();
    let rust_token = (rust.next)();
    assert_token(row, c_token, rust_token);
    (c_token, rust_token)
}

unsafe fn tokenize_case(c: &Api, rust: &Api, input: &[u8], row: usize) -> Vec<Token> {
    load_both(c, rust, input, row);
    let mut tokens = Vec::new();
    loop {
        let (c_token, _) = next_both(c, rust, row);
        let done = c_token.token_type == 0;
        tokens.push(c_token);
        if done {
            return tokens;
        }
        assert!(
            tokens.len() <= input.len() + 2,
            "tokenizer did not reach EOF"
        );
    }
}

unsafe fn analyze_both(c: &Api, rust: &Api, input: &[u8], row: usize) -> AnalysisResult {
    let text = c_string(input);
    let c_result = (c.analyze)(text.as_ptr());
    let rust_result = (rust.analyze)(text.as_ptr());
    assert_eq!(c_result, rust_result, "CONFIGS.md row {row}");
    c_result
}

unsafe fn output_both(
    fd: c_int,
    row: usize,
    c_operation: impl FnOnce(),
    rust_operation: impl FnOnce(),
) -> Vec<u8> {
    let (_, c_output) = capture_fd(fd, c_operation);
    let (_, rust_output) = capture_fd(fd, rust_operation);
    assert_eq!(c_output, rust_output, "CONFIGS.md row {row}: output");
    c_output
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x4d59_5df4_d0f3_3173)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, start: usize, end: usize) -> usize {
        start + (self.next() as usize % (end - start))
    }

    fn byte_from(&mut self, alphabet: &[u8]) -> u8 {
        alphabet[self.range(0, alphabet.len())]
    }
}

fn mark(rows: &mut BTreeSet<usize>, row: usize) {
    assert!(rows.insert(row), "row {row} was marked twice");
}

static ENUM_POSITION: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn custom_load(_: *const c_char) -> c_int {
    ENUM_POSITION.store(0, Ordering::SeqCst);
    0
}

unsafe extern "C" fn failing_load(_: *const c_char) -> c_int {
    -7
}

unsafe extern "C" fn custom_next() -> Token {
    if ENUM_POSITION.fetch_add(1, Ordering::SeqCst) == 0 {
        Token::new(12)
    } else {
        Token::new(0)
    }
}

unsafe extern "C" fn custom_peek() -> Token {
    custom_next()
}

unsafe extern "C" fn custom_reset() {
    ENUM_POSITION.store(0, Ordering::SeqCst);
}

unsafe extern "C" fn custom_stats(lines: *mut usize, tokens: *mut usize, chars: *mut usize) {
    if let Some(lines) = lines.as_mut() {
        *lines = 0;
    }
    if let Some(tokens) = tokens.as_mut() {
        *tokens = 2;
    }
    if let Some(chars) = chars.as_mut() {
        *chars = 0;
    }
}

fn custom_ops(load: LoadTextFn) -> TokenizerOps {
    TokenizerOps {
        next_token: Some(custom_next),
        peek_token: Some(custom_peek),
        reset: Some(custom_reset),
        load_text: Some(load),
        get_stats: Some(custom_stats),
    }
}

fn rust_library_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("target"));
    let candidates = [
        target.join("debug/deps/libdriver.so"),
        target.join("debug/libdriver.so"),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("Rust cdylib not found under {}", target.display()))
}

#[test]
fn c_and_rust_shared_libraries_match_every_surface_row() {
    unsafe {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c = Api::load(&manifest.join("c_src/build/libdriver_c.so"));
        let rust = Api::load(&rust_library_path());
        let mut configs = BTreeSet::new();
        let mut errors = BTreeSet::new();
        let mut rng = Rng::new();

        // Error rows that require a fresh, uninitialized analyzer.
        let empty = c_string(b"");
        let (c_result, c_error) = capture_fd(2, || (c.analyze)(empty.as_ptr()));
        let (rust_result, rust_error) = capture_fd(2, || (rust.analyze)(empty.as_ptr()));
        assert_eq!(c_result, rust_result);
        assert_eq!(c_error, rust_error);
        assert_eq!(c_result, AnalysisResult::default());
        mark(&mut errors, 1);

        let pattern = c_string(b"x");
        let c_output = output_both(
            1,
            3,
            || (c.find)(pattern.as_ptr()),
            || (rust.find)(pattern.as_ptr()),
        );
        assert!(c_output.is_empty());
        mark(&mut errors, 3);

        assert_eq!((c.load)(std::ptr::null()), (rust.load)(std::ptr::null()));
        assert_eq!((c.load)(std::ptr::null()), -1);
        assert_eq!((rust.load)(std::ptr::null()), -1);
        mark(&mut errors, 5);

        let oversized = c_string(&vec![b'a'; MAX_BUFFER_SIZE]);
        let (c_status, c_error) = capture_fd(2, || (c.load)(oversized.as_ptr()));
        let (rust_status, rust_error) = capture_fd(2, || (rust.load)(oversized.as_ptr()));
        assert_eq!((c_status, &c_error), (rust_status, &rust_error));
        assert_eq!(c_status, -1);
        mark(&mut errors, 6);

        let tokens = tokenize_case(&c, &rust, b"@", 23);
        assert_eq!(tokens[0].token_type, 11);
        mark(&mut errors, 7);

        (c.stats)(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        (rust.stats)(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        let mut c_lines = usize::MAX;
        let mut rust_lines = usize::MAX;
        (c.stats)(&mut c_lines, std::ptr::null_mut(), std::ptr::null_mut());
        (rust.stats)(&mut rust_lines, std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(c_lines, rust_lines);
        mark(&mut errors, 9);

        // Returned callback table, direct callback invocation, and normal init.
        let c_ops = (c.get_ops)();
        let rust_ops = (rust.get_ops)();
        assert!(c_ops.next_token.is_some() && rust_ops.next_token.is_some());
        assert!(c_ops.peek_token.is_some() && rust_ops.peek_token.is_some());
        assert!(c_ops.reset.is_some() && rust_ops.reset.is_some());
        assert!(c_ops.load_text.is_some() && rust_ops.load_text.is_some());
        assert!(c_ops.get_stats.is_some() && rust_ops.get_stats.is_some());
        let callback_text = c_string(b"callback");
        assert_eq!(
            c_ops.load_text.unwrap()(callback_text.as_ptr()),
            rust_ops.load_text.unwrap()(callback_text.as_ptr())
        );
        assert_token(
            1,
            c_ops.next_token.unwrap()(),
            rust_ops.next_token.unwrap()(),
        );
        c_ops.reset.unwrap()();
        rust_ops.reset.unwrap()();
        let mut c_callback_stats = (0, 0, 0);
        let mut rust_callback_stats = (0, 0, 0);
        c_ops.get_stats.unwrap()(
            &mut c_callback_stats.0,
            &mut c_callback_stats.1,
            &mut c_callback_stats.2,
        );
        rust_ops.get_stats.unwrap()(
            &mut rust_callback_stats.0,
            &mut rust_callback_stats.1,
            &mut rust_callback_stats.2,
        );
        assert_eq!(c_callback_stats, rust_callback_stats);
        mark(&mut configs, 1);

        (c.init)(c_ops);
        (rust.init)(rust_ops);
        mark(&mut configs, 31);

        let (c_result, c_error) = capture_fd(2, || (c.analyze)(std::ptr::null()));
        let (rust_result, rust_error) = capture_fd(2, || (rust.analyze)(std::ptr::null()));
        assert_eq!(c_result, rust_result);
        assert_eq!(c_error, rust_error);
        assert_eq!(c_result, AnalysisResult::default());
        mark(&mut errors, 8);

        let (_, c_output) = capture_fd(1, || (c.find)(std::ptr::null()));
        let (_, rust_output) = capture_fd(1, || (rust.find)(std::ptr::null()));
        assert_eq!(c_output, rust_output);
        assert!(c_output.is_empty());
        mark(&mut errors, 4);

        (c.init)(custom_ops(failing_load));
        (rust.init)(custom_ops(failing_load));
        let (c_result, c_error) = capture_fd(2, || (c.analyze)(empty.as_ptr()));
        let (rust_result, rust_error) = capture_fd(2, || (rust.analyze)(empty.as_ptr()));
        assert_eq!(c_result, rust_result);
        assert_eq!(c_error, rust_error);
        assert_eq!(c_result, AnalysisResult::default());
        mark(&mut errors, 2);

        (c.init)(custom_ops(custom_load));
        (rust.init)(custom_ops(custom_load));
        ENUM_POSITION.store(0, Ordering::SeqCst);
        let c_result = (c.analyze)(empty.as_ptr());
        ENUM_POSITION.store(0, Ordering::SeqCst);
        let rust_result = (rust.analyze)(empty.as_ptr());
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, AnalysisResult::default());
        mark(&mut errors, 10);

        (c.init)(c_ops);
        (rust.init)(rust_ops);
        let empty_distribution =
            output_both(1, 37, || (c.distribution)(), || (rust.distribution)());
        assert_eq!(
            empty_distribution,
            b"\n=== Token Distribution ===\n\n=== Most Common Words ===\n"
        );
        mark(&mut configs, 37);

        // Basic load boundaries and token dispatch.
        let tokens = tokenize_case(&c, &rust, b"", 2);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, 0);
        mark(&mut configs, 2);

        for _ in 0..32 {
            let byte = rng.byte_from(b"abcXYZ_019+-()'\"");
            load_both(&c, &rust, &[byte], 3);
        }
        mark(&mut configs, 3);

        load_both(&c, &rust, &vec![b'z'; MAX_BUFFER_SIZE - 1], 4);
        mark(&mut configs, 4);

        for _ in 0..32 {
            let mut input = Vec::new();
            for _ in 0..rng.range(1, 20) {
                input.push(rng.byte_from(b" \t\x0b\x0c\r"));
            }
            input.push(b'x');
            let tokens = tokenize_case(&c, &rust, &input, 5);
            assert_eq!(tokens[0].bytes(), b"x");
        }
        mark(&mut configs, 5);

        for count in 1..=16 {
            let input = vec![b'\n'; count];
            let tokens = tokenize_case(&c, &rust, &input, 6);
            assert_eq!(tokens.len(), count + 1);
        }
        mark(&mut configs, 6);

        for _ in 0..64 {
            let mut identifier = vec![rng.byte_from(b"abcdefghijklmnopqrstuvwxyz_")];
            for _ in 1..rng.range(2, 40) {
                identifier.push(
                    rng.byte_from(
                        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789",
                    ),
                );
            }
            let tokens = tokenize_case(&c, &rust, &identifier, 7);
            assert_eq!(tokens[0].token_type, 6);
        }
        mark(&mut configs, 7);

        const KEYWORDS: &[&[u8]] = &[
            b"if",
            b"else",
            b"while",
            b"for",
            b"return",
            b"int",
            b"char",
            b"float",
            b"double",
            b"void",
            b"struct",
            b"typedef",
            b"const",
            b"static",
            b"extern",
            b"auto",
            b"register",
            b"sizeof",
            b"break",
            b"continue",
            b"switch",
            b"case",
            b"default",
            b"do",
            b"goto",
            b"enum",
            b"union",
            b"signed",
            b"unsigned",
            b"long",
            b"short",
        ];
        for keyword in KEYWORDS {
            let tokens = tokenize_case(&c, &rust, keyword, 8);
            assert_eq!(tokens[0].token_type, 7);
        }
        mark(&mut configs, 8);

        for _ in 0..64 {
            let number: Vec<u8> = (0..rng.range(1, 50))
                .map(|_| rng.byte_from(b"0123456789"))
                .collect();
            assert_eq!(tokenize_case(&c, &rust, &number, 9)[0].token_type, 2);

            let mut decimal = number.clone();
            decimal.insert(rng.range(1, decimal.len() + 1), b'.');
            assert_eq!(tokenize_case(&c, &rust, &decimal, 10)[0].token_type, 2);

            let mut two_dots = decimal;
            two_dots.push(b'.');
            two_dots.push(rng.byte_from(b"0123456789"));
            let tokens = tokenize_case(&c, &rust, &two_dots, 11);
            assert_eq!(tokens[0].token_type, 2);
            assert_eq!(tokens[1].token_type, 3);
        }
        mark(&mut configs, 9);
        mark(&mut configs, 10);
        mark(&mut configs, 11);

        for quote in [b'"', b'\''] {
            for _ in 0..32 {
                let body: Vec<u8> = (0..rng.range(1, 40))
                    .map(|_| rng.byte_from(b"abc XYZ 012"))
                    .collect();
                let mut closed = vec![quote];
                closed.extend_from_slice(&body);
                closed.push(quote);
                assert_eq!(tokenize_case(&c, &rust, &closed, 12)[0].token_type, 9);

                let mut escaped = vec![quote, b'a', b'\\', rng.byte_from(b"nrt\\\"'")];
                escaped.extend_from_slice(b"tail");
                escaped.push(quote);
                assert_eq!(tokenize_case(&c, &rust, &escaped, 13)[0].token_type, 9);

                let mut eof = vec![quote];
                eof.extend_from_slice(&body);
                assert_eq!(tokenize_case(&c, &rust, &eof, 14)[0].token_type, 9);

                let mut newline = eof;
                newline.extend_from_slice(b"\nrest");
                let tokens = tokenize_case(&c, &rust, &newline, 15);
                assert_eq!(tokens[0].token_type, 9);
                assert_eq!(tokens[1].token_type, 5);
            }
        }
        mark(&mut configs, 12);
        mark(&mut configs, 13);
        mark(&mut configs, 14);
        mark(&mut configs, 15);

        for _ in 0..32 {
            let body: Vec<u8> = (0..rng.range(0, 80))
                .map(|_| rng.byte_from(b"abc 123+-"))
                .collect();
            let mut line = b"//".to_vec();
            line.extend_from_slice(&body);
            line.push(b'\n');
            assert_eq!(tokenize_case(&c, &rust, &line, 16)[0].token_type, 10);

            let mut block = b"/*".to_vec();
            block.extend_from_slice(&body);
            block.extend_from_slice(b"*/tail");
            assert_eq!(tokenize_case(&c, &rust, &block, 17)[0].token_type, 10);

            let mut unterminated = b"/*".to_vec();
            unterminated.extend_from_slice(&body);
            assert_eq!(
                tokenize_case(&c, &rust, &unterminated, 18)[0].token_type,
                10
            );

            let input = [b'/', rng.byte_from(b"abc123+-")];
            let tokens = tokenize_case(&c, &rust, &input, 19);
            assert_eq!(tokens[0].token_type, 10);
            assert_eq!(tokens[0].bytes(), b"/");
        }
        mark(&mut configs, 16);
        mark(&mut configs, 17);
        mark(&mut configs, 18);
        mark(&mut configs, 19);

        for &operator in b"+-*/%=<>!&|^~?:" {
            assert_eq!(
                tokenize_case(&c, &rust, &[operator], 20)[0].token_type,
                if operator == b'/' { 10 } else { 8 }
            );
        }
        mark(&mut configs, 20);

        for operator in [
            b"==".as_slice(),
            b"!=",
            b"<=",
            b">=",
            b"&&",
            b"||",
            b"++",
            b"--",
            b"->",
            b"<<",
            b">>",
        ] {
            let tokens = tokenize_case(&c, &rust, operator, 21);
            assert_eq!(tokens[0].bytes(), operator);
        }
        mark(&mut configs, 21);

        for _ in 0..64 {
            let first = rng.byte_from(b"+-%=<>!&|^~?:");
            let second = rng.byte_from(b"abc()09");
            let tokens = tokenize_case(&c, &rust, &[first, second], 22);
            assert_eq!(tokens[0].length, 1);
        }
        mark(&mut configs, 22);

        for &punctuation in b"(){}[];,." {
            let tokens = tokenize_case(&c, &rust, &[punctuation], 23);
            assert_eq!(tokens[0].token_type, 3);
        }
        mark(&mut configs, 23);

        for &seed in b"az09" {
            let input = vec![seed; 300];
            let tokens = tokenize_case(&c, &rust, &input, 24);
            assert_eq!(tokens[0].length, 255);
            assert!(tokens.len() >= 3);
        }
        mark(&mut configs, 24);

        for quote in [b'"', b'\''] {
            let mut input = vec![quote];
            input.extend(std::iter::repeat(b'x').take(300));
            input.push(quote);
            assert_eq!(tokenize_case(&c, &rust, &input, 25)[0].length, 254);
        }
        mark(&mut configs, 25);

        let mut long_line = b"//".to_vec();
        long_line.extend(std::iter::repeat(b'x').take(300));
        assert_eq!(tokenize_case(&c, &rust, &long_line, 26)[0].length, 255);
        let mut long_block = b"/*".to_vec();
        long_block.extend(std::iter::repeat(b'x').take(300));
        long_block.extend_from_slice(b"*/");
        assert_eq!(tokenize_case(&c, &rust, &long_block, 26)[0].length, 254);
        mark(&mut configs, 26);

        // Lookahead, reset, and cumulative stats.
        load_both(&c, &rust, b"peek second", 27);
        let c_first = (c.peek)();
        let rust_first = (rust.peek)();
        assert_token(27, c_first, rust_first);
        assert_token(27, (c.peek)(), (rust.peek)());
        assert_token(27, (c.next)(), (rust.next)());
        mark(&mut configs, 27);

        load_both(&c, &rust, b"first\nsecond", 28);
        assert_token(28, (c.peek)(), (rust.peek)());
        (c.reset)();
        (rust.reset)();
        assert_token(28, (c.peek)(), (rust.peek)());
        mark(&mut configs, 28);

        tokenize_case(&c, &rust, b"stats one\nstats two", 29);
        let mut c_stats = (0, 0, 0);
        let mut rust_stats = (0, 0, 0);
        (c.stats)(&mut c_stats.0, &mut c_stats.1, &mut c_stats.2);
        (rust.stats)(&mut rust_stats.0, &mut rust_stats.1, &mut rust_stats.2);
        assert_eq!(c_stats, rust_stats);
        mark(&mut configs, 29);

        (c.reset)();
        (rust.reset)();
        let mut c_after_reset = (0, 0, 0);
        let mut rust_after_reset = (0, 0, 0);
        (c.stats)(
            &mut c_after_reset.0,
            &mut c_after_reset.1,
            &mut c_after_reset.2,
        );
        (rust.stats)(
            &mut rust_after_reset.0,
            &mut rust_after_reset.1,
            &mut rust_after_reset.2,
        );
        assert_eq!(c_after_reset, c_stats);
        assert_eq!(rust_after_reset, rust_stats);
        mark(&mut configs, 30);

        // Analyzer aggregation and cumulative result statistics.
        (c.init)(c_ops);
        (rust.init)(rust_ops);
        analyze_both(&c, &rust, b"", 32);
        mark(&mut configs, 32);

        for _ in 0..64 {
            let mut input = Vec::new();
            for _ in 0..rng.range(3, 30) {
                let piece: &[u8] = match rng.range(0, 12) {
                    0 => b"name_1 ",
                    1 => KEYWORDS[rng.range(0, KEYWORDS.len())],
                    2 => b"123.45 ",
                    3 => b"== ",
                    4 => b"// comment\n",
                    5 => b"/* block */ ",
                    6 => b"\"string\" ",
                    7 => b"(;), ",
                    8 => b"\t\r ",
                    9 => b"\n",
                    10 => b"@ ",
                    _ => b"/ ",
                };
                input.extend_from_slice(piece);
            }
            analyze_both(&c, &rust, &input, 33);
        }
        mark(&mut configs, 33);

        let first = analyze_both(&c, &rust, b"one\n", 34);
        let second = analyze_both(&c, &rust, b"two\nthree", 34);
        assert!(second.line_count >= first.line_count);
        assert!(second.char_count >= first.char_count);
        mark(&mut configs, 34);

        (c.init)(c_ops);
        (rust.init)(rust_ops);
        for _ in 0..16 {
            analyze_both(&c, &rust, b"beta alpha beta gamma alpha", 35);
        }
        let distribution = output_both(1, 35, || (c.distribution)(), || (rust.distribution)());
        assert!(distribution.windows(4).any(|window| window == b"beta"));
        mark(&mut configs, 35);

        (c.init)(c_ops);
        (rust.init)(rust_ops);
        let mut many_words = Vec::new();
        for index in 0..120 {
            many_words.extend_from_slice(format!("word{index} ").as_bytes());
        }
        analyze_both(&c, &rust, &many_words, 36);
        let distribution = output_both(1, 36, || (c.distribution)(), || (rust.distribution)());
        assert_eq!(
            distribution
                .split(|&byte| byte == b'\n')
                .filter(|line| line.first().is_some_and(u8::is_ascii_digit))
                .count(),
            10
        );
        mark(&mut configs, 36);

        for punctuation_count in [9, 10, 21] {
            (c.init)(c_ops);
            (rust.init)(rust_ops);
            let mut input = b"if return + == ".to_vec();
            input.extend(std::iter::repeat(b'(').take(punctuation_count));
            analyze_both(&c, &rust, &input, 38);
            assert_eq!((c.complexity)(), (rust.complexity)());
        }
        mark(&mut configs, 38);

        (c.init)(c_ops);
        (rust.init)(rust_ops);
        analyze_both(&c, &rust, b"// one\n// two\n// three\n+", 39);
        assert_eq!((c.complexity)(), (rust.complexity)());
        assert_eq!((c.complexity)(), 0);
        mark(&mut configs, 39);

        (c.init)(c_ops);
        (rust.init)(rust_ops);
        analyze_both(&c, &rust, b"alpha alphabet beta\nalpha42 no_match", 40);
        for pattern in [b"zzz".as_slice(), b"alpha", b"ha"] {
            let pattern = c_string(pattern);
            output_both(
                1,
                40,
                || (c.find)(pattern.as_ptr()),
                || (rust.find)(pattern.as_ptr()),
            );
        }
        mark(&mut configs, 40);

        let empty_pattern = c_string(b"");
        let output = output_both(
            1,
            41,
            || (c.find)(empty_pattern.as_ptr()),
            || (rust.find)(empty_pattern.as_ptr()),
        );
        assert!(output.windows(6).any(|window| window == b"Found "));
        mark(&mut configs, 41);

        assert_eq!(
            configs,
            (1..=41).collect(),
            "not every CONFIGS.md row executed"
        );
        assert_eq!(
            errors,
            (1..=10).collect(),
            "not every ERRORS.md row executed"
        );
    }
}
