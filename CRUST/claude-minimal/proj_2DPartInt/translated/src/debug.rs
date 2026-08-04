use std::fs;
use std::io::Write;

/// Writes the current step/state of the system - all the data structures
/// that represent a particle - to a file in a specified folder.
pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    let path = format!("{}/debug_step_{}.csv", debug_folder, step);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    write_header(step, particle_index, &mut file);
    write_values(particle_index, contacts_size, &mut file);
}

/// Writes a header in the given file. The header contains the columns
/// that represent each attribute of each simulation data structure.
pub fn write_header(step: u64, particle_index: usize, file: &mut std::fs::File) {
    let _ = writeln!(file, "step,particle_index,contacts_size");
    let _ = write!(file, "{},{},", step, particle_index);
}

/// Writes the value of each data structure to the file.
pub fn write_values(particle_index: usize, contacts_size: usize, file: &mut std::fs::File) {
    let _ = writeln!(file, "{},{}", particle_index, contacts_size);
}
