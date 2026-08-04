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
// main.c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tokenizer.h"
#include "analyzer.h"

#define MAX_INPUT_SIZE 4096

void print_menu(void) {
    printf("\n=== Text Analyzer ===\n");
    printf("1. Analyze text\n");
    printf("2. Load text from file\n");
    printf("3. Show token distribution\n");
    printf("4. Calculate complexity score\n");
    printf("5. Find pattern\n");
    printf("6. Interactive tokenizer\n");
    printf("7. Exit\n");
    printf("Choice: ");
}

void print_analysis_result(analysis_result_t result) {
    printf("\n=== Analysis Results ===\n");
    printf("Words/Identifiers: %zu\n", result.word_count);
    printf("Numbers: %zu\n", result.number_count);
    printf("Keywords: %zu\n", result.keyword_count);
    printf("Operators: %zu\n", result.operator_count);
    printf("Comments: %zu\n", result.comment_count);
    printf("Strings: %zu\n", result.string_count);
    printf("Lines: %zu\n", result.line_count);
    printf("Characters: %zu\n", result.char_count);
}

void interactive_tokenizer(tokenizer_ops_t ops) {
    printf("\nEnter text (empty line to stop):\n");
    
    char input[MAX_INPUT_SIZE] = "";
    char line[256];
    
    while (fgets(line, sizeof(line), stdin)) {
        if (line[0] == '\n') {
            break;
        }
        strncat(input, line, MAX_INPUT_SIZE - strlen(input) - 1);
    }
    
    if (ops.load_text(input) != 0) {
        printf("Failed to load text\n");
        return;
    }
    
    printf("\n=== Tokens ===\n");
    
    const char *token_type_names[] = {
        "EOF", "WORD", "NUMBER", "PUNCT", "SPACE",
        "NEWLINE", "IDENT", "KEYWORD", "OPERATOR", 
        "STRING", "COMMENT", "ERROR"
    };
    
    token_t token;
    int count = 0;
    
    while ((token = ops.next_token()).type != TOKEN_EOF) {
        printf("[%s] '%s' (L%d:C%d)\n",
               token_type_names[token.type],
               token.value,
               token.line,
               token.column);
        count++;
        
        if (count > 100) {
            printf("... (truncated, too many tokens)\n");
            break;
        }
    }
}

char* read_file(const char *filename) {
    FILE *file = fopen(filename, "r");
    if (!file) {
        fprintf(stderr, "Error: Could not open file '%s'\n", filename);
        return NULL;
    }
    
    fseek(file, 0, SEEK_END);
    long size = ftell(file);
    fseek(file, 0, SEEK_SET);
    
    if (size > MAX_BUFFER_SIZE) {
        fprintf(stderr, "Error: File too large\n");
        fclose(file);
        return NULL;
    }
    
    char *content = malloc(size + 1);
    if (!content) {
        fprintf(stderr, "Error: Memory allocation failed\n");
        fclose(file);
        return NULL;
    }
    
    size_t read_size = fread(content, 1, size, file);
    content[read_size] = '\0';
    
    fclose(file);
    return content;
}

int main(void) {
    // Get tokenizer operations (function pointers)
    tokenizer_ops_t ops = get_tokenizer_ops();
    
    // Initialize analyzer with function pointers
    analyzer_init(ops);
    
    printf("Text Analysis and Tokenization System\n");
    printf("This system demonstrates function pointers and static globals\n");
    
    char input[256];
    int choice;
    
    while (1) {
        print_menu();
        
        if (!fgets(input, sizeof(input), stdin)) {
            break;
        }
        
        if (sscanf(input, "%d", &choice) != 1) {
            printf("Invalid input\n");
            continue;
        }
        
        switch (choice) {
            case 1: {
                printf("Enter text to analyze (empty line to stop):\n");
                char text[MAX_INPUT_SIZE] = "";
                char line[256];
                
                while (fgets(line, sizeof(line), stdin)) {
                    if (line[0] == '\n') {
                        break;
                    }
                    strncat(text, line, MAX_INPUT_SIZE - strlen(text) - 1);
                }
                
                analysis_result_t result = analyze_text(text);
                print_analysis_result(result);
                break;
            }
            
            case 2: {
                printf("Enter filename: ");
                if (!fgets(input, sizeof(input), stdin)) {
                    break;
                }
                input[strcspn(input, "\n")] = 0;
                
                char *content = read_file(input);
                if (content) {
                    analysis_result_t result = analyze_text(content);
                    print_analysis_result(result);
                    free(content);
                }
                break;
            }
            
            case 3: {
                print_token_distribution();
                break;
            }
            
            case 4: {
                int score = calculate_complexity_score();
                printf("\nComplexity Score: %d\n", score);
                if (score < 10) {
                    printf("Complexity: Low\n");
                } else if (score < 50) {
                    printf("Complexity: Medium\n");
                } else {
                    printf("Complexity: High\n");
                }
                break;
            }
            
            case 5: {
                printf("Enter pattern to search: ");
                if (!fgets(input, sizeof(input), stdin)) {
                    break;
                }
                input[strcspn(input, "\n")] = 0;
                
                find_patterns(input);
                break;
            }
            
            case 6: {
                interactive_tokenizer(ops);
                break;
            }
            
            case 7: {
                printf("Goodbye!\n");
                return 0;
            }
            
            default:
                printf("Invalid choice\n");
                break;
        }
    }
    
    return 0;
}
