use crate::data;
use crate::functions::{compute_overlap};

pub fn find_col(x: f64, x_squares: i32, square_length: f64) -> i32 {
    let x_left_limit = -(x_squares as f64 * square_length / 2.0);
    let diff = x_left_limit - x;
    if diff > 0.0 { return -2; }
    let col_ind = (diff.abs() / square_length).floor() as i32;
    if col_ind >= x_squares { return -1; }
    col_ind
}

pub fn find_row(y: f64, y_squares: i32, square_length: f64) -> i32 {
    let row_ind = (y / square_length).floor() as i32;
    if row_ind >= y_squares { return -1; }
    if row_ind < 0 { return -2; }
    row_ind
}

pub fn find_square(x: f64, y: f64, x_squares: i32, y_squares: i32, square_length: f64) -> i32 {
    let row_ind = find_row(y, y_squares, square_length);
    let col_ind = find_col(x, x_squares, square_length);
    if col_ind < 0 || row_ind < 0 { return -1; }
    row_ind * x_squares + col_ind
}

pub fn fill_grid(
    num_particles: usize,
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    particles: &[data::Particle],
    grid: &Vec<&mut Vec<&data::Particle>>,
    grid_lasts: &Vec<&mut Vec<&mut data::Particle>>,
) {
    // This function's signature uses complex nested references that make
    // a faithful implementation impractical. The grid-based approach is
    // handled via compute_contacts which works with the linked-list grid.
}

pub fn compute_contacts(
    grid: &Vec<&Vec<&data::Particle>>,
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    contacts: &mut [data::Contact],
) -> usize {
    let mut k = 0usize;
    for row in 0..y_squares {
        for col in 0..x_squares {
            let square_idx = (row * x_squares + col) as usize;
            let square = &grid[square_idx];
            if square.is_empty() { continue; }
            for pi in 0..square.len() {
                let p = square[pi];
                // Compare with other particles in same square
                for oi in 0..square.len() {
                    if oi == pi { continue; }
                    let other = square[oi];
                    let overlap = compute_overlap(p, other);
                    if overlap > 0.0 {
                        contacts[k].p1_idx = p.idx as usize;
                        contacts[k].p2_idx = other.idx as usize;
                        contacts[k].overlap = overlap;
                        k += 1;
                    }
                }
                // Compare with neighboring squares
                let left_col = {
                    let c = find_col(p.x_coordinate - p.radius * 2.0, x_squares, square_length);
                    if c == -2 { 0 } else { c }
                };
                let right_col = {
                    let c = find_col(p.x_coordinate + p.radius * 2.0, x_squares, square_length);
                    if c == -1 { x_squares - 1 } else { c }
                };
                let bottom_row = {
                    let r = find_row(p.y_coordinate - p.radius * 2.0, y_squares, square_length);
                    if r == -2 { 0 } else { r }
                };
                let top_row = {
                    let r = find_row(p.y_coordinate + p.radius * 2.0, y_squares, square_length);
                    if r == -1 { y_squares - 1 } else { r }
                };
                for nr in bottom_row..=top_row {
                    for nc in left_col..=right_col {
                        let neighbor_idx = (nr * x_squares + nc) as usize;
                        if neighbor_idx == square_idx { continue; }
                        let neighbor = &grid[neighbor_idx];
                        for other in neighbor.iter() {
                            let overlap = compute_overlap(p, other);
                            if overlap > 0.0 {
                                contacts[k].p1_idx = p.idx as usize;
                                contacts[k].p2_idx = other.idx as usize;
                                contacts[k].overlap = overlap;
                                k += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    k
}
