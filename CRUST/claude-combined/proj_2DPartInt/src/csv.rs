use crate::data;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn ensure_output_folder(output_folder: &str) -> i32 {
    if Path::new(output_folder).is_dir() {
        return 0;
    }
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
    let path = format!("{}/step_{}.csv", folder.trim_end_matches('/'), step);
    let file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut writer = std::io::BufWriter::new(file);
    let _ = writeln!(writer, "x_coord,y_coord,z_coord,radius");
    for i in 0..num_particles {
        let p = &particles[i];
        let _ = writeln!(
            writer,
            "{},{},0,{}",
            p.x_coordinate, p.y_coordinate, p.radius
        );
    }
}

pub fn write_grid(
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    folder: &str,
) {
    let path = format!("{}/grid.csv", folder.trim_end_matches('/'));
    let file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut writer = std::io::BufWriter::new(file);
    let _ = writeln!(writer, "x_coord,y_coord,z_coord");
    let x_left = -((x_squares as f64) * square_length / 2.0);
    for row in 0..=y_squares {
        for col in 0..=x_squares {
            let x = x_left + (col as f64) * square_length;
            let y = (row as f64) * square_length;
            let _ = writeln!(writer, "{},{},0", x, y);
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
    let file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut writer = std::io::BufWriter::new(file);
    let _ = writeln!(writer, "x_coord,y_coord,z_coord,radius,square_idx");
    for row in 0..y_squares {
        for col in 0..x_squares {
            let idx = (row * x_squares + col) as usize;
            if idx >= grid.len() {
                continue;
            }
            for p in grid[idx].iter() {
                let _ = writeln!(
                    writer,
                    "{},{},0,{},{}",
                    p.x_coordinate, p.y_coordinate, p.radius, idx
                );
            }
        }
    }
}
