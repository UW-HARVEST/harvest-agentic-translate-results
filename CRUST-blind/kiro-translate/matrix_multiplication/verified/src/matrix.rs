pub fn generate_matrix(x: usize, y: usize, random: bool) -> Vec<Vec<f32>> {
    if random {
        use rand::Rng;
        let mut rng = rand::rng();
        (0..x).map(|_| (0..y).map(|_| rng.random::<i32>() as f32).collect()).collect()
    } else {
        (0..x).map(|i| (0..y).map(|j| (j + i * x) as f32).collect()).collect()
    }
}
pub fn free_matrix(matrix: &mut Vec<Vec<f32>>) -> i32 {
    matrix.clear();
    0
}
pub fn multiply(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>], method: i32) -> i32 {
    let x1 = m1.len();
    let y1 = if x1 > 0 { m1[0].len() } else { 0 };
    let x2 = m2.len();
    let _y2 = if x2 > 0 { m2[0].len() } else { 0 };
    if y1 != x2 {
        println!("The number of columns in the first matrix must be equal to the number of rows in the second matrix. ");
        return -1;
    }
    match method {
        1 => algorithm1(m1, m2, result),
        2 => algorithm2(m1, m2, result),
        3 => algorithm3(m1, m2, result),
        _ => {
            println!("Choose the correct method!");
            return -1;
        }
    }
    0
}
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
pub fn algorithm2(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let size = m1.len();
    // Transpose m2 into b
    let mut b = vec![vec![0.0f32; size]; size];
    for i in 0..size {
        for j in 0..size {
            b[i][j] = m2[j][i];
            result[i][j] = 0.0;
        }
    }
    // Multiply using dot products: result[i][k] = sum_j(m1[i][j] * b[k][j])
    for i in 0..size {
        for k in 0..size {
            let mut sum = 0.0f32;
            for j in 0..size {
                sum += m1[i][j] * b[k][j];
            }
            result[i][k] += sum;
        }
    }
}
