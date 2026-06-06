use rand::Rng;

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
                // Mirror C's `(float)rand()` which yields a non-negative int
                // value in the range [0, RAND_MAX].
                let v: i32 = rng.random_range(0..=i32::MAX);
                row.push(v as f32);
            }
            matrix.push(row);
        }
    }
    matrix
}

pub fn free_matrix(matrix: &mut Vec<Vec<f32>>) -> i32 {
    // In Rust, memory is freed automatically when the vector is dropped.
    // To mirror the C `free_matrix` semantics (which zeroes out the
    // pointer), we clear the outer vector so callers see an empty matrix.
    matrix.clear();
    0
}

pub fn multiply(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>], method: i32) -> i32 {
    let x1 = m1.len();
    let y1 = if x1 == 0 { 0 } else { m1[0].len() };
    let x2 = m2.len();
    let _y2 = if x2 == 0 { 0 } else { m2[0].len() };

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

pub fn algorithm1(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let x1 = m1.len();
    let y1 = if x1 == 0 { 0 } else { m1[0].len() };
    let y2 = if m2.is_empty() { 0 } else { m2[0].len() };

    for i in 0..x1 {
        for j in 0..y2 {
            let mut sum: f32 = 0.0;
            for k in 0..y1 {
                sum += m1[i][k] * m2[k][j];
            }
            result[i][j] = sum;
        }
    }
}

pub fn algorithm3(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let x1 = m1.len();
    let y1 = if x1 == 0 { 0 } else { m1[0].len() };
    let y2 = if m2.is_empty() { 0 } else { m2[0].len() };

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

pub fn algorithm2(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    // The C reference operates on square matrices: `size x size` where
    // `size` is a multiple of 8 (so the AVX2 vector loads work). The
    // computation is a standard matrix multiply with `m2` transposed for
    // cache locality, plus heavy manual unrolling. In safe Rust we simply
    // perform the equivalent computation: result = m1 * m2.
    let size = m1.len();

    // Transpose m2 to mirror the C code's layout.
    let mut b: Vec<Vec<f32>> = vec![vec![0.0f32; size]; size];
    for i in 0..size {
        for j in 0..size {
            b[i][j] = m2[j][i];
            result[i][j] = 0.0;
        }
    }

    for i in 0..size {
        for j in (0..size).step_by(8) {
            let m_a = &m1[i][j..j + 8];
            for k in 0..size {
                let m_b = &b[k][j..j + 8];
                let mut acc: f32 = 0.0;
                for idx in 0..8 {
                    acc += m_a[idx] * m_b[idx];
                }
                result[i][k] += acc;
            }
        }
    }
}
