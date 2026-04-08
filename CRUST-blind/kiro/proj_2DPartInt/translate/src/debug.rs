use std::io::Write;

pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    let path = format!("{}/debug_step_{}.csv", debug_folder, step);
    let mut file = std::fs::File::create(&path).expect("Could not create debug file");
    write_header(step, particle_index, &mut file);
    write_values(particle_index, contacts_size, &mut file);
}
pub fn write_header(
    step: u64,
    particle_index: usize,
    file: &mut std::fs::File,
) {
    writeln!(file, "step,particle_index").unwrap();
    writeln!(file, "{},{}", step, particle_index).unwrap();
}
pub fn write_values(
    particle_index: usize,
    contacts_size: usize,
    file: &mut std::fs::File,
) {
    writeln!(file, "particle_index,contacts_size").unwrap();
    writeln!(file, "{},{}", particle_index, contacts_size).unwrap();
}
