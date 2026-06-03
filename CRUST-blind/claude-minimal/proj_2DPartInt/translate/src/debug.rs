use std::fs::{File, OpenOptions};
use std::io::Write;

/// Writes the current step/state of the system to a debug file.
pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    let filename = format!(
        "{}/debug_{}.csv",
        debug_folder.trim_end_matches('/'),
        particle_index
    );
    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filename)
    {
        Ok(f) => f,
        Err(_) => return,
    };

    write_header(step, particle_index, &mut file);
    write_values(particle_index, contacts_size, &mut file);
}

/// Writes a header row containing column names for a particle's debug file.
pub fn write_header(step: u64, particle_index: usize, file: &mut File) {
    let _ = writeln!(
        file,
        "step={},particle_index={}",
        step, particle_index
    );
    let _ = writeln!(file, "particle_index,contacts_size");
}

/// Writes the values of the particle/contacts to the debug file.
pub fn write_values(particle_index: usize, contacts_size: usize, file: &mut File) {
    let _ = writeln!(file, "{},{}", particle_index, contacts_size);
}
