use crate::data;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Ensures the output folder exists. If not, it tries to create it.
/// Returns 0 on success, non-zero on error (mirrors the C int-return).
pub fn ensure_output_folder(output_folder: &str) -> i32 {
    let path = Path::new(output_folder);
    if path.exists() {
        return 0;
    }
    match fs::create_dir_all(path) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Writes a CSV file that can be read by ParaView, with the current
/// status of the simulation. The file is written in the specified folder
/// and suffixed with the step number.
pub fn write_simulation_step(
    num_particles: usize,
    particles: &[data::Particle],
    folder: &str,
    step: u64,
) {
    let path = format!("{}/step_{}.csv", folder, step);
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

/// Writes a CSV representation of the grid (its squares) to the
/// specified folder.
pub fn write_grid(x_squares: i32, y_squares: i32, square_length: f64, folder: &str) {
    let path = format!("{}/grid.csv", folder);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "row,col,x_min,y_min,x_max,y_max");
    let x_left_limit = -((x_squares as f64) * square_length / 2.0);
    for row in 0..y_squares {
        for col in 0..x_squares {
            let x_min = x_left_limit + (col as f64) * square_length;
            let y_min = (row as f64) * square_length;
            let x_max = x_min + square_length;
            let y_max = y_min + square_length;
            let _ = writeln!(file, "{},{},{},{},{},{}", row, col, x_min, y_min, x_max, y_max);
        }
    }
}

/// Writes the particles inside each grid square to a CSV file in the
/// specified folder.
pub fn write_particles_from_grid(
    x_squares: i32,
    y_squares: i32,
    folder: &str,
    grid: &Vec<&mut Vec<&mut data::Particle>>,
    step: i32,
) {
    let path = format!("{}/grid_particles_{}.csv", folder, step);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "square_idx,row,col,particle_idx,x_coordinate,y_coordinate,radius");
    for row in 0..y_squares {
        for col in 0..x_squares {
            let square_idx = (row * x_squares + col) as usize;
            if square_idx >= grid.len() {
                continue;
            }
            for p in grid[square_idx].iter() {
                let _ = writeln!(
                    file,
                    "{},{},{},{},{},{},{}",
                    square_idx, row, col, p.idx, p.x_coordinate, p.y_coordinate, p.radius
                );
            }
        }
    }
}
