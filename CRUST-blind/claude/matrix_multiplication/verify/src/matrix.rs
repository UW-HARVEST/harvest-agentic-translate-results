use rand::Rng;

pub fn generate_matrix(x: usize, y: usize, random: bool) -> Vec<Vec<f32>> {
    let mut matrix: Vec<Vec<f32>> = Vec::with_capacity(x);
    if !random {
        // Match C behavior: matrix[i][j] = (j + i * x) as f32
        // Row length matches the number of columns actually written.
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
                // Mirror C's (float)rand() — a non-negative pseudo-random integer cast to float.
                let v: i32 = rng.random_range(0..=i32::MAX);
                row.push(v as f32);
            }
            matrix.push(row);
        }
    }
    matrix
}

pub fn free_matrix(matrix: &mut Vec<Vec<f32>>) -> i32 {
    // In Rust, dropping the inner Vecs deallocates their memory. Clearing the
    // outer Vec mirrors the C semantics of releasing all rows and resetting
    // the matrix to an empty state.
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
            let mut acc: f32 = 0.0;
            for k in 0..y1 {
                acc += m1[i][k] * m2[k][j];
            }
            result[i][j] = acc;
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
    // The C version operates only on square matrices and uses AVX intrinsics
    // to vectorize the inner loop after transposing the right-hand operand.
    // Mathematically the operation is plain square matrix multiplication.
    let size = m1.len();

    // Build a transposed copy of m2: b[i][j] = m2[j][i]
    let mut b: Vec<Vec<f32>> = Vec::with_capacity(size);
    for i in 0..size {
        let mut row: Vec<f32> = Vec::with_capacity(size);
        for j in 0..size {
            row.push(m2[j][i]);
        }
        b.push(row);
    }

    // Initialize result rows to zero, then accumulate dot products.
    for i in 0..size {
        for k in 0..size {
            result[i][k] = 0.0;
        }
        for k in 0..size {
            let mut acc: f32 = 0.0;
            for j in 0..size {
                acc += m1[i][j] * b[k][j];
            }
            result[i][k] += acc;
        }
    }
}
