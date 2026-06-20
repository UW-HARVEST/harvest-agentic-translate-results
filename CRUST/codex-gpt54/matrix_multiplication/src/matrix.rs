pub fn generate_matrix(x: usize, y: usize, random: bool) -> Vec<Vec<f32>> {
    if random {
        let mut rng = rand::rng();
        (0..x)
            .map(|_| (0..y).map(|_| rand::Rng::random(&mut rng)).collect())
            .collect()
    } else {
        (0..x)
            .map(|i| (0..y).map(|j| (j + i * x) as f32).collect())
            .collect()
    }
}
pub fn free_matrix(matrix: &mut Vec<Vec<f32>>) -> i32 {
    matrix.clear();
    0
}
pub fn multiply(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>], method: i32) -> i32 {
    let y1 = m1.first().map_or(0, Vec::len);
    let x2 = m2.len();
    if y1 != x2 {
        println!(
            "The number of columns in the first matrix must be equal to the number of rows in the second matrix. "
        );
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
    let y2 = m2.first().map_or(0, Vec::len);

    for (i, row1) in m1.iter().enumerate() {
        for j in 0..y2 {
            let mut sum = 0.0;
            for (k, value) in row1.iter().enumerate() {
                sum += *value * m2[k][j];
            }
            result[i][j] = sum;
        }
    }
}
pub fn algorithm3(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let y2 = m2.first().map_or(0, Vec::len);

    for row in result.iter_mut().take(m1.len()) {
        for value in row.iter_mut().take(y2) {
            *value = 0.0;
        }
    }

    for (i, row1) in m1.iter().enumerate() {
        for (k, value) in row1.iter().enumerate() {
            for j in 0..y2 {
                result[i][j] += *value * m2[k][j];
            }
        }
    }
}
pub fn algorithm2(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let size = m2.first().map_or(0, Vec::len);
    let transposed: Vec<Vec<f32>> = (0..size)
        .map(|i| (0..m2.len()).map(|j| m2[j][i]).collect())
        .collect();

    for (i, row1) in m1.iter().enumerate() {
        for (j, row_t) in transposed.iter().enumerate() {
            result[i][j] = row1.iter().zip(row_t.iter()).map(|(a, b)| a * b).sum();
        }
    }
}
