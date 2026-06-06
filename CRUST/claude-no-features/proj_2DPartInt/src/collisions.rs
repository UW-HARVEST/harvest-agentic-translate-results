use crate::data;
use crate::functions::compute_overlap;

pub fn fill_grid(
    _num_particles: usize,
    _x_squares: i32,
    _y_squares: i32,
    _square_length: f64,
    _particles: &[data::Particle],
    _grid: &Vec<&mut Vec<&data::Particle>>,
    _grid_lasts: &Vec<&mut Vec<&mut data::Particle>>,
) {
    // The Rust signature provided uses shared (`&`) references to the outer
    // Vecs, which prevents reborrowing the inner mutable references for
    // pushing/clearing. The corresponding C function builds singly-linked
    // lists using mutable particle pointers, which cannot be expressed
    // safely under this signature. Therefore this function is intentionally
    // a no-op for safety; callers that need grid filling should use a
    // signature that grants mutable access to the grid.
}

pub fn compute_contacts(
    grid: &Vec<&Vec<&data::Particle>>,
    x_squares: i32,
    y_squares: i32,
    _square_length: f64,
    contacts: &mut [data::Contact],
) -> usize {
    let mut k: usize = 0;
    let x_squares_us = x_squares.max(0) as usize;
    let y_squares_us = y_squares.max(0) as usize;

    for row in 0..y_squares_us {
        for col in 0..x_squares_us {
            let square_idx = row * x_squares_us + col;
            if square_idx >= grid.len() {
                continue;
            }
            let cell = grid[square_idx];
            if cell.is_empty() {
                continue;
            }

            // Compare every particle to every particle in the same square.
            for p in cell.iter() {
                for other in cell.iter() {
                    if std::ptr::eq(*p, *other) {
                        continue;
                    }
                    let overlap = compute_overlap(p, other);
                    if overlap > 0.0 {
                        if k < contacts.len() {
                            contacts[k] = data::Contact {
                                p1_idx: p.idx as usize,
                                p2_idx: other.idx as usize,
                                overlap,
                            };
                        }
                        k += 1;
                    }
                }
                // Compare against neighbouring squares (Moore neighbourhood, radius 1
                // is sufficient since particles share the same radius and the
                // neighbouring circle fits within the inscribed square of size 2*radius).
                let row_i = row as isize;
                let col_i = col as isize;
                for dr in -1..=1isize {
                    for dc in -1..=1isize {
                        if dr == 0 && dc == 0 {
                            continue;
                        }
                        let nr = row_i + dr;
                        let nc = col_i + dc;
                        if nr < 0 || nc < 0
                            || nr >= y_squares_us as isize
                            || nc >= x_squares_us as isize
                        {
                            continue;
                        }
                        let neighbor_idx = (nr as usize) * x_squares_us + (nc as usize);
                        if neighbor_idx >= grid.len() {
                            continue;
                        }
                        let neighbor_cell = grid[neighbor_idx];
                        for other in neighbor_cell.iter() {
                            let overlap = compute_overlap(p, other);
                            if overlap > 0.0 {
                                if k < contacts.len() {
                                    contacts[k] = data::Contact {
                                        p1_idx: p.idx as usize,
                                        p2_idx: other.idx as usize,
                                        overlap,
                                    };
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
        return -2;
    }
    let col_ind = (diff.abs() / square_length).floor() as i32;
    if col_ind >= x_squares {
        return -1;
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
