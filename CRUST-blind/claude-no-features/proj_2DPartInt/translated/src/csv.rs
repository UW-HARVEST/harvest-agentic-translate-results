use crate::data;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Ensures the output folder exists. If not, attempts to create it.
/// Returns 0 on success and 1 on failure.
pub fn ensure_output_folder(output_folder: &str) -> i32 {
    let path = Path::new(output_folder);
    if path.exists() {
        if path.is_dir() {
            return 0;
        }
        return 1;
    }
    match fs::create_dir_all(path) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

/// Writes a CSV file describing the current state of the simulation,
/// suitable for ParaView consumption. The file is named `<step>.csv`
/// and stored inside `folder`.
pub fn write_simulation_step(
    num_particles: usize,
    particles: &[data::Particle],
    folder: &str,
    step: u64,
) {
    let path = Path::new(folder).join(format!("{}.csv", step));
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

/// Writes the grid corner positions to a CSV file inside `folder`.
pub fn write_grid(x_squares: i32, y_squares: i32, square_length: f64, folder: &str) {
    let path = Path::new(folder).join("grid.csv");
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "x,y");
    let x_left = -((x_squares as f64) * square_length / 2.0);
    for row in 0..=y_squares {
        for col in 0..=x_squares {
            let x = x_left + (col as f64) * square_length;
            let y = (row as f64) * square_length;
            let _ = writeln!(file, "{},{}", x, y);
        }
    }
}

/// Writes the particles in the grid to a CSV file. Each row contains the
/// particle's coordinates, radius, idx and the square index it belongs to.
pub fn write_particles_from_grid(
    x_squares: i32,
    y_squares: i32,
    folder: &str,
    grid: &Vec<&mut Vec<&mut data::Particle>>,
    step: i32,
) {
    let path = Path::new(folder).join(format!("grid_{}.csv", step));
    let mut file = match fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "x_coordinate,y_coordinate,radius,idx,square_idx");

    for row in 0..y_squares {
        for col in 0..x_squares {
            let square_idx = (row * x_squares + col) as usize;
            if square_idx >= grid.len() {
                continue;
            }
            let square = &grid[square_idx];
            for p in square.iter() {
                let _ = writeln!(
                    file,
                    "{},{},{},{},{}",
                    p.x_coordinate, p.y_coordinate, p.radius, p.idx, square_idx
                );
            }
        }
    }
}
