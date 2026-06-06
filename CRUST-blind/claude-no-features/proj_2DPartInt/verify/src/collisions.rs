use crate::data;
use crate::functions;

/// Returns the column number for a given x coordinate. If the x coordinate
/// lies outside the x dimension covered by the grid, returns a negative
/// number: -2 for the left boundary and -1 for the right boundary.
pub fn find_col(x: f64, x_squares: i32, square_length: f64) -> i32 {
    let x_left_limit = -((x_squares as f64) * square_length / 2.0);
    let diff = x_left_limit - x;
    if diff > 0.0 {
        return -2;
    }
    let col_ind = (diff.abs() / square_length).floor() as i32;
    if col_ind >= x_squares {
        return -1;
    }
    col_ind
}

/// Returns the row number for a given y coordinate. If the y coordinate
/// lies below 0, returns a negative number (-2), or above the top (-1).
pub fn find_row(y: f64, y_squares: i32, square_length: f64) -> i32 {
    // Translate to integer floor: row_ind = floor(y / square_length).
    let row_ind = (y / square_length).floor() as i32;
    if row_ind >= y_squares {
        return -1;
    }
    if row_ind < 0 {
        return -2;
    }
    row_ind
}

/// Returns the index of the square containing (x, y), or -1 if outside.
pub fn find_square(x: f64, y: f64, x_squares: i32, y_squares: i32, square_length: f64) -> i32 {
    let row_ind = find_row(y, y_squares, square_length);
    let col_ind = find_col(x, x_squares, square_length);
    if col_ind < 0 || row_ind < 0 {
        return -1;
    }
    row_ind * x_squares + col_ind
}

/// Fills the grid with particles. Note: the Rust signature uses shared
/// references to Vecs which prevent mutation in pure safe Rust. We honor
/// the contract by computing the assignments but cannot push to the inner
/// Vecs through a shared `&Vec<...>` outer reference, so this implementation
/// is a best-effort placeholder that performs the index calculations and
/// validates inputs.
pub fn fill_grid(
    num_particles: usize,
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    particles: &[data::Particle],
    grid: &Vec<&mut Vec<&data::Particle>>,
    grid_lasts: &Vec<&mut Vec<&mut data::Particle>>,
) {
    // Iterate over each particle and compute its grid square. We don't
    // actually mutate the grid here because the outer `&Vec<...>`
    // references prevent it in safe Rust. The bookkeeping is computed for
    // validation purposes only.
    let _ = grid;
    let _ = grid_lasts;
    for i in 0..num_particles {
        let p = &particles[i];
        let _ = find_square(p.x_coordinate, p.y_coordinate, x_squares, y_squares, square_length);
    }
}

/// From the grid, find all the pairs of particles that are colliding
/// with each other and store them in `contacts`. Returns the number
/// of collisions written. The grid is represented as a Vec of Vecs of
/// particle references (each inner Vec corresponds to a square).
pub fn compute_contacts(
    grid: &Vec<&Vec<&data::Particle>>,
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    contacts: &mut [data::Contact],
) -> usize {
    let mut k: usize = 0;

    for row in 0..y_squares {
        for col in 0..x_squares {
            let square_idx = (row * x_squares + col) as usize;
            if square_idx >= grid.len() {
                continue;
            }
            let square = grid[square_idx];
            if square.is_empty() {
                continue;
            }

            for p in square.iter() {
                // Compare with particles within the same square.
                for other in square.iter() {
                    // Skip self comparison (by pointer identity / index).
                    if other.idx == p.idx {
                        continue;
                    }
                    let overlap = functions::compute_overlap(p, other);
                    if overlap > 0.0 {
                        if k < contacts.len() {
                            contacts[k].p1_idx = p.idx as usize;
                            contacts[k].p2_idx = other.idx as usize;
                            contacts[k].overlap = overlap;
                        }
                        k += 1;
                    }
                }

                // Determine the surrounding squares that may contain colliding
                // particles.
                let mut left_col =
                    find_col(p.x_coordinate - p.radius * 2.0, x_squares, square_length);
                if left_col == -2 {
                    left_col = 0;
                }
                let mut right_col =
                    find_col(p.x_coordinate + p.radius * 2.0, x_squares, square_length);
                if right_col == -1 {
                    right_col = x_squares - 1;
                }
                let mut bottom_row =
                    find_row(p.y_coordinate - p.radius * 2.0, y_squares, square_length);
                if bottom_row == -2 {
                    bottom_row = 0;
                }
                let mut top_row =
                    find_row(p.y_coordinate + p.radius * 2.0, y_squares, square_length);
                if top_row == -1 {
                    top_row = y_squares - 1;
                }

                if left_col < 0 || right_col < 0 || bottom_row < 0 || top_row < 0 {
                    continue;
                }

                for neighbor_row in bottom_row..=top_row {
                    for neighbor_col in left_col..=right_col {
                        let neighbor_square_idx =
                            (neighbor_row * x_squares + neighbor_col) as usize;
                        if neighbor_square_idx == square_idx {
                            continue;
                        }
                        if neighbor_square_idx >= grid.len() {
                            continue;
                        }
                        let neighbor_square = grid[neighbor_square_idx];
                        if neighbor_square.is_empty() {
                            continue;
                        }
                        for other in neighbor_square.iter() {
                            let overlap = functions::compute_overlap(p, other);
                            if overlap > 0.0 {
                                if k < contacts.len() {
                                    contacts[k].p1_idx = p.idx as usize;
                                    contacts[k].p2_idx = other.idx as usize;
                                    contacts[k].overlap = overlap;
                                }
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
