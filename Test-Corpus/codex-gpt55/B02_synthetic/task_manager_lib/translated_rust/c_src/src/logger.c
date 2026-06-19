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

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "logger.h"

static FILE *log_file = NULL;

int initialize_logger() {
    const char *log_file_env = getenv("LOG_FILE");
    const char *log_file_path = log_file_env ? log_file_env : "default.log";

    log_file = fopen(log_file_path, "a");
    if (!log_file) {
        fprintf(stderr, "Failed to open log file: %s\n", log_file_path);
        return -1;
    }

    log_info("Logger initialized.");
    return 0;
}

void log_info(const char *message) {
    if (log_file) {
        fprintf(log_file, "[INFO] %s\n", message);
    }
}

void log_warning(const char *message) {
    if (log_file) {
        fprintf(log_file, "[WARNING] %s\n", message);
    }
}

void log_error(const char *message) {
    if (log_file) {
        fprintf(log_file, "[ERROR] %s\n", message);
    }
}

void finalize_logger() {
    if (log_file) {
        log_info("Logger finalized.");
        fclose(log_file);
    }
}
