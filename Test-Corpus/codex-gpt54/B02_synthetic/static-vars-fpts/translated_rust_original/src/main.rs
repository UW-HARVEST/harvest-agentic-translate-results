use std::cell::RefCell;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};

const MAX_TOKEN_LENGTH: usize = 256;
const MAX_BUFFER_SIZE: usize = 8192;
const MAX_INPUT_SIZE: usize = 4096;

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum TokenType {
    Eof = 0,
    Word,
    Number,
    Punctuation,
    Whitespace,
    Newline,
    Identifier,
    Keyword,
    Operator,
    String,
    Comment,
    Error,
}

#[allow(dead_code)]
#[derive(Clone)]
struct Token {
    token_type: TokenType,
    value: Vec<u8>,
    length: usize,
    line: i32,
    column: i32,
}

impl Token {
    fn eof() -> Self {
        Self {
            token_type: TokenType::Eof,
            value: Vec::new(),
            length: 0,
            line: 1,
            column: 1,
        }
    }
}

#[derive(Clone, Copy, Default)]
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

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct TokenizerOps {
    next_token: fn() -> Token,
    peek_token: fn() -> Token,
    reset: fn(),
    load_text: fn(&[u8]) -> i32,
    get_stats: fn(&mut usize, &mut usize, &mut usize),
}

struct TokenizerState {
    input_buffer: [u8; MAX_BUFFER_SIZE],
    buffer_length: usize,
    current_position: usize,
    current_line: i32,
    current_column: i32,
    total_tokens_processed: usize,
    total_lines_processed: usize,
    total_chars_processed: usize,
    lookahead_token: Token,
    lookahead_valid: bool,
}

impl Default for TokenizerState {
    fn default() -> Self {
        Self {
            input_buffer: [0; MAX_BUFFER_SIZE],
            buffer_length: 0,
            current_position: 0,
            current_line: 1,
            current_column: 1,
            total_tokens_processed: 0,
            total_lines_processed: 0,
            total_chars_processed: 0,
            lookahead_token: Token::eof(),
            lookahead_valid: false,
        }
    }
}

thread_local! {
    static TOKENIZER_STATE: RefCell<TokenizerState> = RefCell::new(TokenizerState::default());
    static ANALYZER_STATE: RefCell<AnalyzerState> = RefCell::new(AnalyzerState::default());
}

const KEYWORDS: [&[u8]; 31] = [
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

fn stdout_write(bytes: &[u8]) {
    let _ = io::stdout().write_all(bytes);
}

fn stderr_write(bytes: &[u8]) {
    let _ = io::stderr().write_all(bytes);
}

fn stdout_print(s: &str) {
    stdout_write(s.as_bytes());
}

fn stderr_print(s: &str) {
    stderr_write(s.as_bytes());
}

fn first_nul_len(bytes: &[u8]) -> usize {
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
}

fn is_ascii_space_non_newline(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | 0x0b | 0x0c | b'\r')
}

fn is_ascii_alpha(b: u8) -> bool {
    b.is_ascii_alphabetic()
}

fn is_ascii_alnum(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

fn create_token(state: &mut TokenizerState, token_type: TokenType, value: &[u8], length: usize) -> Token {
    let token_length = if length < MAX_TOKEN_LENGTH {
        length
    } else {
        MAX_TOKEN_LENGTH - 1
    };
    state.total_tokens_processed += 1;
    Token {
        token_type,
        value: value[..token_length].to_vec(),
        length: token_length,
        line: state.current_line,
        column: state.current_column - token_length as i32,
    }
}

fn is_keyword(bytes: &[u8]) -> bool {
    KEYWORDS.iter().any(|kw| *kw == bytes)
}

fn peek_char(state: &TokenizerState) -> u8 {
    if state.current_position >= state.buffer_length {
        0
    } else {
        state.input_buffer[state.current_position]
    }
}

fn advance_char(state: &mut TokenizerState) -> u8 {
    if state.current_position >= state.buffer_length {
        return 0;
    }

    let c = state.input_buffer[state.current_position];
    state.current_position += 1;
    state.total_chars_processed += 1;

    if c == b'\n' {
        state.current_line += 1;
        state.current_column = 1;
        state.total_lines_processed += 1;
    } else {
        state.current_column += 1;
    }

    c
}

fn skip_whitespace(state: &mut TokenizerState) {
    while peek_char(state) != 0 && is_ascii_space_non_newline(peek_char(state)) {
        advance_char(state);
    }
}

fn scan_word(state: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    while peek_char(state) != 0
        && (is_ascii_alnum(peek_char(state)) || peek_char(state) == b'_')
        && buffer.len() < MAX_TOKEN_LENGTH - 1
    {
        buffer.push(advance_char(state));
    }
    if is_keyword(&buffer) {
        create_token(state, TokenType::Keyword, &buffer, buffer.len())
    } else {
        create_token(state, TokenType::Identifier, &buffer, buffer.len())
    }
}

fn scan_number(state: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    let mut has_decimal = false;
    while peek_char(state) != 0
        && (peek_char(state).is_ascii_digit() || peek_char(state) == b'.')
        && buffer.len() < MAX_TOKEN_LENGTH - 1
    {
        if peek_char(state) == b'.' {
            if has_decimal {
                break;
            }
            has_decimal = true;
        }
        buffer.push(advance_char(state));
    }
    create_token(state, TokenType::Number, &buffer, buffer.len())
}

fn scan_string(state: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    let quote = advance_char(state);
    buffer.push(quote);

    while peek_char(state) != 0
        && peek_char(state) != quote
        && peek_char(state) != b'\n'
        && buffer.len() < MAX_TOKEN_LENGTH - 2
    {
        if peek_char(state) == b'\\' {
            buffer.push(advance_char(state));
            if peek_char(state) != 0 {
                buffer.push(advance_char(state));
            }
        } else {
            buffer.push(advance_char(state));
        }
    }

    if peek_char(state) == quote {
        buffer.push(advance_char(state));
    }

    create_token(state, TokenType::String, &buffer, buffer.len())
}

fn scan_comment(state: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    buffer.push(advance_char(state));

    if peek_char(state) == b'/' {
        buffer.push(advance_char(state));
        while peek_char(state) != 0 && peek_char(state) != b'\n' && buffer.len() < MAX_TOKEN_LENGTH - 1 {
            buffer.push(advance_char(state));
        }
    } else if peek_char(state) == b'*' {
        buffer.push(advance_char(state));
        while peek_char(state) != 0 && buffer.len() < MAX_TOKEN_LENGTH - 2 {
            if peek_char(state) == b'*' {
                buffer.push(advance_char(state));
                if peek_char(state) == b'/' {
                    buffer.push(advance_char(state));
                    break;
                }
            } else {
                buffer.push(advance_char(state));
            }
        }
    }

    create_token(state, TokenType::Comment, &buffer, buffer.len())
}

fn scan_operator(state: &mut TokenizerState) -> Token {
    let mut buffer = Vec::with_capacity(MAX_TOKEN_LENGTH);
    let c = peek_char(state);
    buffer.push(advance_char(state));
    let next = peek_char(state);

    if (c == b'=' && next == b'=')
        || (c == b'!' && next == b'=')
        || (c == b'<' && next == b'=')
        || (c == b'>' && next == b'=')
        || (c == b'&' && next == b'&')
        || (c == b'|' && next == b'|')
        || (c == b'+' && next == b'+')
        || (c == b'-' && next == b'-')
        || (c == b'-' && next == b'>')
        || (c == b'<' && next == b'<')
        || (c == b'>' && next == b'>')
    {
        buffer.push(advance_char(state));
    }

    create_token(state, TokenType::Operator, &buffer, buffer.len())
}

fn tokenizer_next_token() -> Token {
    TOKENIZER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();

        if state.lookahead_valid {
            state.lookahead_valid = false;
            return state.lookahead_token.clone();
        }

        skip_whitespace(&mut state);

        if state.current_position >= state.buffer_length {
            return create_token(&mut state, TokenType::Eof, b"", 0);
        }

        let c = peek_char(&state);

        if c == b'\n' {
            let newline = [advance_char(&mut state), 0];
            return create_token(&mut state, TokenType::Newline, &newline[..1], 1);
        }

        if is_ascii_alpha(c) || c == b'_' {
            return scan_word(&mut state);
        }

        if c.is_ascii_digit() {
            return scan_number(&mut state);
        }

        if c == b'"' || c == b'\'' {
            return scan_string(&mut state);
        }

        if c == b'/' {
            return scan_comment(&mut state);
        }

        if b"+-*/%=<>!&|^~?:".contains(&c) {
            return scan_operator(&mut state);
        }

        if b"(){}[];,.".contains(&c) {
            let punct = [advance_char(&mut state), 0];
            return create_token(&mut state, TokenType::Punctuation, &punct[..1], 1);
        }

        let unknown = [advance_char(&mut state), 0];
        create_token(&mut state, TokenType::Error, &unknown[..1], 1)
    })
}

fn tokenizer_peek_token() -> Token {
    TOKENIZER_STATE.with(|state_cell| {
        if !state_cell.borrow().lookahead_valid {
            let token = tokenizer_next_token();
            let mut state = state_cell.borrow_mut();
            state.lookahead_token = token.clone();
            state.lookahead_valid = true;
            token
        } else {
            state_cell.borrow().lookahead_token.clone()
        }
    })
}

fn tokenizer_reset() {
    TOKENIZER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        state.current_position = 0;
        state.current_line = 1;
        state.current_column = 1;
        state.lookahead_valid = false;
    });
}

fn tokenizer_load_text(text: &[u8]) -> i32 {
    let length = first_nul_len(text);
    if length >= MAX_BUFFER_SIZE {
        stderr_print("Error: Input text too large\n");
        return -1;
    }

    TOKENIZER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        state.input_buffer.fill(0);
        state.input_buffer[..length].copy_from_slice(&text[..length]);
        state.buffer_length = length;
        state.current_position = 0;
        state.current_line = 1;
        state.current_column = 1;
        state.lookahead_valid = false;
    });

    0
}

fn tokenizer_get_stats(lines: &mut usize, tokens: &mut usize, chars: &mut usize) {
    TOKENIZER_STATE.with(|state_cell| {
        let state = state_cell.borrow();
        *lines = state.total_lines_processed;
        *tokens = state.total_tokens_processed;
        *chars = state.total_chars_processed;
    });
}

fn get_tokenizer_ops() -> TokenizerOps {
    TokenizerOps {
        next_token: tokenizer_next_token,
        peek_token: tokenizer_peek_token,
        reset: tokenizer_reset,
        load_text: tokenizer_load_text,
        get_stats: tokenizer_get_stats,
    }
}

#[derive(Default)]
struct AnalyzerState {
    tokenizer_ops: Option<TokenizerOps>,
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<Vec<u8>>,
    common_word_counts: Vec<i32>,
}

fn analyzer_init(ops: TokenizerOps) {
    ANALYZER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        state.tokenizer_ops = Some(ops);
        state.initialized = true;
        state.token_type_counts = [0; 20];
        state.common_word_counts.clear();
        state.common_words.clear();
    });
}

fn track_word(word: &[u8]) {
    ANALYZER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        for i in 0..state.common_words.len() {
            if state.common_words[i] == word {
                state.common_word_counts[i] += 1;
                return;
            }
        }

        if state.common_words.len() < 100 {
            let mut stored = word[..word.len().min(MAX_TOKEN_LENGTH - 1)].to_vec();
            stored.truncate(MAX_TOKEN_LENGTH - 1);
            state.common_words.push(stored);
            state.common_word_counts.push(1);
        }
    });
}

fn analyze_text(text: &[u8]) -> AnalysisResult {
    let ops = ANALYZER_STATE.with(|state_cell| {
        let state = state_cell.borrow();
        if !state.initialized {
            None
        } else {
            state.tokenizer_ops
        }
    });

    let Some(ops) = ops else {
        stderr_print("Error: Analyzer not initialized\n");
        return AnalysisResult::default();
    };

    if (ops.load_text)(text) != 0 {
        stderr_print("Error: Failed to load text\n");
        return AnalysisResult::default();
    }

    let mut result = AnalysisResult::default();

    loop {
        let token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }

        ANALYZER_STATE.with(|state_cell| {
            let mut state = state_cell.borrow_mut();
            state.token_type_counts[token.token_type as usize] += 1;
        });

        match token.token_type {
            TokenType::Word | TokenType::Identifier => {
                result.word_count += 1;
                track_word(&token.value);
            }
            TokenType::Number => result.number_count += 1,
            TokenType::Keyword => result.keyword_count += 1,
            TokenType::Operator => result.operator_count += 1,
            TokenType::Comment => result.comment_count += 1,
            TokenType::String => result.string_count += 1,
            TokenType::Newline => result.line_count += 1,
            _ => {}
        }
    }

    let mut lines = 0;
    let mut tokens = 0;
    let mut chars = 0;
    (ops.get_stats)(&mut lines, &mut tokens, &mut chars);
    result.line_count = lines;
    result.char_count = chars;
    result
}

fn print_token_distribution() {
    stdout_print("\n=== Token Distribution ===\n");

    let token_names: [&[u8]; 12] = [
        b"EOF",
        b"WORD",
        b"NUMBER",
        b"PUNCTUATION",
        b"WHITESPACE",
        b"NEWLINE",
        b"IDENTIFIER",
        b"KEYWORD",
        b"OPERATOR",
        b"STRING",
        b"COMMENT",
        b"ERROR",
    ];

    ANALYZER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();

        for (i, name) in token_names.iter().enumerate() {
            if state.token_type_counts[i] > 0 {
                stdout_write(name);
                stdout_print(": ");
                stdout_print(&state.token_type_counts[i].to_string());
                stdout_print("\n");
            }
        }

        stdout_print("\n=== Most Common Words ===\n");

        let len = state.common_words.len();
        for i in 0..len.saturating_sub(1) {
            for j in 0..(len - i - 1) {
                if state.common_word_counts[j] < state.common_word_counts[j + 1] {
                    state.common_word_counts.swap(j, j + 1);
                    state.common_words.swap(j, j + 1);
                }
            }
        }

        let limit = len.min(10);
        for i in 0..limit {
            stdout_print(&(i + 1).to_string());
            stdout_print(". ");
            stdout_write(&state.common_words[i]);
            stdout_print(": ");
            stdout_print(&state.common_word_counts[i].to_string());
            stdout_print(" times\n");
        }
    });
}

fn calculate_complexity_score() -> i32 {
    ANALYZER_STATE.with(|state_cell| {
        let state = state_cell.borrow();
        let mut score = 0;
        score += state.token_type_counts[TokenType::Keyword as usize] * 2;
        score += state.token_type_counts[TokenType::Operator as usize];
        score += state.token_type_counts[TokenType::Punctuation as usize] / 10;
        score -= state.token_type_counts[TokenType::Comment as usize];
        if score < 0 {
            0
        } else {
            score
        }
    })
}

fn find_patterns(pattern: &[u8]) {
    let ops = ANALYZER_STATE.with(|state_cell| {
        let state = state_cell.borrow();
        if !state.initialized {
            None
        } else {
            state.tokenizer_ops
        }
    });

    let Some(ops) = ops else {
        return;
    };

    stdout_print("\n=== Searching for pattern: '");
    stdout_write(pattern);
    stdout_print("' ===\n");

    (ops.reset)();

    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }
        if pattern.is_empty() || find_subslice(&token.value, pattern).is_some() {
            stdout_print("Line ");
            stdout_print(&token.line.to_string());
            stdout_print(", Column ");
            stdout_print(&token.column.to_string());
            stdout_print(": ");
            stdout_write(&token.value);
            stdout_print("\n");
            count += 1;
        }
    }

    stdout_print("Found ");
    stdout_print(&count.to_string());
    stdout_print(" occurrences\n");
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn print_menu() {
    stdout_print("\n=== Text Analyzer ===\n");
    stdout_print("1. Analyze text\n");
    stdout_print("2. Load text from file\n");
    stdout_print("3. Show token distribution\n");
    stdout_print("4. Calculate complexity score\n");
    stdout_print("5. Find pattern\n");
    stdout_print("6. Interactive tokenizer\n");
    stdout_print("7. Exit\n");
    stdout_print("Choice: ");
}

fn print_analysis_result(result: AnalysisResult) {
    stdout_print("\n=== Analysis Results ===\n");
    stdout_print("Words/Identifiers: ");
    stdout_print(&result.word_count.to_string());
    stdout_print("\nNumbers: ");
    stdout_print(&result.number_count.to_string());
    stdout_print("\nKeywords: ");
    stdout_print(&result.keyword_count.to_string());
    stdout_print("\nOperators: ");
    stdout_print(&result.operator_count.to_string());
    stdout_print("\nComments: ");
    stdout_print(&result.comment_count.to_string());
    stdout_print("\nStrings: ");
    stdout_print(&result.string_count.to_string());
    stdout_print("\nLines: ");
    stdout_print(&result.line_count.to_string());
    stdout_print("\nCharacters: ");
    stdout_print(&result.char_count.to_string());
    stdout_print("\n");
}

struct InputReader<R: Read> {
    reader: BufReader<R>,
}

impl<R: Read> InputReader<R> {
    fn new(inner: R) -> Self {
        Self {
            reader: BufReader::new(inner),
        }
    }

    fn fgets_like(&mut self, size: usize) -> io::Result<Option<Vec<u8>>> {
        if size == 0 {
            return Ok(Some(Vec::new()));
        }

        let mut out = Vec::new();
        let limit = size - 1;

        while out.len() < limit {
            let mut byte = [0u8; 1];
            match self.reader.read(&mut byte)? {
                0 => {
                    if out.is_empty() {
                        return Ok(None);
                    }
                    break;
                }
                _ => {
                    out.push(byte[0]);
                    if byte[0] == b'\n' {
                        break;
                    }
                }
            }
        }

        Ok(Some(out))
    }
}

fn strip_newline(bytes: &mut Vec<u8>) {
    if let Some(pos) = bytes.iter().position(|&b| b == b'\n') {
        bytes.truncate(pos);
    }
}

fn strncat_like(dest: &mut Vec<u8>, src: &[u8], capacity: usize) {
    let len = dest.len().min(first_nul_len(dest));
    dest.truncate(len);
    if capacity == 0 || len >= capacity - 1 {
        return;
    }
    let available = capacity - len - 1;
    let src_len = first_nul_len(src);
    let to_copy = available.min(src_len);
    dest.extend_from_slice(&src[..to_copy]);
}

fn parse_choice(input: &[u8]) -> Option<i32> {
    let s = &input[..first_nul_len(input)];
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    if i >= s.len() {
        return None;
    }

    let mut sign = 1i64;
    if s[i] == b'+' {
        i += 1;
    } else if s[i] == b'-' {
        sign = -1;
        i += 1;
    }

    let start = i;
    let mut value = 0i64;
    while i < s.len() && s[i].is_ascii_digit() {
        value = value.saturating_mul(10).saturating_add((s[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        None
    } else {
        Some((value * sign) as i32)
    }
}

fn read_file(filename: &[u8]) -> Option<Vec<u8>> {
    let path = String::from_utf8_lossy(&filename[..first_nul_len(filename)]).into_owned();
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            stderr_print("Error: Could not open file '");
            stderr_write(path.as_bytes());
            stderr_print("'\n");
            return None;
        }
    };

    let mut content = Vec::new();
    if file.read_to_end(&mut content).is_err() {
        stderr_print("Error: Could not open file '");
        stderr_write(path.as_bytes());
        stderr_print("'\n");
        return None;
    }

    if content.len() > MAX_BUFFER_SIZE {
        stderr_print("Error: File too large\n");
        return None;
    }

    Some(content)
}

fn interactive_tokenizer(reader: &mut InputReader<io::StdinLock<'_>>, ops: TokenizerOps) -> io::Result<()> {
    stdout_print("\nEnter text (empty line to stop):\n");

    let mut input = Vec::new();
    while let Some(line) = reader.fgets_like(256)? {
        if line.first() == Some(&b'\n') {
            break;
        }
        strncat_like(&mut input, &line, MAX_INPUT_SIZE);
    }

    if (ops.load_text)(&input) != 0 {
        stdout_print("Failed to load text\n");
        return Ok(());
    }

    stdout_print("\n=== Tokens ===\n");
    let token_type_names: [&[u8]; 12] = [
        b"EOF",
        b"WORD",
        b"NUMBER",
        b"PUNCT",
        b"SPACE",
        b"NEWLINE",
        b"IDENT",
        b"KEYWORD",
        b"OPERATOR",
        b"STRING",
        b"COMMENT",
        b"ERROR",
    ];

    let mut count = 0;
    loop {
        let token = (ops.next_token)();
        if token.token_type == TokenType::Eof {
            break;
        }
        stdout_print("[");
        stdout_write(token_type_names[token.token_type as usize]);
        stdout_print("] '");
        stdout_write(&token.value);
        stdout_print("' (L");
        stdout_print(&token.line.to_string());
        stdout_print(":C");
        stdout_print(&token.column.to_string());
        stdout_print(")\n");
        count += 1;
        if count > 100 {
            stdout_print("... (truncated, too many tokens)\n");
            break;
        }
    }

    Ok(())
}

fn main() {
    let ops = get_tokenizer_ops();
    analyzer_init(ops);

    stdout_print("Text Analysis and Tokenization System\n");
    stdout_print("This system demonstrates function pointers and static globals\n");

    let stdin = io::stdin();
    let mut reader = InputReader::new(stdin.lock());

    loop {
        print_menu();

        let Some(input) = reader.fgets_like(256).unwrap_or(None) else {
            break;
        };

        let Some(choice) = parse_choice(&input) else {
            stdout_print("Invalid input\n");
            continue;
        };

        match choice {
            1 => {
                stdout_print("Enter text to analyze (empty line to stop):\n");
                let mut text = Vec::new();
                while let Some(line) = reader.fgets_like(256).unwrap_or(None) {
                    if line.first() == Some(&b'\n') {
                        break;
                    }
                    strncat_like(&mut text, &line, MAX_INPUT_SIZE);
                }
                let result = analyze_text(&text);
                print_analysis_result(result);
            }
            2 => {
                stdout_print("Enter filename: ");
                let Some(mut input) = reader.fgets_like(256).unwrap_or(None) else {
                    break;
                };
                strip_newline(&mut input);
                if let Some(content) = read_file(&input) {
                    let result = analyze_text(&content);
                    print_analysis_result(result);
                }
            }
            3 => {
                print_token_distribution();
            }
            4 => {
                let score = calculate_complexity_score();
                stdout_print("\nComplexity Score: ");
                stdout_print(&score.to_string());
                stdout_print("\n");
                if score < 10 {
                    stdout_print("Complexity: Low\n");
                } else if score < 50 {
                    stdout_print("Complexity: Medium\n");
                } else {
                    stdout_print("Complexity: High\n");
                }
            }
            5 => {
                stdout_print("Enter pattern to search: ");
                let Some(mut input) = reader.fgets_like(256).unwrap_or(None) else {
                    break;
                };
                strip_newline(&mut input);
                find_patterns(&input);
            }
            6 => {
                let _ = interactive_tokenizer(&mut reader, ops);
            }
            7 => {
                stdout_print("Goodbye!\n");
                return;
            }
            _ => {
                stdout_print("Invalid choice\n");
            }
        }
    }
}
