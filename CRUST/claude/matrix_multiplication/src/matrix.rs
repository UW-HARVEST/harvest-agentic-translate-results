use rand::Rng;

pub fn generate_matrix(x: usize, y: usize, random: bool) -> Vec<Vec<f32>> {
    let mut matrix: Vec<Vec<f32>> = Vec::with_capacity(x);
    if !random {
        for i in 0..x {
            let mut row: Vec<f32> = Vec::with_capacity(y);
            for j in 0..y {
                // Mirrors C: (*matrix)[i][j] = (ARRAY_TYPE)(j + i * x);
                row.push((j + i * x) as f32);
            }
            matrix.push(row);
        }
    } else {
        let mut rng = rand::rng();
        for _ in 0..x {
            let mut row: Vec<f32> = Vec::with_capacity(y);
            for _ in 0..y {
                // Mirrors C: (ARRAY_TYPE)rand();
                let v: i32 = rng.random();
                row.push(v as f32);
            }
            matrix.push(row);
        }
    }
    matrix
}

pub fn free_matrix(matrix: &mut Vec<Vec<f32>>) -> i32 {
    // In Rust, dropping the vector frees memory; we mirror the C contract by
    // emptying the outer vector (and thereby its rows).
    matrix.clear();
    0
}

pub fn multiply(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>], method: i32) -> i32 {
    let y1 = if m1.is_empty() { 0 } else { m1[0].len() };
    let x2 = m2.len();

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
    let y1 = if m1.is_empty() { 0 } else { m1[0].len() };
    let y2 = if m2.is_empty() { 0 } else { m2[0].len() };

    for i in 0..x1 {
        for j in 0..y2 {
            let mut sum = 0.0f32;
            for k in 0..y1 {
                sum += m1[i][k] * m2[k][j];
            }
            result[i][j] = sum;
        }
    }
}

pub fn algorithm3(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let x1 = m1.len();
    let y1 = if m1.is_empty() { 0 } else { m1[0].len() };
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
    // Algorithm 2 in the original C code is restricted to square matrices.
    let size = m1.len();
    if size == 0 {
        return;
    }

    // Build the transpose of m2 (named `b` in the C source) for
    // cache-friendly access.
    let mut b: Vec<Vec<f32>> = vec![vec![0.0f32; size]; size];
    for i in 0..size {
        for j in 0..size {
            b[i][j] = m2[j][i];
        }
    }

    // Zero-initialize the result.
    for i in 0..size {
        for j in 0..size {
            result[i][j] = 0.0;
        }
    }

    // Compute c[i][k] = sum_j m1[i][j] * b[k][j] = sum_j m1[i][j] * m2[j][k]
    for i in 0..size {
        for k in 0..size {
            let mut sum = 0.0f32;
            for j in 0..size {
                sum += m1[i][j] * b[k][j];
            }
            result[i][k] = sum;
        }
    }
}
