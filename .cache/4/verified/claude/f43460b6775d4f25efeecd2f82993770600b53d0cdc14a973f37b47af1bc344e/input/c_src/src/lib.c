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
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

/* Forward declarations */
static bool parse_bool(char c);
static int apply_permissions(bool read, bool write, bool execute);
static int evaluate_conditions(bool cond1, bool cond2, bool cond3, int logic_op);
static int configure_flags(bool *decisions, size_t count);
static int validate_sequence(char *sequence, size_t len);

/**
 * Main entrance function - processes boolean decisions
 * 
 * @param decision_string String of 'y'/'n' characters representing decisions
 * @param length Length of decision string
 * @param operation Operation to perform:
 *                  0: apply permissions (uses first 3 decisions)
 *                  1: evaluate conditions with logic (uses first 3 decisions)
 *                  2: configure flags (uses all decisions)
 *                  3: validate sequence (checks pattern)
 * @param param Operation-specific parameter (logic operator, mode, etc)
 * @return Operation result or error code
 */
int process_decisions(char *decision_string, size_t length, 
                     int operation, int param) {
    
    if (decision_string == NULL || length == 0) {
        return -1;
    }
    
    switch (operation) {
        case 0: {
            /* Apply permissions: read, write, execute */
            if (length < 3) return -2;
            
            bool read = parse_bool(decision_string[0]);
            bool write = parse_bool(decision_string[1]);
            bool execute = parse_bool(decision_string[2]);
            
            return apply_permissions(read, write, execute);
        }
        
        case 1: {
            /* Evaluate logical conditions */
            if (length < 3) return -2;
            
            bool cond1 = parse_bool(decision_string[0]);
            bool cond2 = parse_bool(decision_string[1]);
            bool cond3 = parse_bool(decision_string[2]);
            
            /* param determines logic operation: 0=AND, 1=OR, 2=XOR, 3=NAND */
            return evaluate_conditions(cond1, cond2, cond3, param);
        }
        
        case 2: {
            /* Configure flags from all decisions */
            bool decisions[32];
            size_t count = (length < 32) ? length : 32;
            
            for (size_t i = 0; i < count; i++) {
                decisions[i] = parse_bool(decision_string[i]);
            }
            
            return configure_flags(decisions, count);
        }
        
        case 3: {
            /* Validate decision sequence */
            return validate_sequence(decision_string, length);
        }
        
        default:
            return -3;
    }
}

/**
 * Parse character to boolean
 * 'y' or 'Y' -> true
 * 'n' or 'N' -> false
 * anything else -> false
 */
static bool parse_bool(char c) {
    if (c == 'y' || c == 'Y') {
        return true;
    } else if (c == 'n' || c == 'N') {
        return false;
    }
    return false;  /* Default to false for invalid input */
}

/**
 * Apply permissions based on read/write/execute flags
 * Complex branching based on boolean combinations
 */
static int apply_permissions(bool read, bool write, bool execute) {
    int permission_value = 0;
    
    /* Base permission calculation */
    if (read) {
        permission_value += 4;
    }
    if (write) {
        permission_value += 2;
    }
    if (execute) {
        permission_value += 1;
    }
    
    /* Complex decision tree based on combinations */
    if (read && write && execute) {
        /* Full permissions - special handling */
        return 100 + permission_value;  /* 107 */
    } else if (read && write) {
        /* Read/write but no execute */
        if (permission_value == 6) {
            return 50 + permission_value;  /* 56 */
        }
    } else if (read && execute) {
        /* Read/execute but no write */
        return 30 + permission_value;  /* 35 */
    } else if (write && execute) {
        /* Write/execute but no read - unusual case */
        return 20 + permission_value;  /* 23 */
    } else if (read) {
        /* Read only */
        return 10 + permission_value;  /* 14 */
    } else if (write) {
        /* Write only - dangerous */
        return -10;
    } else if (execute) {
        /* Execute only - very unusual */
        return -20;
    }
    
    /* No permissions */
    return 0;
}

/**
 * Evaluate three conditions with different logic operators
 */
static int evaluate_conditions(bool cond1, bool cond2, bool cond3, int logic_op) {
    bool result;
    
    switch (logic_op) {
        case 0: {
            /* AND - all must be true */
            result = cond1 && cond2 && cond3;
            
            if (result) {
                return 100;
            } else {
                /* Partial matches */
                if (cond1 && cond2) return 50;
                if (cond1 && cond3) return 51;
                if (cond2 && cond3) return 52;
                if (cond1) return 10;
                if (cond2) return 11;
                if (cond3) return 12;
                return 0;
            }
        }
        
        case 1: {
            /* OR - at least one must be true */
            result = cond1 || cond2 || cond3;
            
            if (result) {
                /* Count how many are true */
                int count = 0;
                if (cond1) count++;
                if (cond2) count++;
                if (cond3) count++;
                return 100 + count;
            }
            return 0;
        }
        
        case 2: {
            /* XOR - odd number must be true */
            result = (cond1 ^ cond2 ^ cond3);
            
            if (result) {
                /* Determine which combination */
                if (cond1 && !cond2 && !cond3) return 1;
                if (!cond1 && cond2 && !cond3) return 2;
                if (!cond1 && !cond2 && cond3) return 3;
                if (cond1 && cond2 && cond3) return 7;
                return 90;
            }
            return 0;
        }
        
        case 3: {
            /* NAND - not all true */
            result = !(cond1 && cond2 && cond3);
            
            if (result) {
                /* Various false combinations */
                if (!cond1 && !cond2 && !cond3) return 200;
                if (!cond1) return 150;
                if (!cond2) return 151;
                if (!cond3) return 152;
                return 100;
            }
            return 0;
        }
        
        default:
            return -1;
    }
}

/**
 * Configure system flags based on array of decisions
 * Creates bitmask and applies various rules
 */
static int configure_flags(bool *decisions, size_t count) {
    uint32_t flags = 0;
    int special_count = 0;
    
    /* Build flag bitmask */
    for (size_t i = 0; i < count && i < 32; i++) {
        if (decisions[i]) {
            flags |= (1u << i);
            special_count++;
        }
    }
    
    /* Apply rules based on flag patterns */
    if (special_count == 0) {
        /* All false */
        return 0;
    } else if (special_count == count) {
        /* All true */
        return 1000 + (int)count;
    } else if (special_count == 1) {
        /* Exactly one true - find which one */
        for (size_t i = 0; i < count; i++) {
            if (decisions[i]) {
                return 100 + (int)i;
            }
        }
    } else if (special_count == count - 1) {
        /* Exactly one false - find which one */
        for (size_t i = 0; i < count; i++) {
            if (!decisions[i]) {
                return 200 + (int)i;
            }
        }
    }
    
    /* Check for alternating pattern */
    bool alternating = true;
    for (size_t i = 1; i < count; i++) {
        if (decisions[i] == decisions[i-1]) {
            alternating = false;
            break;
        }
    }
    
    if (alternating) {
        return 500 + special_count;
    }
    
    /* Check for consecutive true values */
    int max_consecutive = 0;
    int current_consecutive = 0;
    for (size_t i = 0; i < count; i++) {
        if (decisions[i]) {
            current_consecutive++;
            if (current_consecutive > max_consecutive) {
                max_consecutive = current_consecutive;
            }
        } else {
            current_consecutive = 0;
        }
    }
    
    if (max_consecutive >= 3) {
        return 300 + max_consecutive;
    }
    
    /* Return count of true values */
    return special_count;
}

/**
 * Validate sequence has proper pattern
 * Checks various decision sequence rules
 */
static int validate_sequence(char *sequence, size_t len) {
    if (len == 0) return 0;
    
    /* Convert all to bools */
    bool *bools = (bool*)sequence;  /* Reuse buffer */
    for (size_t i = 0; i < len; i++) {
        bool val = parse_bool(sequence[i]);
        bools[i] = val;
    }
    
    /* Rule 1: Must start with 'y' */
    if (!bools[0]) {
        return -10;
    }
    
    /* Rule 2: Must end with 'n' if length > 1 */
    if (len > 1 && bools[len-1]) {
        return -11;
    }
    
    /* Rule 3: Cannot have more than 3 consecutive same values */
    int consecutive = 1;
    for (size_t i = 1; i < len; i++) {
        if (bools[i] == bools[i-1]) {
            consecutive++;
            if (consecutive > 3) {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }
    
    /* Rule 4: Count transitions */
    int transitions = 0;
    for (size_t i = 1; i < len; i++) {
        if (bools[i] != bools[i-1]) {
            transitions++;
        }
    }
    
    /* Validate based on length */
    if (len <= 3) {
        /* Short sequences - simple rules */
        if (transitions == 0) return 1;  /* All same (but passes other rules) */
        if (transitions == len - 1) return 2;  /* All different */
        return 10 + transitions;
    } else if (len <= 10) {
        /* Medium sequences */
        if (transitions < len / 3) return 20;  /* Few transitions */
        if (transitions > len / 2) return 30;  /* Many transitions */
        return 25;
    } else {
        /* Long sequences */
        if (transitions < 3) return 40;
        if (transitions > len - 3) return 50;
        return 45;
    }
}
