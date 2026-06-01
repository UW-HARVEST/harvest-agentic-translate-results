use std::fs;
use std::io::Write;

pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    let _ = fs::create_dir_all(debug_folder);
    let path = format!(
        "{}/debug_step_{}_particle_{}.txt",
        debug_folder.trim_end_matches('/'),
        step,
        particle_index
    );
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    write_header(step, particle_index, &mut file);
    write_values(particle_index, contacts_size, &mut file);
}

pub fn write_header(
    step: u64,
    particle_index: usize,
    file: &mut std::fs::File,
) {
    let _ = writeln!(
        file,
        "step,particle_index,contacts_size"
    );
    let _ = writeln!(file, "# step={}, particle_index={}", step, particle_index);
}

pub fn write_values(
    particle_index: usize,
    contacts_size: usize,
    file: &mut std::fs::File,
) {
    let _ = writeln!(file, "{},{}", particle_index, contacts_size);
}
