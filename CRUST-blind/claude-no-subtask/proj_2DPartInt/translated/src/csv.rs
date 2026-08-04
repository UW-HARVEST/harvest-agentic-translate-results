use crate::data;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Ensures the output folder exists. If it does not, attempts to create it.
/// Returns 0 on success, -1 on failure.
pub fn ensure_output_folder(output_folder: &str) -> i32 {
    let path = Path::new(output_folder);
    if path.exists() {
        if path.is_dir() {
            return 0;
        }
        return -1;
    }
    match fs::create_dir_all(path) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Writes a CSV file representing the current simulation step. The file
/// is suffixed by the step number and placed in the provided folder.
pub fn write_simulation_step(
    num_particles: usize,
    particles: &[data::Particle],
    folder: &str,
    step: u64,
) {
    if ensure_output_folder(folder) != 0 {
        return;
    }
    let path = format!("{}/step_{}.csv", folder.trim_end_matches('/'), step);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "x_coordinate,y_coordinate,radius,idx");
    for i in 0..num_particles {
        let p = &particles[i];
        let _ = writeln!(
            file,
            "{},{},{},{}",
            p.x_coordinate, p.y_coordinate, p.radius, p.idx
        );
    }
}

/// Writes the grid layout to a CSV file.
pub fn write_grid(x_squares: i32, y_squares: i32, square_length: f64, folder: &str) {
    if ensure_output_folder(folder) != 0 {
        return;
    }
    let path = format!("{}/grid.csv", folder.trim_end_matches('/'));
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "row,col,x_left,y_bottom,x_right,y_top");
    let x_left_limit = -((x_squares as f64) * square_length / 2.0);
    for row in 0..y_squares {
        for col in 0..x_squares {
            let x_left = x_left_limit + (col as f64) * square_length;
            let y_bottom = (row as f64) * square_length;
            let x_right = x_left + square_length;
            let y_top = y_bottom + square_length;
            let _ = writeln!(
                file,
                "{},{},{},{},{},{}",
                row, col, x_left, y_bottom, x_right, y_top
            );
        }
    }
}

/// Writes the particles in each grid cell to a CSV file.
pub fn write_particles_from_grid(
    x_squares: i32,
    y_squares: i32,
    folder: &str,
    grid: &Vec<&mut Vec<&mut data::Particle>>,
    step: i32,
) {
    if ensure_output_folder(folder) != 0 {
        return;
    }
    let path = format!(
        "{}/particles_grid_{}.csv",
        folder.trim_end_matches('/'),
        step
    );
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "row,col,idx,x_coordinate,y_coordinate,radius");
    for row in 0..y_squares {
        for col in 0..x_squares {
            let idx = (row * x_squares + col) as usize;
            if idx >= grid.len() {
                continue;
            }
            for p in grid[idx].iter() {
                let _ = writeln!(
                    file,
                    "{},{},{},{},{},{}",
                    row, col, p.idx, p.x_coordinate, p.y_coordinate, p.radius
                );
            }
        }
    }
}
