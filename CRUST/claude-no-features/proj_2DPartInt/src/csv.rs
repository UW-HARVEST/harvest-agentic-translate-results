use crate::data;
use std::fs;
use std::io::Write;
use std::path::Path;

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

pub fn write_simulation_step(
    num_particles: usize,
    particles: &[data::Particle],
    folder: &str,
    step: u64,
) {
    let _ = ensure_output_folder(folder);
    let path = format!("{}/step_{}.csv", folder, step);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "x_coordinate,y_coordinate,radius,idx");
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

pub fn write_grid(x_squares: i32, y_squares: i32, square_length: f64, folder: &str) {
    let _ = ensure_output_folder(folder);
    let path = format!("{}/grid.csv", folder);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "row,col,x,y,square_length");
    for row in 0..y_squares {
        for col in 0..x_squares {
            let x = (col as f64) * square_length - (x_squares as f64) * square_length / 2.0;
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
    let path = format!("{}/grid_step_{}.csv", folder, step);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "row,col,idx,x_coordinate,y_coordinate,radius");
    let x_squares_us = x_squares.max(0) as usize;
    let y_squares_us = y_squares.max(0) as usize;
    for row in 0..y_squares_us {
        for col in 0..x_squares_us {
            let idx = row * x_squares_us + col;
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
