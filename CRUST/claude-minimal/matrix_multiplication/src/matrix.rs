use rand::Rng;

/// Generate an x-by-y matrix.
///
/// Mirrors `int generate_matrix(ARRAY_TYPE*** matrix, int x, int y, bool random)`
/// from c_src/matrix.c. When `random` is false the cell at row `i`, column `j`
/// is set to `(j + i * x) as f32` (matching the C version, which uses `x`
/// rather than `y` for the multiplier). When `random` is true cells are filled
/// with pseudo-random `f32` values.
pub fn generate_matrix(x: usize, y: usize, random: bool) -> Vec<Vec<f32>> {
    let mut matrix: Vec<Vec<f32>> = Vec::with_capacity(x);
    if !random {
        for i in 0..x {
            let mut row: Vec<f32> = Vec::with_capacity(y);
            for j in 0..y {
                row.push((j + i * x) as f32);
            }
            matrix.push(row);
        }
    } else {
        let mut rng = rand::rng();
        for _ in 0..x {
            let mut row: Vec<f32> = Vec::with_capacity(y);
            for _ in 0..y {
                // Match the C `(ARRAY_TYPE)rand()` style: random integer cast to f32.
                let r: i32 = rng.random();
                row.push(r as f32);
            }
            matrix.push(row);
        }
    }
    matrix
}

/// "Free" a matrix. In Rust, memory is reclaimed automatically when the
/// `Vec` goes out of scope, but we still emulate the C function by clearing
/// the contents and returning 0 to mirror `int free_matrix(...)`.
pub fn free_matrix(matrix: &mut Vec<Vec<f32>>) -> i32 {
    for row in matrix.iter_mut() {
        row.clear();
    }
    matrix.clear();
    0
}

/// Dispatch to one of the three multiplication algorithms based on `method`.
///
/// Returns 0 on success and -1 on error (mismatched dimensions or unknown
/// method), matching `int multiply(...)` from c_src/matrix.c.
pub fn multiply(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>], method: i32) -> i32 {
    let x1 = m1.len();
    let y1 = if x1 > 0 { m1[0].len() } else { 0 };
    let x2 = m2.len();
    let _y2 = if x2 > 0 { m2[0].len() } else { 0 };

    if y1 != x2 {
        println!(
            "The number of columns in the first matrix must be equal to the number of rows in the second matrix. "
        );
        return -1;
    }

    match method {
        1 => {
            algorithm1(m1, m2, result);
            0
        }
        2 => {
            algorithm2(m1, m2, result);
            0
        }
        3 => {
            algorithm3(m1, m2, result);
            0
        }
        _ => {
            println!("Choose the correct method!");
            -1
        }
    }
}

/// Naive ijk matrix multiplication: `result = m1 * m2`.
pub fn algorithm1(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let x1 = m1.len();
    let y1 = if x1 > 0 { m1[0].len() } else { 0 };
    let y2 = if !m2.is_empty() { m2[0].len() } else { 0 };

    for i in 0..x1 {
        for j in 0..y2 {
            result[i][j] = 0.0;
            for k in 0..y1 {
                result[i][j] += m1[i][k] * m2[k][j];
            }
        }
    }
}

/// SIMD-style multiplication for square matrices whose size is a multiple of 8.
///
/// Mirrors the AVX-based C algorithm2: the second operand is transposed into
/// `b` so that dot products are computed across contiguous rows, and the
/// result is accumulated into `c`. The math reduces to a standard square
/// matmul over chunks of 8.
pub fn algorithm2(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let size = m1.len();
    if size == 0 {
        return;
    }

    // Transpose m2 into b and zero out the result, just like the C version.
    let mut b: Vec<Vec<f32>> = vec![vec![0.0f32; size]; size];
    for i in 0..size {
        for j in 0..size {
            b[i][j] = m2[j][i];
            result[i][j] = 0.0;
        }
    }

    // Chunked dot-product accumulation. The C version unrolls in groups of 8,
    // operating on lanes [j, j+8); the math is equivalent to iterating over
    // every column index in chunks of 8.
    let mut j = 0usize;
    while j < size {
        let end = (j + 8).min(size);
        for i in 0..size {
            for k in 0..size {
                let mut acc = 0.0f32;
                for jj in j..end {
                    acc += m1[i][jj] * b[k][jj];
                }
                result[i][k] += acc;
            }
        }
        j += 8;
    }
}

/// ikj-ordered matrix multiplication: same math as algorithm1 with a
/// cache-friendlier loop nest.
pub fn algorithm3(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let x1 = m1.len();
    let y1 = if x1 > 0 { m1[0].len() } else { 0 };
    let y2 = if !m2.is_empty() { m2[0].len() } else { 0 };

    for i in 0..x1 {
        for j in 0..y2 {
            result[i][j] = 0.0;
        }
    }

    for i in 0..x1 {
        for k in 0..y1 {
            for j in 0..y2 {
                result[i][j] += m1[i][k] * m2[k][j];
            }
        }
    }
}
