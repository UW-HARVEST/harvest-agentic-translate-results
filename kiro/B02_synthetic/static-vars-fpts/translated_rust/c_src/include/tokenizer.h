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
#ifndef TOKENIZER_H
#define TOKENIZER_H

#include <stddef.h>

#define MAX_TOKEN_LENGTH 256
#define MAX_BUFFER_SIZE 8192

typedef enum {
    TOKEN_EOF = 0,
    TOKEN_WORD,
    TOKEN_NUMBER,
    TOKEN_PUNCTUATION,
    TOKEN_WHITESPACE,
    TOKEN_NEWLINE,
    TOKEN_IDENTIFIER,
    TOKEN_KEYWORD,
    TOKEN_OPERATOR,
    TOKEN_STRING,
    TOKEN_COMMENT,
    TOKEN_ERROR
} token_type_t;

typedef struct {
    token_type_t type;
    char value[MAX_TOKEN_LENGTH];
    size_t length;
    int line;
    int column;
} token_t;

// Function pointer types for tokenizer operations
typedef token_t (*tokenizer_next_fn)(void);
typedef token_t (*tokenizer_peek_fn)(void);
typedef void (*tokenizer_reset_fn)(void);
typedef int (*tokenizer_load_fn)(const char *text);
typedef void (*tokenizer_get_stats_fn)(size_t *lines, size_t *tokens, size_t *chars);

// Tokenizer operations structure
typedef struct {
    tokenizer_next_fn next_token;
    tokenizer_peek_fn peek_token;
    tokenizer_reset_fn reset;
    tokenizer_load_fn load_text;
    tokenizer_get_stats_fn get_stats;
} tokenizer_ops_t;

// Get tokenizer operations (returns function pointers)
tokenizer_ops_t get_tokenizer_ops(void);

// Individual functions (can also be called directly)
token_t tokenizer_next_token(void);
token_t tokenizer_peek_token(void);
void tokenizer_reset(void);
int tokenizer_load_text(const char *text);
void tokenizer_get_stats(size_t *lines, size_t *tokens, size_t *chars);

#endif // TOKENIZER_H
