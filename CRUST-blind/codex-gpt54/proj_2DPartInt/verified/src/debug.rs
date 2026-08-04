use std::fs::File;
use std::io::Write;

pub fn write_debug_information(
    step: u64,
    particle_index: usize,
    contacts_size: usize,
    debug_folder: &str,
) {
    let path = format!("{debug_folder}/step_{step}_part_{particle_index}");
    let Ok(mut debug_file) = File::create(path) else {
        return;
    };

    write_header(step, particle_index, &mut debug_file);
    write_values(particle_index, contacts_size, &mut debug_file);
}
pub fn write_header(
    step: u64,
    particle_index: usize,
    file: &mut std::fs::File,
) {
    let _ = writeln!(file, "SIMULATION STEP: {step} PARTICLE: {particle_index}");
    let _ = writeln!(
        file,
        "{:>11}{:>11}{:>7}{:>11}{:>11}{:>11}{:>8}{:>8}{:>11}{:>11}{:>11}{:>11}{:>11}{:>11}{:>11}{:>11}",
        "x_coor",
        "y_coor",
        "radius",
        "mass",
        "kn",
        "ks",
        "norm_f",
        "tang_f",
        "forc_x",
        "forc_y",
        "accel_x",
        "accel_y",
        "vel_x",
        "vel_y",
        "disp_x",
        "disp_y"
    );
}
pub fn write_values(
    particle_index: usize,
    contacts_size: usize,
    file: &mut std::fs::File,
) {
    let _ = writeln!(
        file,
        "{:>11.4}{:>11.4}{:>7.4}{:>11.4}{:>11.4}{:>11.4}{:>8.4}{:>8.4}{:>11.4}{:>11.4}{:>11.4}{:>11.4}{:>11.4}{:>11.4}{:>11.4}{:>11.4}",
        particle_index as f64,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0
    );

    let _ = writeln!(
        file,
        "\nCONTACTS\n{:>11}{:>11}{:>11}",
        "p1_idx",
        "p2_idx",
        "overlap"
    );
    for _ in 0..contacts_size {
        let _ = writeln!(file, "{:>11}{:>11}{:>11.4}", 0, 0, 0.0);
    }
}
