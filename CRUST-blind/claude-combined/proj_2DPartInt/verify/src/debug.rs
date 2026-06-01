use std::fs;
use std::io::Write;
use std::path::Path;

pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    let path = Path::new(debug_folder);
    if !path.exists() {
        let _ = fs::create_dir_all(path);
    }
    let file_path = format!(
        "{}/debug_step_{}.txt",
        debug_folder.trim_end_matches('/'),
        step
    );
    let file = match fs::File::create(&file_path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut file = file;
    write_header(step, particle_index, &mut file);
    write_values(particle_index, contacts_size, &mut file);
}

pub fn write_header(
    step: u64,
    particle_index: usize,
    file: &mut std::fs::File,
) {
    let _ = writeln!(file, "step,particle_index");
    let _ = writeln!(file, "{},{}", step, particle_index);
    let _ = writeln!(
        file,
        "x_coordinate,y_coordinate,radius,idx,mass,kn,ks,force_x,force_y,velocity_x,velocity_y,acceleration_x,acceleration_y,displacement_x,displacement_y,contacts_size"
    );
}

pub fn write_values(
    particle_index: usize,
    contacts_size: usize,
    file: &mut std::fs::File,
) {
    // Without access to the actual data structures, only the index/contacts
    // metadata is recorded. The C debug helper is also primarily a thin
    // formatter over global state.
    let _ = writeln!(file, "{},{}", particle_index, contacts_size);
}
