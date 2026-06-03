use crate::data;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Ensures the output folder exists. Returns 0 on success, -1 on failure.
pub fn ensure_output_folder(output_folder: &str) -> i32 {
    let path = Path::new(output_folder);
    if path.is_dir() {
        return 0;
    }
    match fs::create_dir_all(path) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Writes a CSV file readable by ParaView, suffixed with the step number.
pub fn write_simulation_step(
    num_particles: usize,
    particles: &[data::Particle],
    folder: &str,
    step: u64,
) {
    let filename = format!("{}/step_{}.csv", folder.trim_end_matches('/'), step);
    let mut file = match fs::File::create(&filename) {
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

/// Writes the static grid layout to a CSV file in the given folder.
pub fn write_grid(x_squares: i32, y_squares: i32, square_length: f64, folder: &str) {
    let filename = format!("{}/grid.csv", folder.trim_end_matches('/'));
    let mut file = match fs::File::create(&filename) {
        Ok(f) => f,
        Err(_) => return,
    };

    let _ = writeln!(file, "row,col,x,y,length");
    let x_left_limit = -((x_squares as f64) * square_length / 2.0);
    for row in 0..y_squares {
        for col in 0..x_squares {
            let x = x_left_limit + (col as f64) * square_length;
            let y = (row as f64) * square_length;
            let _ = writeln!(file, "{},{},{},{},{}", row, col, x, y, square_length);
        }
    }
}

/// Writes the particles found in each grid square to a step-specific CSV file.
pub fn write_particles_from_grid(
    x_squares: i32,
    y_squares: i32,
    folder: &str,
    grid: &Vec<&mut Vec<&mut data::Particle>>,
    step: i32,
) {
    let filename = format!(
        "{}/grid_step_{}.csv",
        folder.trim_end_matches('/'),
        step
    );
    let mut file = match fs::File::create(&filename) {
        Ok(f) => f,
        Err(_) => return,
    };

    let _ = writeln!(file, "square_idx,row,col,particle_idx,x,y,radius");
    for row in 0..y_squares {
        for col in 0..x_squares {
            let square_idx = (row * x_squares + col) as usize;
            if square_idx >= grid.len() {
                continue;
            }
            for particle in grid[square_idx].iter() {
                let _ = writeln!(
                    file,
                    "{},{},{},{},{},{},{}",
                    square_idx,
                    row,
                    col,
                    particle.idx,
                    particle.x_coordinate,
                    particle.y_coordinate,
                    particle.radius
                );
            }
        }
    }
}
