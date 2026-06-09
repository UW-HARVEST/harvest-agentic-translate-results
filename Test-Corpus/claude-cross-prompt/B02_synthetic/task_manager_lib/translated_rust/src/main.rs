use std::io::{self, Read, Write};
use std::process::ExitCode;

mod logger;
mod task_manager;
mod driver;

fn main() -> ExitCode {
    // Read all of stdin into a buffer (matches passing tasks string to driver())
    let mut input = Vec::new();
    if let Err(_) = io::stdin().read_to_end(&mut input) {
        return ExitCode::from(1);
    }

    let mut logger = logger::Logger::new();
    let mut task_manager_state = task_manager::TaskManagerState::new();

    let exit_code = driver::driver(&input, &mut logger, &mut task_manager_state);

    // Flush stdout before exit
    let _ = io::stdout().flush();

    ExitCode::from(exit_code as u8)
}
