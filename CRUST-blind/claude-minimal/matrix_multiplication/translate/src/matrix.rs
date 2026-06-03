use rand::Rng;

pub fn generate_matrix(x: usize, y: usize, random: bool) -> Vec<Vec<f32>> {
    let mut matrix: Vec<Vec<f32>> = Vec::with_capacity(x);
    if !random {
        // Match C: (*matrix)[i] is allocated with size x (not y) when not random,
        // and values are (j + i * x) for j in 0..y.
        for i in 0..x {
            let mut row: Vec<f32> = Vec::with_capacity(x);
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
                // Match C's `(ARRAY_TYPE)rand()` which yields a non-negative int cast to float.
                let v: i32 = rng.random_range(0..i32::MAX);
                row.push(v as f32);
            }
            matrix.push(row);
        }
    }
    matrix
}

pub fn free_matrix(matrix: &mut Vec<Vec<f32>>) -> i32 {
    // In Rust, memory is freed automatically on drop; emulate the C
    // function by clearing the outer vector (which drops each inner Vec).
    for row in matrix.iter_mut() {
        row.clear();
        row.shrink_to_fit();
    }
    matrix.clear();
    matrix.shrink_to_fit();
    0
}

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

pub fn algorithm2(a: &[Vec<f32>], bb: &[Vec<f32>], c: &mut [Vec<f32>]) {
    // Algorithm 2 from c_src/matrix.c assumes both matrices are square with
    // identical dimensions. The C version transposes `bb` into `b` and then
    // performs an AVX-accelerated multiplication, processing the inner
    // dimension in blocks of 8. The Rust port reproduces the same algorithm
    // (transpose + blocked multiplication) without explicit SIMD intrinsics.
    let size = a.len();

    // Transpose bb into b and zero-initialize c.
    let mut b: Vec<Vec<f32>> = vec![vec![0.0f32; size]; size];
    for i in 0..size {
        for j in 0..size {
            b[i][j] = bb[j][i];
            c[i][j] = 0.0;
        }
    }

    // Process the inner index `j` in blocks of 8, matching the AVX width.
    let block = 8usize;
    for i in 0..size {
        let mut j = 0usize;
        while j + block <= size {
            for k in 0..size {
                let mut sum = 0.0f32;
                for m in 0..block {
                    sum += a[i][j + m] * b[k][j + m];
                }
                c[i][k] += sum;
            }
            j += block;
        }
        // Handle any remainder if size is not a multiple of 8.
        if j < size {
            for k in 0..size {
                let mut sum = 0.0f32;
                for m in j..size {
                    sum += a[i][m] * b[k][m];
                }
                c[i][k] += sum;
            }
        }
    }
}
