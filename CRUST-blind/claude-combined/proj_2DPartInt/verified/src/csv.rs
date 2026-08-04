use crate::data;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Ensures the output folder exists; creates it if necessary.
/// Returns 0 on success and a non-zero value on failure.
pub fn ensure_output_folder(output_folder: &str) -> i32 {
    let path = Path::new(output_folder);
    if path.exists() {
        if path.is_dir() {
            0
        } else {
            -1
        }
    } else {
        match fs::create_dir_all(path) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

pub fn write_simulation_step(
    num_particles: usize,
    particles: &[data::Particle],
    folder: &str,
    step: u64,
) {
    let _ = ensure_output_folder(folder);
    let path = format!("{}/step_{}.csv", folder.trim_end_matches('/'), step);
    let file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut file = file;
    let _ = writeln!(file, "x,y,radius,idx");
    for i in 0..num_particles {
        if i >= particles.len() {
            break;
        }
        let p = &particles[i];
        let _ = writeln!(
            file,
            "{},{},{},{}",
            p.x_coordinate, p.y_coordinate, p.radius, p.idx
        );
    }
}

pub fn write_grid(
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    folder: &str,
) {
    let _ = ensure_output_folder(folder);
    let path = format!("{}/grid.csv", folder.trim_end_matches('/'));
    let file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut file = file;
    let _ = writeln!(file, "row,col,x,y,length");
    let x_left = -((x_squares as f64) * square_length / 2.0);
    for row in 0..y_squares {
        for col in 0..x_squares {
            let x = x_left + (col as f64) * square_length;
            let y = (row as f64) * square_length;
            let _ = writeln!(file, "{},{},{},{},{}", row, col, x, y, square_length);
        }
    }
}

pub fn write_particles_from_grid(
    x_squares: i32,
    y_squares: i32,
    folder: &str,
    grid: &Vec<&mut Vec<&mut data::Particle>>,
    step: i32,
) {
    let _ = ensure_output_folder(folder);
    let path = format!("{}/grid_step_{}.csv", folder.trim_end_matches('/'), step);
    let file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut file = file;
    let _ = writeln!(file, "square,row,col,x,y,radius,idx");
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
                    square_idx, row, col, p.x_coordinate, p.y_coordinate, p.radius, p.idx
                );
            }
        }
    }
}
