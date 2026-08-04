pub fn generate_matrix(x: usize, y: usize, random: bool) -> Vec<Vec<f32>> {
    let mut matrix = Vec::with_capacity(x);
    for i in 0..x {
        let mut row = Vec::with_capacity(y);
        for j in 0..y {
            let value = if random {
                rand::random::<u32>() as f32
            } else {
                (j + i * x) as f32
            };
            row.push(value);
        }
        matrix.push(row);
    }
    matrix
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
    for (i, result_row) in result.iter_mut().enumerate() {
        let Some(m1_row) = m1.get(i) else {
            result_row.fill(0.0);
            continue;
        };

        for (j, cell) in result_row.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (k, lhs) in m1_row.iter().enumerate() {
                if let Some(rhs_row) = m2.get(k) {
                    sum += *lhs * rhs_row.get(j).copied().unwrap_or(0.0);
                }
            }
            *cell = sum;
        }
    }
}
pub fn algorithm3(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    for row in result.iter_mut() {
        row.fill(0.0);
    }

    for (i, m1_row) in m1.iter().enumerate() {
        let Some(result_row) = result.get_mut(i) else {
            break;
        };

        for (k, lhs) in m1_row.iter().enumerate() {
            let Some(m2_row) = m2.get(k) else {
                continue;
            };

            for (j, cell) in result_row.iter_mut().enumerate() {
                *cell += *lhs * m2_row.get(j).copied().unwrap_or(0.0);
            }
        }
    }
}
pub fn algorithm2(m1: &[Vec<f32>], m2: &[Vec<f32>], result: &mut [Vec<f32>]) {
    let size = m1.len().min(m2.len()).min(result.len());

    let mut transposed = vec![vec![0.0; size]; size];
    for i in 0..size {
        for j in 0..size {
            transposed[i][j] = m2.get(j).and_then(|row| row.get(i)).copied().unwrap_or(0.0);
        }
    }

    for i in 0..size {
        if let Some(result_row) = result.get_mut(i) {
            let width = result_row.len().min(size);
            for cell in result_row.iter_mut().take(width) {
                *cell = 0.0;
            }

            let lhs_row = &m1[i];
            for k in 0..width {
                let mut sum = 0.0;
                for j in 0..size {
                    sum += lhs_row.get(j).copied().unwrap_or(0.0) * transposed[k][j];
                }
                result_row[k] += sum;
            }
        }
    }
}
