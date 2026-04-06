pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    use std::io::Write;
    let path = format!("{}/debug_{}_{}.csv", debug_folder, step, particle_index);
    let mut file = std::fs::File::create(&path).expect("Failed to create debug file");
    write_header(step, particle_index, &mut file);
    write_values(particle_index, contacts_size, &mut file);
}
pub fn write_header(
    step: u64,
    particle_index: usize,
    file: &mut std::fs::File,
) {
    use std::io::Write;
    writeln!(file, "step,particle_index").unwrap();
    writeln!(file, "{},{}", step, particle_index).unwrap();
}
pub fn write_values(
    particle_index: usize,
    contacts_size: usize,
    file: &mut std::fs::File,
) {
    use std::io::Write;
    writeln!(file, "particle_index,contacts_size").unwrap();
    writeln!(file, "{},{}", particle_index, contacts_size).unwrap();
}
