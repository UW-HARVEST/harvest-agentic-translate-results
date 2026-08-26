use std::fs::File;
use std::io::{self, Read, Write};

const MAX_TOKEN_LENGTH: usize = 256;
const MAX_BUFFER_SIZE: usize = 8192;
const MAX_INPUT_SIZE: usize = 4096;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
#[allow(dead_code)]
enum TokenType {
    Eof = 0,
    Word = 1,
    Number = 2,
    Punctuation = 3,
    Whitespace = 4,
    Newline = 5,
    Identifier = 6,
    Keyword = 7,
    Operator = 8,
    String = 9,
    Comment = 10,
    Error = 11,
}

#[derive(Clone)]
#[allow(dead_code)]
struct Token {
    token_type: TokenType,
    value: Vec<u8>,
    length: usize,
    line: i32,
    column: i32,
}

impl Default for Token {
    fn default() -> Self {
        Self {
            token_type: TokenType::Eof,
            value: Vec::new(),
            length: 0,
            line: 1,
            column: 1,
        }
    }
}

#[derive(Default)]
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

struct InputReader {
    data: Vec<u8>,
    pos: usize,
}

impl InputReader {
    fn new() -> Self {
        let mut data = Vec::new();
        let _ = io::stdin().read_to_end(&mut data);
        Self { data, pos: 0 }
    }

    fn fgets(&mut self, size: usize) -> Option<Vec<u8>> {
        if self.pos >= self.data.len() || size == 0 {
            return None;
        }
        let max = size.saturating_sub(1);
        if max == 0 {
            return Some(Vec::new());
        }
        let start = self.pos;
        let mut end = self.pos;
        while end < self.data.len() && end - start < max {
            let b = self.data[end];
            end += 1;
            if b == b'\n' {
                break;
            }
        }
        self.pos = end;
        Some(self.data[start..end].to_vec())
    }
}

struct Tokenizer {
    input_buffer: Vec<u8>,
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

impl Tokenizer {
    fn new() -> Self {
        Self {
            input_buffer: Vec::new(),
            buffer_length: 0,
            current_position: 0,
            current_line: 1,
            current_column: 1,
            total_tokens_processed: 0,
            total_lines_processed: 0,
            total_chars_processed: 0,
            lookahead_token: Token::default(),
            lookahead_valid: false,
        }
    }

    fn is_keyword(s: &[u8]) -> bool {
        const KEYWORDS: [&[u8]; 31] = [
            b"if", b"else", b"while", b"for", b"return", b"int", b"char", b"float",
            b"double", b"void", b"struct", b"typedef", b"const", b"static", b"extern",
            b"auto", b"register", b"sizeof", b"break", b"continue", b"switch", b"case",
            b"default", b"do", b"goto", b"enum", b"union", b"signed", b"unsigned", b"long",
            b"short",
        ];
        KEYWORDS.iter().any(|kw| *kw == s)
    }

    fn peek_char(&self) -> u8 {
        if self.current_position >= self.buffer_length {
            0
        } else {
            self.input_buffer[self.current_position]
        }
    }

    fn advance_char(&mut self) -> u8 {
        if self.current_position >= self.buffer_length {
            return 0;
        }
        let c = self.input_buffer[self.current_position];
        self.current_position += 1;
        self.total_chars_processed += 1;
        if c == b'\n' {
            self.current_line += 1;
            self.current_column = 1;
            self.total_lines_processed += 1;
        } else {
            self.current_column += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while self.peek_char() != 0 && is_space(self.peek_char()) && self.peek_char() != b'\n' {
            self.advance_char();
        }
    }

    fn create_token(&mut self, token_type: TokenType, value: &[u8], length: usize) -> Token {
        let token_length = if length < MAX_TOKEN_LENGTH {
            length
        } else {
            MAX_TOKEN_LENGTH - 1
        };
        let mut token_value = value[..token_length.min(value.len())].to_vec();
        if token_value.len() > token_length {
            token_value.truncate(token_length);
        }
        self.total_tokens_processed += 1;
        Token {
            token_type,
            value: token_value,
            length: token_length,
            line: self.current_line,
            column: self.current_column - token_length as i32,
        }
    }

    fn scan_word(&mut self) -> Token {
        let mut buffer = Vec::new();
        while self.peek_char() != 0
            && (is_alnum(self.peek_char()) || self.peek_char() == b'_')
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            let c = self.advance_char();
            buffer.push(c);
        }
        if Self::is_keyword(&buffer) {
            self.create_token(TokenType::Keyword, &buffer, buffer.len())
        } else {
            self.create_token(TokenType::Identifier, &buffer, buffer.len())
        }
    }

    fn scan_number(&mut self) -> Token {
        let mut buffer = Vec::new();
        let mut has_decimal = false;
        while self.peek_char() != 0
            && (is_digit(self.peek_char()) || self.peek_char() == b'.')
            && buffer.len() < MAX_TOKEN_LENGTH - 1
        {
            if self.peek_char() == b'.' {
                if has_decimal {
                    break;
                }
                has_decimal = true;
            }
            let c = self.advance_char();
            buffer.push(c);
        }
        self.create_token(TokenType::Number, &buffer, buffer.len())
    }

    fn scan_string(&mut self) -> Token {
        let mut buffer = Vec::new();
        let quote = self.advance_char();
        buffer.push(quote);
        while self.peek_char() != 0
            && self.peek_char() != quote
            && self.peek_char() != b'\n'
            && buffer.len() < MAX_TOKEN_LENGTH - 2
        {
            if self.peek_char() == b'\\' {
                let c = self.advance_char();
                buffer.push(c);
                if self.peek_char() != 0 {
                    let c = self.advance_char();
                    buffer.push(c);
                }
            } else {
                let c = self.advance_char();
                buffer.push(c);
            }
        }
        if self.peek_char() == quote {
            let c = self.advance_char();
            buffer.push(c);
        }
        self.create_token(TokenType::String, &buffer, buffer.len())
    }

    fn scan_comment(&mut self) -> Token {
        let mut buffer = Vec::new();
        let c = self.advance_char();
        buffer.push(c);
        if self.peek_char() == b'/' {
            let c = self.advance_char();
            buffer.push(c);
            while self.peek_char() != 0
                && self.peek_char() != b'\n'
                && buffer.len() < MAX_TOKEN_LENGTH - 1
            {
                let c = self.advance_char();
                buffer.push(c);
            }
        } else if self.peek_char() == b'*' {
            let c = self.advance_char();
            buffer.push(c);
            while self.peek_char() != 0 && buffer.len() < MAX_TOKEN_LENGTH - 2 {
                if self.peek_char() == b'*' {
                    let c = self.advance_char();
                    buffer.push(c);
                    if self.peek_char() == b'/' {
                        let c = self.advance_char();
                        buffer.push(c);
                        break;
                    }
                } else {
                    let c = self.advance_char();
                    buffer.push(c);
                }
            }
        }
        self.create_token(TokenType::Comment, &buffer, buffer.len())
    }

    fn scan_operator(&mut self) -> Token {
        let mut buffer = Vec::new();
        let c = self.peek_char();
        let first = self.advance_char();
        buffer.push(first);
        let next = self.peek_char();
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
            let c = self.advance_char();
            buffer.push(c);
        }
        self.create_token(TokenType::Operator, &buffer, buffer.len())
    }

    fn next_token(&mut self) -> Token {
        if self.lookahead_valid {
            self.lookahead_valid = false;
            return self.lookahead_token.clone();
        }
        self.skip_whitespace();
        if self.current_position >= self.buffer_length {
            return self.create_token(TokenType::Eof, b"", 0);
        }
        let c = self.peek_char();
        if c == b'\n' {
            let newline = [self.advance_char()];
            return self.create_token(TokenType::Newline, &newline, 1);
        }
        if is_alpha(c) || c == b'_' {
            return self.scan_word();
        }
        if is_digit(c) {
            return self.scan_number();
        }
        if c == b'"' || c == b'\'' {
            return self.scan_string();
        }
        if c == b'/' && (self.peek_char() == b'/' || self.peek_char() == b'*') {
            return self.scan_comment();
        }
        if b"+-*/%=<>!&|^~?:".contains(&c) {
            return self.scan_operator();
        }
        if b"(){}[];,.".contains(&c) {
            let punct = [self.advance_char()];
            return self.create_token(TokenType::Punctuation, &punct, 1);
        }
        let unknown = [self.advance_char()];
        self.create_token(TokenType::Error, &unknown, 1)
    }

    #[allow(dead_code)]
    fn peek_token(&mut self) -> Token {
        if !self.lookahead_valid {
            self.lookahead_token = self.next_token();
            self.lookahead_valid = true;
        }
        self.lookahead_token.clone()
    }

    fn reset(&mut self) {
        self.current_position = 0;
        self.current_line = 1;
        self.current_column = 1;
        self.lookahead_valid = false;
    }

    fn load_text(&mut self, text: &[u8], stderr: &mut dyn Write) -> i32 {
        let length = c_strlen(text);
        if length >= MAX_BUFFER_SIZE {
            let _ = stderr.write_all(b"Error: Input text too large\n");
            return -1;
        }
        self.input_buffer = text[..length].to_vec();
        if self.input_buffer.len() > MAX_BUFFER_SIZE - 1 {
            self.input_buffer.truncate(MAX_BUFFER_SIZE - 1);
        }
        self.buffer_length = length;
        self.reset();
        0
    }

    fn get_stats(&self) -> (usize, usize, usize) {
        (
            self.total_lines_processed,
            self.total_tokens_processed,
            self.total_chars_processed,
        )
    }
}

struct Analyzer {
    initialized: bool,
    token_type_counts: [i32; 20],
    common_words: Vec<Vec<u8>>,
    common_word_counts: Vec<i32>,
}

impl Analyzer {
    fn new() -> Self {
        let mut analyzer = Self {
            initialized: false,
            token_type_counts: [0; 20],
            common_words: Vec::new(),
            common_word_counts: Vec::new(),
        };
        analyzer.init();
        analyzer
    }

    fn init(&mut self) {
        self.initialized = true;
        self.token_type_counts = [0; 20];
        self.common_words.clear();
        self.common_word_counts.clear();
    }

    fn track_word(&mut self, word: &[u8]) {
        for i in 0..self.common_words.len() {
            if self.common_words[i] == word {
                self.common_word_counts[i] += 1;
                return;
            }
        }
        if self.common_words.len() < 100 {
            let mut stored = word.to_vec();
            if stored.len() > MAX_TOKEN_LENGTH - 1 {
                stored.truncate(MAX_TOKEN_LENGTH - 1);
            }
            self.common_words.push(stored);
            self.common_word_counts.push(1);
        }
    }

    fn analyze_text(
        &mut self,
        tokenizer: &mut Tokenizer,
        text: &[u8],
        stderr: &mut dyn Write,
    ) -> AnalysisResult {
        let mut result = AnalysisResult::default();
        if !self.initialized {
            let _ = stderr.write_all(b"Error: Analyzer not initialized\n");
            return result;
        }
        if tokenizer.load_text(text, stderr) != 0 {
            let _ = stderr.write_all(b"Error: Failed to load text\n");
            return result;
        }
        loop {
            let token = tokenizer.next_token();
            if token.token_type == TokenType::Eof {
                break;
            }
            self.token_type_counts[token.token_type as usize] += 1;
            match token.token_type {
                TokenType::Word | TokenType::Identifier => {
                    result.word_count += 1;
                    self.track_word(&token.value);
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
        let (lines, _, chars) = tokenizer.get_stats();
        result.line_count = lines;
        result.char_count = chars;
        result
    }

    fn print_token_distribution(&mut self, stdout: &mut dyn Write) {
        write_bytes(stdout, b"\n=== Token Distribution ===\n");
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
        for i in 0..12 {
            if self.token_type_counts[i] > 0 {
                write_bytes(stdout, token_names[i]);
                write_fmt(stdout, format_args!(": {}\n", self.token_type_counts[i]));
            }
        }
        write_bytes(stdout, b"\n=== Most Common Words ===\n");
        let n = self.common_words.len();
        for i in 0..n.saturating_sub(1) {
            for j in 0..(n - i - 1) {
                if self.common_word_counts[j] < self.common_word_counts[j + 1] {
                    self.common_word_counts.swap(j, j + 1);
                    self.common_words.swap(j, j + 1);
                }
            }
        }
        let limit = n.min(10);
        for i in 0..limit {
            write_fmt(stdout, format_args!("{}. ", i + 1));
            write_cstr(stdout, &self.common_words[i]);
            write_fmt(stdout, format_args!(": {} times\n", self.common_word_counts[i]));
        }
    }

    fn calculate_complexity_score(&self) -> i32 {
        let mut score = 0;
        score += self.token_type_counts[TokenType::Keyword as usize] * 2;
        score += self.token_type_counts[TokenType::Operator as usize];
        score += self.token_type_counts[TokenType::Punctuation as usize] / 10;
        score -= self.token_type_counts[TokenType::Comment as usize];
        if score < 0 {
            score = 0;
        }
        score
    }

    fn find_patterns(&self, tokenizer: &mut Tokenizer, pattern: &[u8], stdout: &mut dyn Write) {
        if !self.initialized {
            return;
        }
        write_bytes(stdout, b"\n=== Searching for pattern: '");
        write_cstr(stdout, pattern);
        write_bytes(stdout, b"' ===\n");
        tokenizer.reset();
        let mut count = 0;
        loop {
            let token = tokenizer.next_token();
            if token.token_type == TokenType::Eof {
                break;
            }
            if contains_subslice(&token.value, &pattern[..c_strlen(pattern)]) {
                write_fmt(stdout, format_args!("Line {}, Column {}: ", token.line, token.column));
                write_cstr(stdout, &token.value);
                write_bytes(stdout, b"\n");
                count += 1;
            }
        }
        write_fmt(stdout, format_args!("Found {} occurrences\n", count));
    }
}

fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn c_strlen(bytes: &[u8]) -> usize {
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
}

fn trim_newline_in_place(bytes: &mut Vec<u8>) {
    if let Some(pos) = bytes.iter().position(|&b| b == b'\n') {
        bytes.truncate(pos);
    }
}

fn parse_c_int(input: &[u8]) -> Option<i32> {
    let mut i = 0;
    while i < input.len() && is_space(input[i]) {
        i += 1;
    }
    let mut sign = 1i32;
    if i < input.len() && (input[i] == b'+' || input[i] == b'-') {
        if input[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    if i >= input.len() || !is_digit(input[i]) {
        return None;
    }
    let mut value = 0i32;
    while i < input.len() && is_digit(input[i]) {
        value = value.wrapping_mul(10).wrapping_add((input[i] - b'0') as i32);
        i += 1;
    }
    Some(value.wrapping_mul(sign))
}

fn read_multiline_input(reader: &mut InputReader) -> Vec<u8> {
    let mut input = Vec::new();
    while let Some(line) = reader.fgets(256) {
        if line.first() == Some(&b'\n') {
            break;
        }
        let current_len = c_strlen(&input);
        let n = MAX_INPUT_SIZE.saturating_sub(current_len).saturating_sub(1);
        let line_len = c_strlen(&line);
        let append_len = n.min(line_len);
        input.truncate(current_len);
        input.extend_from_slice(&line[..append_len]);
    }
    input
}

fn print_menu(stdout: &mut dyn Write) {
    write_bytes(stdout, b"\n=== Text Analyzer ===\n");
    write_bytes(stdout, b"1. Analyze text\n");
    write_bytes(stdout, b"2. Load text from file\n");
    write_bytes(stdout, b"3. Show token distribution\n");
    write_bytes(stdout, b"4. Calculate complexity score\n");
    write_bytes(stdout, b"5. Find pattern\n");
    write_bytes(stdout, b"6. Interactive tokenizer\n");
    write_bytes(stdout, b"7. Exit\n");
    write_bytes(stdout, b"Choice: ");
}

fn print_analysis_result(result: &AnalysisResult, stdout: &mut dyn Write) {
    write_bytes(stdout, b"\n=== Analysis Results ===\n");
    write_fmt(stdout, format_args!("Words/Identifiers: {}\n", result.word_count));
    write_fmt(stdout, format_args!("Numbers: {}\n", result.number_count));
    write_fmt(stdout, format_args!("Keywords: {}\n", result.keyword_count));
    write_fmt(stdout, format_args!("Operators: {}\n", result.operator_count));
    write_fmt(stdout, format_args!("Comments: {}\n", result.comment_count));
    write_fmt(stdout, format_args!("Strings: {}\n", result.string_count));
    write_fmt(stdout, format_args!("Lines: {}\n", result.line_count));
    write_fmt(stdout, format_args!("Characters: {}\n", result.char_count));
}

fn interactive_tokenizer(
    tokenizer: &mut Tokenizer,
    reader: &mut InputReader,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) {
    write_bytes(stdout, b"\nEnter text (empty line to stop):\n");
    let input = read_multiline_input(reader);
    if tokenizer.load_text(&input, stderr) != 0 {
        write_bytes(stdout, b"Failed to load text\n");
        return;
    }
    write_bytes(stdout, b"\n=== Tokens ===\n");
    let token_type_names: [&[u8]; 12] = [
        b"EOF", b"WORD", b"NUMBER", b"PUNCT", b"SPACE", b"NEWLINE", b"IDENT", b"KEYWORD",
        b"OPERATOR", b"STRING", b"COMMENT", b"ERROR",
    ];
    let mut count = 0;
    loop {
        let token = tokenizer.next_token();
        if token.token_type == TokenType::Eof {
            break;
        }
        write_bytes(stdout, b"[");
        write_bytes(stdout, token_type_names[token.token_type as usize]);
        write_bytes(stdout, b"] '");
        write_cstr(stdout, &token.value);
        write_fmt(stdout, format_args!("' (L{}:C{})\n", token.line, token.column));
        count += 1;
        if count > 100 {
            write_bytes(stdout, b"... (truncated, too many tokens)\n");
            break;
        }
    }
}

fn read_file_content(filename: &[u8], stdout_err: &mut dyn Write) -> Option<Vec<u8>> {
    let name_len = c_strlen(filename);
    let name = String::from_utf8_lossy(&filename[..name_len]).into_owned();
    let mut file = match File::open(&name) {
        Ok(file) => file,
        Err(_) => {
            write_bytes(stdout_err, b"Error: Could not open file '");
            write_cstr(stdout_err, filename);
            write_bytes(stdout_err, b"'\n");
            return None;
        }
    };
    let size = match file.metadata() {
        Ok(metadata) => metadata.len() as usize,
        Err(_) => 0,
    };
    if size > MAX_BUFFER_SIZE {
        write_bytes(stdout_err, b"Error: File too large\n");
        return None;
    }
    let mut content = Vec::with_capacity(size + 1);
    let _ = file.read_to_end(&mut content);
    Some(content)
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|window| window == needle)
}

fn write_bytes(out: &mut dyn Write, bytes: &[u8]) {
    let _ = out.write_all(bytes);
}

fn write_cstr(out: &mut dyn Write, bytes: &[u8]) {
    let len = c_strlen(bytes);
    let _ = out.write_all(&bytes[..len]);
}

fn write_fmt(out: &mut dyn Write, args: std::fmt::Arguments<'_>) {
    let _ = out.write_fmt(args);
}

fn main() {
    let mut reader = InputReader::new();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let mut tokenizer = Tokenizer::new();
    let mut analyzer = Analyzer::new();

    write_bytes(&mut stdout, b"Text Analysis and Tokenization System\n");
    write_bytes(
        &mut stdout,
        b"This system demonstrates function pointers and static globals\n",
    );

    loop {
        print_menu(&mut stdout);
        let input = match reader.fgets(256) {
            Some(input) => input,
            None => break,
        };
        let choice = match parse_c_int(&input) {
            Some(choice) => choice,
            None => {
                write_bytes(&mut stdout, b"Invalid input\n");
                continue;
            }
        };
        match choice {
            1 => {
                write_bytes(&mut stdout, b"Enter text to analyze (empty line to stop):\n");
                let text = read_multiline_input(&mut reader);
                let result = analyzer.analyze_text(&mut tokenizer, &text, &mut stderr);
                print_analysis_result(&result, &mut stdout);
            }
            2 => {
                write_bytes(&mut stdout, b"Enter filename: ");
                let mut input = match reader.fgets(256) {
                    Some(input) => input,
                    None => break,
                };
                trim_newline_in_place(&mut input);
                if let Some(content) = read_file_content(&input, &mut stderr) {
                    let result = analyzer.analyze_text(&mut tokenizer, &content, &mut stderr);
                    print_analysis_result(&result, &mut stdout);
                }
            }
            3 => analyzer.print_token_distribution(&mut stdout),
            4 => {
                let score = analyzer.calculate_complexity_score();
                write_fmt(&mut stdout, format_args!("\nComplexity Score: {}\n", score));
                if score < 10 {
                    write_bytes(&mut stdout, b"Complexity: Low\n");
                } else if score < 50 {
                    write_bytes(&mut stdout, b"Complexity: Medium\n");
                } else {
                    write_bytes(&mut stdout, b"Complexity: High\n");
                }
            }
            5 => {
                write_bytes(&mut stdout, b"Enter pattern to search: ");
                let mut input = match reader.fgets(256) {
                    Some(input) => input,
                    None => break,
                };
                trim_newline_in_place(&mut input);
                analyzer.find_patterns(&mut tokenizer, &input, &mut stdout);
            }
            6 => interactive_tokenizer(&mut tokenizer, &mut reader, &mut stdout, &mut stderr),
            7 => {
                write_bytes(&mut stdout, b"Goodbye!\n");
                return;
            }
            _ => write_bytes(&mut stdout, b"Invalid choice\n"),
        }
    }
}
