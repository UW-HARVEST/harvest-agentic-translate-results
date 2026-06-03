/*
 * Rust translation of c_src/src/driver.c
 */

use crate::logger::{finalize_logger, initialize_logger};
use crate::task_manager::{add_task, create_task_manager, destroy_task_manager, print_tasks};

/// EXIT_FAILURE constant from <stdlib.h>.
const EXIT_FAILURE: i32 = 1;

/// Translated entry point. Splits `tasks` on newlines and adds each non-final
/// segment as a task with an incrementing priority, then prints and tears down.
pub fn driver(tasks: &str) -> i32 {
    let res = initialize_logger();
    if res != 0 {
        return EXIT_FAILURE;
    }

    let mut manager = match create_task_manager() {
        Some(m) => m,
        None => {
            return EXIT_FAILURE;
        }
    };

    // Mirror the C loop: iterate from the start of the string, splitting on '\n'.
    // The C version does not produce a final empty task when the string ends
    // immediately after a '\n' (because it advances past the terminator and the
    // outer `while (*start != '\0')` exits). We reproduce that by walking
    // manually instead of using `split('\n')`.
    let bytes = tasks.as_bytes();
    let mut start: usize = 0;
    let mut priority: i32 = 1;
    while start < bytes.len() {
        let rel_end = bytes[start..].iter().position(|&b| b == b'\n');
        let end = match rel_end {
            Some(off) => start + off,
            None => bytes.len(),
        };

        let segment = match std::str::from_utf8(&bytes[start..end]) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Error: Failed to allocate memory for task.");
                destroy_task_manager(manager);
                finalize_logger();
                return EXIT_FAILURE;
            }
        };

        add_task(&mut manager, segment, priority);
        priority += 1;

        // Advance: skip the '\n' if there was one, otherwise we're at end and the loop exits.
        start = if end < bytes.len() && bytes[end] == b'\n' {
            end + 1
        } else {
            end
        };
    }

    print_tasks(&manager);

    destroy_task_manager(manager);
    finalize_logger();

    0
}
