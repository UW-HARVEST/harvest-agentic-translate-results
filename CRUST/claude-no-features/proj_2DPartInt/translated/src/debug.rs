use std::io::Write;

pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    if std::path::Path::new(debug_folder).exists() == false {
        let _ = std::fs::create_dir_all(debug_folder);
    }
    let path = format!("{}/debug_step_{}_p_{}.csv", debug_folder, step, particle_index);
    let mut file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    write_header(step, particle_index, &mut file);
    write_values(particle_index, contacts_size, &mut file);
}

pub fn write_header(step: u64, particle_index: usize, file: &mut std::fs::File) {
    let _ = writeln!(file, "step,{},particle_index,{}", step, particle_index);
    let _ = writeln!(file, "contacts_size");
}

pub fn write_values(particle_index: usize, contacts_size: usize, file: &mut std::fs::File) {
    let _ = writeln!(file, "{},{}", particle_index, contacts_size);
}
