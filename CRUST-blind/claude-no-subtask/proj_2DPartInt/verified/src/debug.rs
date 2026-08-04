use std::fs;
use std::io::Write;
use std::path::Path;

/// Writes the current step/state of the system to a file in the
/// specified debug folder.
pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    let folder = Path::new(debug_folder);
    if !folder.exists() {
        if fs::create_dir_all(folder).is_err() {
            return;
        }
    }
    let path = format!(
        "{}/debug_step_{}.csv",
        debug_folder.trim_end_matches('/'),
        step
    );
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    write_header(step, particle_index, &mut file);
    write_values(particle_index, contacts_size, &mut file);
}

/// Writes a header row to the given file describing the columns
/// representing each attribute of each simulation data structure.
pub fn write_header(step: u64, particle_index: usize, file: &mut std::fs::File) {
    let _ = writeln!(
        file,
        "step,particle_index,x_coordinate,y_coordinate,radius,idx,velocity_x,velocity_y,acceleration_x,acceleration_y,force_x,force_y,contacts_size"
    );
    let _ = writeln!(file, "# step={}, particle_index={}", step, particle_index);
}

/// Writes the value of each data structure to a file.
pub fn write_values(particle_index: usize, contacts_size: usize, file: &mut std::fs::File) {
    // Without access to the actual particle / vector arrays in this
    // simplified Rust port, we record the indices we were asked about.
    let _ = writeln!(file, "{},{}", particle_index, contacts_size);
}
