/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 * 
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 * 
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
#include "tokenizer.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

// Static global variables (file-local scope)
static char input_buffer[MAX_BUFFER_SIZE];
static size_t buffer_length = 0;
static size_t current_position = 0;
static int current_line = 1;
static int current_column = 1;
static size_t total_tokens_processed = 0;
static size_t total_lines_processed = 0;
static size_t total_chars_processed = 0;
static token_t lookahead_token;
static int lookahead_valid = 0;

// Static keywords for recognition
static const char *keywords[] = {
    "if", "else", "while", "for", "return", "int", "char", 
    "float", "double", "void", "struct", "typedef", "const",
    "static", "extern", "auto", "register", "sizeof", "break",
    "continue", "switch", "case", "default", "do", "goto",
    "enum", "union", "signed", "unsigned", "long", "short"
};
static const int num_keywords = sizeof(keywords) / sizeof(keywords[0]);

// Static helper functions (internal to this file)
static int is_keyword(const char *str) {
    for (int i = 0; i < num_keywords; i++) {
        if (strcmp(str, keywords[i]) == 0) {
            return 1;
        }
    }
    return 0;
}

static char peek_char(void) {
    if (current_position >= buffer_length) {
        return '\0';
    }
    return input_buffer[current_position];
}

static char advance_char(void) {
    if (current_position >= buffer_length) {
        return '\0';
    }
    
    char c = input_buffer[current_position++];
    total_chars_processed++;
    
    if (c == '\n') {
        current_line++;
        current_column = 1;
        total_lines_processed++;
    } else {
        current_column++;
    }
    
    return c;
}

static void skip_whitespace(void) {
    while (peek_char() != '\0' && isspace(peek_char()) && peek_char() != '\n') {
        advance_char();
    }
}

static token_t create_token(token_type_t type, const char *value, size_t length) {
    token_t token;
    token.type = type;
    token.length = length < MAX_TOKEN_LENGTH ? length : MAX_TOKEN_LENGTH - 1;
    strncpy(token.value, value, token.length);
    token.value[token.length] = '\0';
    token.line = current_line;
    token.column = current_column - token.length;
    total_tokens_processed++;
    return token;
}

static token_t scan_word(void) {
    char buffer[MAX_TOKEN_LENGTH];
    size_t length = 0;
    
    while (peek_char() != '\0' && 
           (isalnum(peek_char()) || peek_char() == '_') && 
           length < MAX_TOKEN_LENGTH - 1) {
        buffer[length++] = advance_char();
    }
    
    buffer[length] = '\0';
    
    // Check if it's a keyword
    if (is_keyword(buffer)) {
        return create_token(TOKEN_KEYWORD, buffer, length);
    }
    
    return create_token(TOKEN_IDENTIFIER, buffer, length);
}

static token_t scan_number(void) {
    char buffer[MAX_TOKEN_LENGTH];
    size_t length = 0;
    int has_decimal = 0;
    
    while (peek_char() != '\0' && 
           (isdigit(peek_char()) || peek_char() == '.') && 
           length < MAX_TOKEN_LENGTH - 1) {
        
        if (peek_char() == '.') {
            if (has_decimal) {
                break; // Second decimal point
            }
            has_decimal = 1;
        }
        
        buffer[length++] = advance_char();
    }
    
    buffer[length] = '\0';
    return create_token(TOKEN_NUMBER, buffer, length);
}

static token_t scan_string(void) {
    char buffer[MAX_TOKEN_LENGTH];
    size_t length = 0;
    char quote = advance_char(); // Consume opening quote
    
    buffer[length++] = quote;
    
    while (peek_char() != '\0' && 
           peek_char() != quote && 
           peek_char() != '\n' &&
           length < MAX_TOKEN_LENGTH - 2) {
        
        if (peek_char() == '\\') {
            buffer[length++] = advance_char(); // Escape character
            if (peek_char() != '\0') {
                buffer[length++] = advance_char(); // Escaped character
            }
        } else {
            buffer[length++] = advance_char();
        }
    }
    
    if (peek_char() == quote) {
        buffer[length++] = advance_char(); // Closing quote
    }
    
    buffer[length] = '\0';
    return create_token(TOKEN_STRING, buffer, length);
}

static token_t scan_comment(void) {
    char buffer[MAX_TOKEN_LENGTH];
    size_t length = 0;
    
    // Assume we've seen '/'
    buffer[length++] = advance_char(); // First '/'
    
    if (peek_char() == '/') {
        // Single-line comment
        buffer[length++] = advance_char(); // Second '/'
        
        while (peek_char() != '\0' && 
               peek_char() != '\n' && 
               length < MAX_TOKEN_LENGTH - 1) {
            buffer[length++] = advance_char();
        }
    } else if (peek_char() == '*') {
        // Multi-line comment
        buffer[length++] = advance_char(); // '*'
        
        while (peek_char() != '\0' && length < MAX_TOKEN_LENGTH - 2) {
            if (peek_char() == '*') {
                buffer[length++] = advance_char();
                if (peek_char() == '/') {
                    buffer[length++] = advance_char();
                    break;
                }
            } else {
                buffer[length++] = advance_char();
            }
        }
    }
    
    buffer[length] = '\0';
    return create_token(TOKEN_COMMENT, buffer, length);
}

static token_t scan_operator(void) {
    char buffer[MAX_TOKEN_LENGTH];
    size_t length = 0;
    char c = peek_char();
    
    buffer[length++] = advance_char();
    
    // Check for two-character operators
    char next = peek_char();
    if ((c == '=' && next == '=') ||
        (c == '!' && next == '=') ||
        (c == '<' && next == '=') ||
        (c == '>' && next == '=') ||
        (c == '&' && next == '&') ||
        (c == '|' && next == '|') ||
        (c == '+' && next == '+') ||
        (c == '-' && next == '-') ||
        (c == '-' && next == '>') ||
        (c == '<' && next == '<') ||
        (c == '>' && next == '>')) {
        buffer[length++] = advance_char();
    }
    
    buffer[length] = '\0';
    return create_token(TOKEN_OPERATOR, buffer, length);
}

// Public functions that use static globals
token_t tokenizer_next_token(void) {
    // Check if we have a lookahead token
    if (lookahead_valid) {
        lookahead_valid = 0;
        return lookahead_token;
    }
    
    skip_whitespace();
    
    if (current_position >= buffer_length) {
        return create_token(TOKEN_EOF, "", 0);
    }
    
    char c = peek_char();
    
    // Newline
    if (c == '\n') {
        char newline[2] = {advance_char(), '\0'};
        return create_token(TOKEN_NEWLINE, newline, 1);
    }
    
    // Identifier or keyword
    if (isalpha(c) || c == '_') {
        return scan_word();
    }
    
    // Number
    if (isdigit(c)) {
        return scan_number();
    }
    
    // String
    if (c == '"' || c == '\'') {
        return scan_string();
    }
    
    // Comment
    if (c == '/' && (peek_char() == '/' || peek_char() == '*')) {
        return scan_comment();
    }
    
    // Operator
    if (strchr("+-*/%=<>!&|^~?:", c) != NULL) {
        return scan_operator();
    }
    
    // Punctuation
    if (strchr("(){}[];,.", c) != NULL) {
        char punct[2] = {advance_char(), '\0'};
        return create_token(TOKEN_PUNCTUATION, punct, 1);
    }
    
    // Unknown character
    char unknown[2] = {advance_char(), '\0'};
    return create_token(TOKEN_ERROR, unknown, 1);
}

token_t tokenizer_peek_token(void) {
    if (!lookahead_valid) {
        lookahead_token = tokenizer_next_token();
        lookahead_valid = 1;
    }
    return lookahead_token;
}

void tokenizer_reset(void) {
    current_position = 0;
    current_line = 1;
    current_column = 1;
    lookahead_valid = 0;
    // Note: We don't reset total statistics
}

int tokenizer_load_text(const char *text) {
    if (!text) {
        return -1;
    }
    
    size_t length = strlen(text);
    if (length >= MAX_BUFFER_SIZE) {
        fprintf(stderr, "Error: Input text too large\n");
        return -1;
    }
    
    strncpy(input_buffer, text, MAX_BUFFER_SIZE - 1);
    input_buffer[MAX_BUFFER_SIZE - 1] = '\0';
    buffer_length = length;
    
    tokenizer_reset();
    
    return 0;
}

void tokenizer_get_stats(size_t *lines, size_t *tokens, size_t *chars) {
    if (lines) *lines = total_lines_processed;
    if (tokens) *tokens = total_tokens_processed;
    if (chars) *chars = total_chars_processed;
}

// Function that returns function pointers
tokenizer_ops_t get_tokenizer_ops(void) {
    tokenizer_ops_t ops;
    ops.next_token = tokenizer_next_token;
    ops.peek_token = tokenizer_peek_token;
    ops.reset = tokenizer_reset;
    ops.load_text = tokenizer_load_text;
    ops.get_stats = tokenizer_get_stats;
    return ops;
}
