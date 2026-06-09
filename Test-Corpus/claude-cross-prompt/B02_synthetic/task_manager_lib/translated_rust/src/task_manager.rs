use std::env;
use crate::logger::Logger;

#[derive(Clone)]
pub struct Task {
    /// Description: 256-byte buffer (matches C's `char description[256]`).
    pub description: [u8; 256],
    pub priority: i32,
}

impl Task {
    fn new() -> Self {
        Task { description: [0u8; 256], priority: 0 }
    }
}

pub struct TaskManagerState {
    pub tasks: Vec<Task>,
    pub max_tasks: i32,
    pub task_count: i32,
}

impl TaskManagerState {
    pub fn new() -> Self {
        TaskManagerState { tasks: Vec::new(), max_tasks: 0, task_count: 0 }
    }
}

/// Mimics atoi(): parses leading digits (with optional sign), returns 0 on no parseable digits.
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    // Skip leading whitespace (matches atoi behavior)
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n'
        || bytes[i] == b'\r' || bytes[i] == 0x0b || bytes[i] == 0x0c) {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        result = result.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    (result * sign as i64) as i32
}

/// Returns Some(state) on success, None on failure (matches C's create_task_manager returning NULL).
pub fn create_task_manager(state: &mut TaskManagerState, logger: &mut Logger) -> bool {
    let max_tasks_env = env::var("MAX_TASKS").ok();
    let max_tasks = match max_tasks_env {
        Some(s) => c_atoi(&s),
        None => 10,
    };
    state.max_tasks = max_tasks;
    state.task_count = 0;

    // Allocate tasks vector with max_tasks entries (mimicking malloc).
    // C calls malloc(max_tasks * sizeof(Task)). If max_tasks is <= 0, malloc(0) may succeed or
    // return NULL implementation-defined. We replicate by allocating an empty vector when <= 0.
    if max_tasks < 0 {
        // Treat negative as failure to allocate (Rust would also likely fail).
        logger.log_error("Failed to allocate memory for tasks.");
        return false;
    }
    state.tasks = vec![Task::new(); max_tasks as usize];

    logger.log_info("TaskManager created successfully.");
    true
}

pub fn add_task(state: &mut TaskManagerState, logger: &mut Logger, description: &[u8], priority: i32) {
    if state.task_count >= state.max_tasks {
        logger.log_warning("Cannot add task: Maximum task limit reached.");
        return;
    }

    let idx = state.task_count as usize;
    let task = &mut state.tasks[idx];
    state.task_count += 1;

    // strncpy(task->description, description, 255); task->description[255] = '\0';
    // Copy up to 255 bytes from description into task.description, null-terminate.
    let copy_len = description.len().min(255);
    // Reset buffer to zero first (strncpy pads with zeros if src shorter than n)
    for b in task.description.iter_mut() { *b = 0; }
    task.description[..copy_len].copy_from_slice(&description[..copy_len]);
    // strncpy copies up to n bytes; if description has > 255 bytes, only first 255 copied,
    // then we manually set [255] = '\0'. If <= 255 bytes, strncpy already null-terminated rest.
    task.description[255] = 0;
    task.priority = priority;

    logger.log_info("Task added successfully.");
}

pub fn print_tasks(state: &TaskManagerState) {
    println!("Tasks:");
    for i in 0..state.task_count {
        let task = &state.tasks[i as usize];
        // Find null terminator within description buffer
        let end = task.description.iter().position(|&b| b == 0).unwrap_or(256);
        // Use lossless byte output: write raw bytes through stdout to match C's printf "%s"
        // (which writes raw bytes verbatim until '\0').
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = write!(handle, "  [{}] ", i + 1);
        let _ = handle.write_all(&task.description[..end]);
        let _ = writeln!(handle, " (Priority: {})", task.priority);
    }
}

pub fn destroy_task_manager(state: &mut TaskManagerState, logger: &mut Logger) {
    state.tasks.clear();
    state.task_count = 0;
    state.max_tasks = 0;
    logger.log_info("TaskManager destroyed successfully.");
}
