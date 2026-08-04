use crate::data;
use std::fs;
use std::io::Write;

pub fn ensure_output_folder(output_folder: &str) -> i32 {
    match fs::create_dir_all(output_folder) {
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
    let path = format!("{}/step_{}.csv", folder, step);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "x,y,radius");
    for i in 0..num_particles {
        let _ = writeln!(
            file,
            "{},{},{}",
            particles[i].x_coordinate, particles[i].y_coordinate, particles[i].radius
        );
    }
}
pub fn write_grid(
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    folder: &str,
) {
    let path = format!("{}/grid.csv", folder);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "x,y");
    let x_offset = -(x_squares as f64 * square_length / 2.0);
    for row in 0..=y_squares {
        let y = row as f64 * square_length;
        for col in 0..=x_squares {
            let x = x_offset + col as f64 * square_length;
            let _ = writeln!(file, "{},{}", x, y);
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
    let path = format!("{}/grid_step_{}.csv", folder, step);
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "x,y,radius");
    let total_squares = (x_squares * y_squares) as usize;
    for i in 0..total_squares {
        for p in grid[i].iter() {
            let _ = writeln!(file, "{},{},{}", p.x_coordinate, p.y_coordinate, p.radius);
        }
    }
}
