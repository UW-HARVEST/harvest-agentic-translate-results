use crate::logger::Logger;
use crate::task_manager::{TaskManagerState, create_task_manager, add_task, print_tasks, destroy_task_manager};

const EXIT_FAILURE: i32 = 1;

/// Driver function: takes the input bytes (representing a null-terminated C string) and processes
/// each line as a task. Returns 0 on success, EXIT_FAILURE on error.
pub fn driver(tasks: &[u8], logger: &mut Logger, manager_state: &mut TaskManagerState) -> i32 {
    let res = logger.initialize();
    if res != 0 {
        return EXIT_FAILURE;
    }

    if !create_task_manager(manager_state, logger) {
        return EXIT_FAILURE;
    }

    // In C: const char *start = tasks; while (*start != '\0') { ... }
    // We treat `tasks` as a null-terminated C string. Truncate at first '\0' byte if present.
    let end_of_string = tasks.iter().position(|&b| b == 0).unwrap_or(tasks.len());
    let tasks = &tasks[..end_of_string];

    let mut start: usize = 0;
    let mut priority: i32 = 1;
    while start < tasks.len() {
        // Find next '\n' or end of string
        let rel_end = tasks[start..].iter().position(|&b| b == b'\n');
        let end = match rel_end {
            Some(off) => start + off,
            None => tasks.len(),
        };

        // Extract current task slice
        let task_bytes = &tasks[start..end];

        add_task(manager_state, logger, task_bytes, priority);
        priority += 1;

        // Advance: if we found a '\n', skip past it; else jump to end-of-string
        start = if end < tasks.len() && tasks[end] == b'\n' {
            end + 1
        } else {
            end
        };
    }

    print_tasks(manager_state);

    destroy_task_manager(manager_state, logger);
    logger.finalize();

    0
}
