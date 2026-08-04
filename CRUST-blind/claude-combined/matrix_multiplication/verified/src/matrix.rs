use rand::Rng;

pub fn generate_matrix(x: usize, y: usize, random: bool) -> Vec<Vec<f32>> {
    let mut matrix: Vec<Vec<f32>> = Vec::with_capacity(x);
    if !random {
        for i in 0..x {
            let mut row = vec![0.0f32; y];
            for j in 0..y {
                row[j] = (j + i * x) as f32;
            }
            matrix.push(row);
        }
    } else {
        let mut rng = rand::rng();
        for _ in 0..x {
            let mut row = vec![0.0f32; y];
            for j in 0..y {
                // Match the C `(float)rand()` style: produce a non-negative
                // integer value cast to float. We use any i32 value.
                let v: i32 = rng.random_range(0..i32::MAX);
                row[j] = v as f32;
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
    let y1 = if !m1.is_empty() { m1[0].len() } else { 0 };
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
    let y1 = if !m1.is_empty() { m1[0].len() } else { 0 };
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

pub fn algorithm3(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let x1 = m1.len();
    let y1 = if !m1.is_empty() { m1[0].len() } else { 0 };
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

pub fn algorithm2(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    // The C algorithm2 is restricted to square matrices whose size is a
    // multiple of 8. Logically, it computes the same thing as algorithm1
    // by first transposing m2 and then multiplying. We replicate that
    // behavior using a transpose followed by the standard multiply, which
    // produces the mathematically identical result.
    let size = m1.len();
    if size == 0 {
        return;
    }
    // Transpose m2 -> b
    let mut b: Vec<Vec<f32>> = vec![vec![0.0f32; size]; size];
    for i in 0..size {
        for j in 0..size {
            b[i][j] = m2[j][i];
        }
    }
    for i in 0..size {
        for j in 0..size {
            result[i][j] = 0.0;
        }
    }
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
