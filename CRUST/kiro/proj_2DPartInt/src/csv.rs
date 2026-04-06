use crate::data;
pub fn ensure_output_folder(output_folder: &str) -> i32 {
    match std::fs::create_dir_all(output_folder) {
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
    use std::io::Write;
    let path = format!("{}/step_{}.csv", folder, step);
    let mut file = std::fs::File::create(&path).expect("Failed to create CSV file");
    writeln!(file, "x_coordinate,y_coordinate,radius,idx").unwrap();
    for i in 0..num_particles {
        let p = &particles[i];
        writeln!(file, "{},{},{},{}", p.x_coordinate, p.y_coordinate, p.radius, p.idx).unwrap();
    }
}
pub fn write_grid(
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    folder: &str,
) {
    use std::io::Write;
    let path = format!("{}/grid.csv", folder);
    let mut file = std::fs::File::create(&path).expect("Failed to create grid CSV");
    writeln!(file, "x,y,length").unwrap();
    for row in 0..y_squares {
        for col in 0..x_squares {
            let x = col as f64 * square_length - (x_squares as f64 * square_length / 2.0);
            let y = row as f64 * square_length;
            writeln!(file, "{},{},{}", x, y, square_length).unwrap();
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
    use std::io::Write;
    let path = format!("{}/grid_particles_{}.csv", folder, step);
    let mut file = std::fs::File::create(&path).expect("Failed to create grid particles CSV");
    writeln!(file, "x_coordinate,y_coordinate,radius,idx,square").unwrap();
    for row in 0..y_squares {
        for col in 0..x_squares {
            let square_idx = (row * x_squares + col) as usize;
            if square_idx < grid.len() {
                for p in grid[square_idx].iter() {
                    writeln!(file, "{},{},{},{},{}", p.x_coordinate, p.y_coordinate, p.radius, p.idx, square_idx).unwrap();
                }
            }
        }
    }
}
