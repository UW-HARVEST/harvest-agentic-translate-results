use crate::data;
use crate::functions::compute_overlap;

/// Fills the grid with particles. The side length of each square grid is
/// twice the diameter of each particle, since all particles have the same
/// radius. Running time is O(N), and memory space is O(N) (really 2 * N
/// because of the `grid_lasts` pointers array).
///
/// Note: The provided signature uses `&Vec<&mut Vec<...>>`, which does not
/// permit mutation of the inner buckets through the immutable outer
/// reference. Therefore this implementation only computes the target
/// square index for each particle (the same logic as the C version) and
/// cannot push the particle into the grid.
pub fn fill_grid(
    num_particles: usize,
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    particles: &[data::Particle],
    grid: &Vec<&mut Vec<&data::Particle>>,
    grid_lasts: &Vec<&mut Vec<&mut data::Particle>>,
) {
    let _ = (grid, grid_lasts);
    for i in 0..num_particles {
        let p = &particles[i];
        let square_ind =
            find_square(p.x_coordinate, p.y_coordinate, x_squares, y_squares, square_length);
        if square_ind < 0 {
            // The particle is outside the boundaries of our grid.
            continue;
        }
        // The signature does not allow us to mutate the grid entries here.
    }
}

/// From the Grid, find all the pairs of particles that are colliding with
/// each other, create a `Contact` for each one and save it into
/// `contacts`. Returns the number of collisions.
pub fn compute_contacts(
    grid: &Vec<&Vec<&data::Particle>>,
    x_squares: i32,
    y_squares: i32,
    square_length: f64,
    contacts: &mut [data::Contact],
) -> usize {
    let mut k: usize = 0;
    // For each square.
    for row in 0..y_squares {
        for col in 0..x_squares {
            let square_idx = (row * x_squares + col) as usize;
            let square = grid[square_idx];
            if square.is_empty() {
                continue;
            }
            // For each particle in the current square.
            for p in square.iter() {
                // First compare with particles within the same square.
                for other in square.iter() {
                    if (*other).idx == p.idx {
                        continue;
                    }
                    let overlap = compute_overlap(p, other);
                    if overlap > 0.0 {
                        contacts[k].p1_idx = p.idx as usize;
                        contacts[k].p2_idx = other.idx as usize;
                        contacts[k].overlap = overlap;
                        k += 1;
                    }
                }
                // Then, compare with the surrounding squares.
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
                // Iterate over the neighbor squares.
                for neighbor_row in bottom_row..=top_row {
                    for neighbor_col in left_col..=right_col {
                        let neighbor_square_idx =
                            (neighbor_row * x_squares + neighbor_col) as usize;
                        if neighbor_square_idx == square_idx {
                            continue;
                        }
                        let neighbor_square = grid[neighbor_square_idx];
                        if neighbor_square.is_empty() {
                            continue;
                        }
                        for other in neighbor_square.iter() {
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

/// Helper function. Return the column number given an x coordinate. If
/// the x coordinate lies outside the x dimension covered by the grid,
/// then return a negative number: -2 for the left boundary and -1 for the
/// right boundary.
pub fn find_col(x: f64, x_squares: i32, square_length: f64) -> i32 {
    let x_left_limit = -((x_squares as f64) * square_length / 2.0);
    let diff = x_left_limit - x;
    if diff > 0.0 {
        return -2; // Outside the left boundary.
    }
    let col_ind = (diff.abs() / square_length).floor() as i32;
    if col_ind >= x_squares {
        return -1; // Outside the right boundary.
    }
    col_ind
}

/// Helper function. Return the row number given an y coordinate. If the
/// y coordinate lies below 0, then return a negative number: -2 for the
/// lower boundary and -1 for the upper boundary.
pub fn find_row(y: f64, y_squares: i32, square_length: f64) -> i32 {
    let row_ind = (y / square_length).floor() as i32;
    if row_ind >= y_squares {
        return -1;
    }
    if row_ind < 0 {
        return -2;
    }
    row_ind
}

/// Helper function. Given a point (x, y), find the index for the square
/// that should contain it. If either coordinate lies outside the area
/// covered by the grid, then return -1.
pub fn find_square(x: f64, y: f64, x_squares: i32, y_squares: i32, square_length: f64) -> i32 {
    let row_ind = find_row(y, y_squares, square_length);
    let col_ind = find_col(x, x_squares, square_length);
    if col_ind < 0 || row_ind < 0 {
        return -1;
    }
    row_ind * x_squares + col_ind
}
