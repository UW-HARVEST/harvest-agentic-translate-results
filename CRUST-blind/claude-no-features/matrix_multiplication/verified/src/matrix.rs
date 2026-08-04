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
                let v: i32 = rng.random();
                row.push(v as f32);
            }
            matrix.push(row);
        }
    }
    matrix
}

pub fn free_matrix(matrix: &mut Vec<Vec<f32>>) -> i32 {
    matrix.clear();
    0
}

pub fn multiply(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>], method: i32) -> i32 {
    let y1 = if m1.is_empty() { 0 } else { m1[0].len() };
    let x2 = m2.len();
    if y1 != x2 {
        println!("The number of columns in the first matrix must be equal to the number of rows in the second matrix. ");
        return -1;
    }
    match method {
        1 => {
            algorithm1(m1, m2, result);
        }
        2 => {
            algorithm2(m1, m2, result);
        }
        3 => {
            algorithm3(m1, m2, result);
        }
        _ => {
            println!("Choose the correct method!");
            return -1;
        }
    }
    0
}

pub fn algorithm1(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let x1 = m1.len();
    let y1 = if m1.is_empty() { 0 } else { m1[0].len() };
    let y2 = if m2.is_empty() { 0 } else { m2[0].len() };
    for i in 0..x1 {
        for j in 0..y2 {
            result[i][j] = 0.0;
            for k in 0..y1 {
                result[i][j] += m1[i][k] * m2[k][j];
            }
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
    // The C algorithm2 is a SIMD-optimized matrix multiplication for square
    // matrices. It transposes m2 into b, then computes
    //   result[i][k] = sum over j of m1[i][j] * b[k][j]
    //               = sum over j of m1[i][j] * m2[j][k]
    // which is the standard matrix multiplication. Implement it that way in
    // safe, portable Rust.
    let size = m1.len();

    // Transpose m2 into b (size x size).
    let mut b: Vec<Vec<f32>> = vec![vec![0.0f32; size]; size];
    for i in 0..size {
        for j in 0..size {
            b[i][j] = m2[j][i];
            result[i][j] = 0.0;
        }
    }

    for i in 0..size {
        for k in 0..size {
            let mut acc: f32 = 0.0;
            for j in 0..size {
                acc += m1[i][j] * b[k][j];
            }
            result[i][k] = acc;
        }
    }
}
