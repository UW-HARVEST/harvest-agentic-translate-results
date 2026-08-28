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
#include <string.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

/* Forward declarations */
static int validate_token(const char *token, const char *expected);
static int parse_command(char *buffer, size_t buf_size, const char *cmd_list[], int list_size);
static int compare_prefix(const char *str, const char *prefix, int exact_match);
static int find_delimiter(const char *data, size_t len, char delim);
static int match_pattern(const char *text, const char *pattern, int case_sensitive);

/**
 * Main entrance function - performs various string comparison operations
 * 
 * @param input Input string buffer
 * @param input_len Length of input buffer
 * @param reference Reference string for comparison
 * @param ref_len Length of reference buffer
 * @param operation Operation code:
 *                  0: validate token
 *                  1: parse command
 *                  2: compare prefix
 *                  3: find delimiter
 *                  4: match pattern
 * @param flags Operation flags (case sensitivity, exact match, etc)
 * @return Operation result (match count, position, or error code)
 */
int process_strings(char *input, size_t input_len, 
                   const char *reference, size_t ref_len,
                   int operation, uint32_t flags) {
    
    if (input == NULL) {
        return -1;
    }
    
    /* Different operations based on operation code */
    switch (operation) {
        case 0: {
            /* Validate token - VULNERABLE if input not null-terminated */
            if (reference == NULL) return -2;
            return validate_token(input, reference);
        }
        
        case 1: {
            /* Parse command from list - checks against multiple strings */
            const char *commands[] = {"START", "STOP", "PAUSE", "RESUME", "RESET"};
            return parse_command(input, input_len, commands, 5);
        }
        
        case 2: {
            /* Compare prefix - can use strcmp or strncmp based on flags */
            if (reference == NULL) return -2;
            int exact = (flags & 0x01);
            return compare_prefix(input, reference, exact);
        }
        
        case 3: {
            /* Find delimiter position */
            char delim = (reference && ref_len > 0) ? reference[0] : ':';
            return find_delimiter(input, input_len, delim);
        }
        
        case 4: {
            /* Match pattern - VULNERABLE in certain paths */
            if (reference == NULL) return -2;
            int case_sens = (flags & 0x02);
            return match_pattern(input, reference, case_sens);
        }
        
        default:
            return -3;
    }
}

/**
 * Validate token against expected value
 * VULNERABLE: Uses strcmp without ensuring null termination
 */
static int validate_token(const char *token, const char *expected) {
    /* Direct strcmp - will overflow if token not null-terminated */
    if (strcmp(token, expected) == 0) {
        return 1;  /* Valid */
    }
    
    /* Also check some common variations */
    if (strcmp(token, "VALID") == 0 || strcmp(token, "OK") == 0) {
        return 1;
    }
    
    return 0;  /* Invalid */
}

/**
 * Parse command from a list of valid commands
 * Mix of safe and unsafe comparisons
 */
static int parse_command(char *buffer, size_t buf_size, 
                        const char *cmd_list[], int list_size) {
    
    /* Iterate through command list */
    for (int i = 0; i < list_size; i++) {
        /* Safe comparison using strncmp first */
        size_t cmd_len = strlen(cmd_list[i]);
        
        if (buf_size >= cmd_len) {
            if (strncmp(buffer, cmd_list[i], cmd_len) == 0) {
                /* Check if exact match - VULNERABLE if buffer not null-terminated */
                if (buffer[cmd_len] == '\0' || buffer[cmd_len] == ' ') {
                    return i;  /* Return command index */
                }
            }
        }
        
        /* Fallback: direct strcmp - VULNERABLE */
        if (strcmp(buffer, cmd_list[i]) == 0) {
            return i;
        }
    }
    
    /* Check for special admin command - always vulnerable strcmp */
    if (strcmp(buffer, "ADMIN") == 0) {
        return 99;
    }
    
    return -1;  /* No match */
}

/**
 * Compare prefix with optional exact matching
 * Safe when exact_match=0, vulnerable when exact_match=1
 */
static int compare_prefix(const char *str, const char *prefix, int exact_match) {
    size_t prefix_len = strlen(prefix);
    
    if (exact_match) {
        /* Exact match required - VULNERABLE strcmp */
        if (strcmp(str, prefix) == 0) {
            return 1;
        }
        
        /* Try with some common suffixes */
        char variations[5][32] = {"_v1", "_v2", "_old", "_new", "_tmp"};
        for (int i = 0; i < 5; i++) {
            /* Construct expected string */
            char expected[64];
            strncpy(expected, prefix, 63);
            expected[63] = '\0';
            strncat(expected, variations[i], 63 - strlen(expected));
            
            /* VULNERABLE: strcmp without length check on str */
            if (strcmp(str, expected) == 0) {
                return 2 + i;
            }
        }
        
        return 0;
    } else {
        /* Prefix match only - safer with strncmp */
        if (strncmp(str, prefix, prefix_len) == 0) {
            return 1;
        }
        return 0;
    }
}

/**
 * Find delimiter position in string
 * Uses strncmp for safety but has edge cases
 */
static int find_delimiter(const char *data, size_t len, char delim) {
    if (len == 0) return -1;
    
    /* Manual search with bounds checking */
    for (size_t i = 0; i < len; i++) {
        if (data[i] == delim) {
            return (int)i;
        }
        if (data[i] == '\0') {
            break;
        }
    }
    
    /* Check for special delimiter patterns using strcmp */
    /* VULNERABLE: doesn't verify data is null-terminated */
    if (delim == '|' && strcmp(data, "NONE") == 0) {
        return -2;  /* Special case */
    }
    
    if (delim == ':' && strcmp(data, "EMPTY") == 0) {
        return -3;  /* Special case */
    }
    
    return -1;  /* Not found */
}

/**
 * Match pattern with optional case sensitivity
 * Multiple vulnerable strcmp calls
 */
static int match_pattern(const char *text, const char *pattern, int case_sensitive) {
    
    if (case_sensitive) {
        /* Case-sensitive exact match - VULNERABLE */
        if (strcmp(text, pattern) == 0) {
            return 1;
        }
        
        /* Try with wildcards - construct patterns */
        char wildcard_patterns[3][64];
        snprintf(wildcard_patterns[0], 64, "*%s*", pattern);
        snprintf(wildcard_patterns[1], 64, "%s*", pattern);
        snprintf(wildcard_patterns[2], 64, "*%s", pattern);
        
        /* VULNERABLE: strcmp on unbounded text */
        for (int i = 0; i < 3; i++) {
            if (strcmp(text, wildcard_patterns[i]) == 0) {
                return 2 + i;
            }
        }
        
        /* Check if text contains pattern - uses strncmp safely */
        size_t text_len = strlen(text);
        size_t pattern_len = strlen(pattern);
        
        for (size_t i = 0; i <= text_len - pattern_len; i++) {
            if (strncmp(&text[i], pattern, pattern_len) == 0) {
                return 10 + i;  /* Return position + offset */
            }
        }
        
    } else {
        /* Case-insensitive - need to check both cases */
        /* First try exact match - VULNERABLE */
        if (strcmp(text, pattern) == 0) {
            return 1;
        }
        
        /* Manual case-insensitive comparison */
        size_t pattern_len = strlen(pattern);
        size_t text_len = strlen(text);
        
        if (text_len != pattern_len) {
            /* Try prefix match with strncmp */
            if (strncmp(text, pattern, pattern_len) == 0) {
                return 5;
            }
        }
        
        /* Compare character by character (safer) */
        if (text_len == pattern_len) {
            int match = 1;
            for (size_t i = 0; i < pattern_len; i++) {
                char c1 = text[i];
                char c2 = pattern[i];
                
                /* Convert to lowercase */
                if (c1 >= 'A' && c1 <= 'Z') c1 += 32;
                if (c2 >= 'A' && c2 <= 'Z') c2 += 32;
                
                if (c1 != c2) {
                    match = 0;
                    break;
                }
            }
            if (match) return 6;
        }
    }
    
    return 0;  /* No match */
}
