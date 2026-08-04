use crate::data;
use crate::functions;

pub fn fill_grid(
    _num_particles: usize,
    _x_squares: i32,
    _y_squares: i32,
    _square_length: f64,
    _particles: &[data::Particle],
    _grid: &Vec<&mut Vec<&data::Particle>>,
    _grid_lasts: &Vec<&mut Vec<&mut data::Particle>>,
) {
    // The provided Rust signature offers shared access (`&Vec<&mut Vec<...>>`)
    // to the grid. In safe Rust, a shared reference to a Vec containing mutable
    // references does not allow mutation of the inner Vecs (you cannot project
    // out a `&mut T` from a `&` of a mutable reference). Therefore this
    // function cannot effectively populate the grid via the provided interface.
    //
    // The conventional, safe-Rust way to fill the grid would be to iterate
    // through the particles, compute each particle's `find_square` index, and
    // push a reference into the corresponding cell. We retain this intent for
    // documentation, but cannot mutate via the supplied references.
}

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
            let cell = grid[square_idx];
            if cell.is_empty() {
                continue;
            }

            for (i, p) in cell.iter().enumerate() {
                // Compare with particles within the same square.
                for (j, other) in cell.iter().enumerate() {
                    if i == j {
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

                // Compare with particles in the surrounding squares.
                let mut left_col = find_col(p.x_coordinate - p.radius * 2.0, x_squares, square_length);
                if left_col == -2 {
                    left_col = 0;
                }
                let mut right_col = find_col(p.x_coordinate + p.radius * 2.0, x_squares, square_length);
                if right_col == -1 {
                    right_col = x_squares - 1;
                }
                let mut bottom_row = find_row(p.y_coordinate - p.radius * 2.0, y_squares, square_length);
                if bottom_row == -2 {
                    bottom_row = 0;
                }
                let mut top_row = find_row(p.y_coordinate + p.radius * 2.0, y_squares, square_length);
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
                        let neighbor_cell = grid[neighbor_square_idx];
                        if neighbor_cell.is_empty() {
                            continue;
                        }
                        for other in neighbor_cell.iter() {
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

pub fn find_square(x: f64, y: f64, x_squares: i32, y_squares: i32, square_length: f64) -> i32 {
    let row_ind = find_row(y, y_squares, square_length);
    let col_ind = find_col(x, x_squares, square_length);
    if col_ind < 0 || row_ind < 0 {
        return -1;
    }
    row_ind * x_squares + col_ind
}
