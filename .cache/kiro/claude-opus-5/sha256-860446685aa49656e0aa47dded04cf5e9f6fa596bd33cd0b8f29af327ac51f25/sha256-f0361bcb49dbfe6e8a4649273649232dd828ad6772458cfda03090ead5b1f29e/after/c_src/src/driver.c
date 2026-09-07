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
#include "task_manager.h"
#include "logger.h"
#include <string.h>

int driver(const char *tasks) {
    int res = initialize_logger();
    if (res != 0) {
        return EXIT_FAILURE;
    }

    TaskManager *manager = create_task_manager();
    if (!manager) {
        return EXIT_FAILURE;
    }

    const char *start = tasks;
    int priority = 1;
    while (*start != '\0') {
        const char *end = strchr(start, '\n');
        if (end == NULL) {
            end = start + strlen(start);
        }

        // Extract the current task
        size_t length = end - start;
        char *task = (char *)malloc(length + 1);
        if (!task) {
            fprintf(stderr, "Error: Failed to allocate memory for task.\n");
            destroy_task_manager(manager);
            finalize_logger();
            return EXIT_FAILURE;
        }
        strncpy(task, start, length);
        task[length] = '\0';

        add_task(manager, task, priority++);
        free(task);
        start = (*end == '\n') ? end + 1 : end;
    }    

    print_tasks(manager);

    destroy_task_manager(manager);
    finalize_logger();

    return 0;
}
