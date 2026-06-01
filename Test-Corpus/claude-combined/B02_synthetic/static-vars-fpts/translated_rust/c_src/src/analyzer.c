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
#include "analyzer.h"
#include <stdio.h>
#include <string.h>

// Static storage for tokenizer function pointers
static tokenizer_ops_t tokenizer_ops;
static int initialized = 0;

// Static arrays for tracking
static int token_type_counts[20];
static char common_words[100][MAX_TOKEN_LENGTH];
static int common_word_counts[100];
static int num_common_words = 0;

void analyzer_init(tokenizer_ops_t ops) {
    tokenizer_ops = ops;
    initialized = 1;
    
    // Reset tracking arrays
    memset(token_type_counts, 0, sizeof(token_type_counts));
    memset(common_word_counts, 0, sizeof(common_word_counts));
    num_common_words = 0;
}

static void track_word(const char *word) {
    // Find if word already exists
    for (int i = 0; i < num_common_words; i++) {
        if (strcmp(common_words[i], word) == 0) {
            common_word_counts[i]++;
            return;
        }
    }
    
    // Add new word
    if (num_common_words < 100) {
        strncpy(common_words[num_common_words], word, MAX_TOKEN_LENGTH - 1);
        common_words[num_common_words][MAX_TOKEN_LENGTH - 1] = '\0';
        common_word_counts[num_common_words] = 1;
        num_common_words++;
    }
}

analysis_result_t analyze_text(const char *text) {
    analysis_result_t result = {0};
    
    if (!initialized) {
        fprintf(stderr, "Error: Analyzer not initialized\n");
        return result;
    }
    
    // Load text using function pointer
    if (tokenizer_ops.load_text(text) != 0) {
        fprintf(stderr, "Error: Failed to load text\n");
        return result;
    }
    
    // Process all tokens using function pointers
    token_t token;
    while ((token = tokenizer_ops.next_token()).type != TOKEN_EOF) {
        // Update counts
        token_type_counts[token.type]++;
        
        switch (token.type) {
            case TOKEN_WORD:
            case TOKEN_IDENTIFIER:
                result.word_count++;
                track_word(token.value);
                break;
                
            case TOKEN_NUMBER:
                result.number_count++;
                break;
                
            case TOKEN_KEYWORD:
                result.keyword_count++;
                break;
                
            case TOKEN_OPERATOR:
                result.operator_count++;
                break;
                
            case TOKEN_COMMENT:
                result.comment_count++;
                break;
                
            case TOKEN_STRING:
                result.string_count++;
                break;
                
            case TOKEN_NEWLINE:
                result.line_count++;
                break;
                
            default:
                break;
        }
    }
    
    // Get final statistics using function pointer
    size_t lines, tokens, chars;
    tokenizer_ops.get_stats(&lines, &tokens, &chars);
    
    result.line_count = lines;
    result.char_count = chars;
    
    return result;
}

void print_token_distribution(void) {
    printf("\n=== Token Distribution ===\n");
    
    const char *token_names[] = {
        "EOF", "WORD", "NUMBER", "PUNCTUATION", "WHITESPACE",
        "NEWLINE", "IDENTIFIER", "KEYWORD", "OPERATOR", 
        "STRING", "COMMENT", "ERROR"
    };
    
    for (int i = 0; i < 12; i++) {
        if (token_type_counts[i] > 0) {
            printf("%s: %d\n", token_names[i], token_type_counts[i]);
        }
    }
    
    printf("\n=== Most Common Words ===\n");
    
    // Simple bubble sort for display
    for (int i = 0; i < num_common_words - 1; i++) {
        for (int j = 0; j < num_common_words - i - 1; j++) {
            if (common_word_counts[j] < common_word_counts[j + 1]) {
                // Swap counts
                int temp_count = common_word_counts[j];
                common_word_counts[j] = common_word_counts[j + 1];
                common_word_counts[j + 1] = temp_count;
                
                // Swap words
                char temp_word[MAX_TOKEN_LENGTH];
                strcpy(temp_word, common_words[j]);
                strcpy(common_words[j], common_words[j + 1]);
                strcpy(common_words[j + 1], temp_word);
            }
        }
    }
    
    // Print top 10
    int limit = num_common_words < 10 ? num_common_words : 10;
    for (int i = 0; i < limit; i++) {
        printf("%d. %s: %d times\n", i + 1, common_words[i], common_word_counts[i]);
    }
}

int calculate_complexity_score(void) {
    int score = 0;
    
    // Base score on keyword density
    score += token_type_counts[TOKEN_KEYWORD] * 2;
    
    // Add points for operators
    score += token_type_counts[TOKEN_OPERATOR];
    
    // Nesting indicators (braces)
    score += token_type_counts[TOKEN_PUNCTUATION] / 10;
    
    // Comments reduce complexity (good documentation)
    score -= token_type_counts[TOKEN_COMMENT];
    
    if (score < 0) score = 0;
    
    return score;
}

void find_patterns(const char *pattern) {
    if (!initialized || !pattern) {
        return;
    }
    
    printf("\n=== Searching for pattern: '%s' ===\n", pattern);
    
    // Reset tokenizer using function pointer
    tokenizer_ops.reset();
    
    int count = 0;
    token_t token;
    
    while ((token = tokenizer_ops.next_token()).type != TOKEN_EOF) {
        if (strstr(token.value, pattern) != NULL) {
            printf("Line %d, Column %d: %s\n", 
                   token.line, token.column, token.value);
            count++;
        }
    }
    
    printf("Found %d occurrences\n", count);
}
