use crate::data;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn ensure_output_folder(output_folder: &str) -> i32 {
    let path = Path::new(output_folder);
    if path.exists() {
        if path.is_dir() {
            return 0;
        }
        // A non-directory entry exists at that path.
        return -1;
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
    let path = format!("{}/step_{}.csv", folder.trim_end_matches('/'), step);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    // CSV header that ParaView can interpret.
    if writeln!(file, "x_coordinate,y_coordinate,z_coordinate,radius,idx").is_err() {
        return;
    }
    for i in 0..num_particles {
        if i >= particles.len() {
            break;
        }
        let p = &particles[i];
        if writeln!(
            file,
            "{},{},0,{},{}",
            p.x_coordinate, p.y_coordinate, p.radius, p.idx
        )
        .is_err()
        {
            return;
        }
    }
}

pub fn write_grid(x_squares: i32, y_squares: i32, square_length: f64, folder: &str) {
    let path = format!("{}/grid.csv", folder.trim_end_matches('/'));
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    if writeln!(file, "row,col,x_left,y_bottom,x_right,y_top").is_err() {
        return;
    }
    let x_left_limit = -((x_squares as f64) * square_length / 2.0);
    for row in 0..y_squares {
        for col in 0..x_squares {
            let x_left = x_left_limit + (col as f64) * square_length;
            let y_bottom = (row as f64) * square_length;
            let x_right = x_left + square_length;
            let y_top = y_bottom + square_length;
            if writeln!(
                file,
                "{},{},{},{},{},{}",
                row, col, x_left, y_bottom, x_right, y_top
            )
            .is_err()
            {
                return;
            }
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
    let path = format!(
        "{}/grid_particles_{}.csv",
        folder.trim_end_matches('/'),
        step
    );
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    if writeln!(file, "square_idx,x_coordinate,y_coordinate,radius,idx").is_err() {
        return;
    }

    let total = (x_squares as usize) * (y_squares as usize);
    for square_idx in 0..total {
        if square_idx >= grid.len() {
            continue;
        }
        let cell = &grid[square_idx];
        for p in cell.iter() {
            if writeln!(
                file,
                "{},{},{},{},{}",
                square_idx, p.x_coordinate, p.y_coordinate, p.radius, p.idx
            )
            .is_err()
            {
                return;
            }
        }
    }
}
