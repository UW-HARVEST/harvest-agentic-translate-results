use std::fs;
use std::io::Write;
use std::path::Path;

/// Writes the current state of the system to a debug file in `debug_folder`.
/// File is named `step_<step>.txt` for easy retrieval.
pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    let path = Path::new(debug_folder).join(format!("step_{}.txt", step));
    if let Ok(mut file) = fs::File::create(&path) {
        write_header(step, particle_index, &mut file);
        write_values(particle_index, contacts_size, &mut file);
    }
}

/// Writes a header describing the columns to the given file.
pub fn write_header(step: u64, particle_index: usize, file: &mut std::fs::File) {
    let _ = writeln!(file, "step,particle_index");
    let _ = writeln!(file, "{},{}", step, particle_index);
    let _ = writeln!(file, "particle_index,contacts_size");
}

/// Writes a row of values for the given particle index and contacts size.
pub fn write_values(particle_index: usize, contacts_size: usize, file: &mut std::fs::File) {
    let _ = writeln!(file, "{},{}", particle_index, contacts_size);
}
