/*
 * Rust translation of c_src/src/task_manager.c
 */

use crate::logger::{log_error, log_info, log_warning};

pub const TASK_DESCRIPTION_LEN: usize = 256;

/// A single task. Mirrors the C `Task` struct (a 256-byte description + priority).
#[derive(Clone)]
pub struct Task {
    pub description: [u8; TASK_DESCRIPTION_LEN],
    pub priority: i32,
}

impl Task {
    fn new() -> Self {
        Task {
            description: [0u8; TASK_DESCRIPTION_LEN],
            priority: 0,
        }
    }

    /// Returns the description as a `&str`, stopping at the first NUL byte.
    pub fn description_str(&self) -> &str {
        let end = self
            .description
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(TASK_DESCRIPTION_LEN);
        std::str::from_utf8(&self.description[..end]).unwrap_or("")
    }
}

/// Mirror of the C `TaskManager` struct.
pub struct TaskManager {
    pub tasks: Vec<Task>,
    pub max_tasks: i32,
    pub task_count: i32,
}

/// Equivalent to the C `create_task_manager()`.
///
/// Returns `None` if allocation conceptually fails (Rust will panic on real OOM,
/// so this is mainly for parity with the C API).
pub fn create_task_manager() -> Option<Box<TaskManager>> {
    let max_tasks = std::env::var("MAX_TASKS")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(10);

    if max_tasks < 0 {
        log_error("Failed to allocate memory for tasks.");
        return None;
    }

    let mut tasks = Vec::with_capacity(max_tasks as usize);
    for _ in 0..max_tasks {
        tasks.push(Task::new());
    }

    let manager = Box::new(TaskManager {
        tasks,
        max_tasks,
        task_count: 0,
    });

    log_info("TaskManager created successfully.");
    Some(manager)
}

/// Equivalent to the C `add_task()`.
pub fn add_task(manager: &mut TaskManager, description: &str, priority: i32) {
    if manager.task_count >= manager.max_tasks {
        log_warning("Cannot add task: Maximum task limit reached.");
        return;
    }

    let idx = manager.task_count as usize;
    let task = &mut manager.tasks[idx];

    // Copy description bytes, leaving room for the trailing NUL (mirrors strncpy(..., 255)).
    let bytes = description.as_bytes();
    let copy_len = bytes.len().min(TASK_DESCRIPTION_LEN - 1);
    task.description[..copy_len].copy_from_slice(&bytes[..copy_len]);
    task.description[copy_len] = 0;
    // Zero out anything past the NUL terminator for cleanliness.
    for b in task.description[copy_len + 1..].iter_mut() {
        *b = 0;
    }

    task.priority = priority;
    manager.task_count += 1;

    log_info("Task added successfully.");
}

/// Equivalent to the C `print_tasks()`.
pub fn print_tasks(manager: &TaskManager) {
    println!("Tasks:");
    for i in 0..manager.task_count {
        let task = &manager.tasks[i as usize];
        println!(
            "  [{}] {} (Priority: {})",
            i + 1,
            task.description_str(),
            task.priority
        );
    }
}

/// Equivalent to the C `destroy_task_manager()`.
///
/// Takes the `Box` by value so it is dropped (freed) at function end, matching
/// the C semantics where the memory is freed inside this function.
pub fn destroy_task_manager(manager: Box<TaskManager>) {
    drop(manager);
    log_info("TaskManager destroyed successfully.");
}
