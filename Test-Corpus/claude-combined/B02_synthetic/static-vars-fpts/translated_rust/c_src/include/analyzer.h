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
#ifndef ANALYZER_H
#define ANALYZER_H

#include "tokenizer.h"

typedef struct {
    size_t word_count;
    size_t number_count;
    size_t keyword_count;
    size_t operator_count;
    size_t comment_count;
    size_t string_count;
    size_t line_count;
    size_t char_count;
} analysis_result_t;

// Initialize analyzer with tokenizer operations
void analyzer_init(tokenizer_ops_t ops);

// Perform analysis using function pointers
analysis_result_t analyze_text(const char *text);

// Get detailed token statistics
void print_token_distribution(void);

// Analyze code complexity
int calculate_complexity_score(void);

// Find patterns in the text
void find_patterns(const char *pattern);

#endif // ANALYZER_H
