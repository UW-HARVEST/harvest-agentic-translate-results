use crate::data;
use std::fs::{self, File};
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
    let path = format!("{folder}/2DPartInt-Out.csv.{step}");
    let Ok(mut output_file) = File::create(path) else {
        return;
    };

    let _ = writeln!(output_file, "x coord, y coord, z coord, radius");
    for particle in particles.iter().take(num_particles) {
        let _ = writeln!(
            output_file,
            "{}, {}, {}, {}",
            particle.x_coordinate, particle.y_coordinate, 0, particle.radius
        );
    }
}
pub fn write_grid(
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    folder: &str,
) {
    let path = format!("{folder}/2DPartInt-Out-GRID.csv");
    let Ok(mut output_file) = File::create(path) else {
        return;
    };

    let _ = writeln!(output_file, "x coord, y coord, length");

    let x_left_limit = -((x_squares as f64) * square_length / 2.0);
    let mut y = 0.0;
    for _row in 0..y_squares {
        let mut x = x_left_limit;
        for _col in 0..x_squares {
            let _ = writeln!(output_file, "{x}, {y}, {square_length}");
            x += square_length;
        }
        y += square_length;
    }
}
pub fn write_particles_from_grid(
    x_squares: i32,
    y_squares: i32,
    folder: &str,
    grid: &Vec<&mut Vec<&mut data::Particle>>,
    step: i32,
) {
    let path = format!("{folder}/2DPartInt-Out-FROM-GRID.csv.{step}");
    let Ok(mut output_file) = File::create(path) else {
        return;
    };

    let _ = writeln!(output_file, "x coord, y coord, length");
    for row in 0..y_squares {
        for col in 0..x_squares {
            let idx = (row * x_squares + col) as usize;
            for particle in grid[idx].iter() {
                let _ = writeln!(
                    output_file,
                    "{}, {}, {}, {}",
                    particle.x_coordinate, particle.y_coordinate, 0, particle.radius
                );
            }
        }
    }
}
