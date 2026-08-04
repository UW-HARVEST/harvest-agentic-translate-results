use crate::common::{randinverse, randrange};
use crate::error::{AcesError, Result};

#[derive(Debug, Clone)]
pub struct Matrix2D {
    pub dim: usize,
    pub data: Vec<u64>,
}
impl Matrix2D {
    pub fn new(dim: usize) -> Self {
        Matrix2D {
            dim,
            data: vec![0u64; dim * dim],
        }
    }
    pub fn row(&self, row: usize) -> Option<&[u64]> {
        if row >= self.dim {
            return None;
        }
        let start = row * self.dim;
        Some(&self.data[start..start + self.dim])
    }
    pub fn row_mut(&mut self, row: usize) -> Option<&mut [u64]> {
        if row >= self.dim {
            return None;
        }
        let dim = self.dim;
        let start = row * dim;
        Some(&mut self.data[start..start + dim])
    }
    pub fn get(&self, row: usize, col: usize) -> Option<u64> {
        if row >= self.dim || col >= self.dim {
            return None;
        }
        Some(self.data[row * self.dim + col])
    }
    pub fn set(&mut self, row: usize, col: usize, value: u64) -> Result<()> {
        if row >= self.dim || col >= self.dim {
            return Err(AcesError::GenericError(
                "matrix index out of bounds".to_string(),
            ));
        }
        let dim = self.dim;
        self.data[row * dim + col] = value;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct Matrix3D {
    pub data: Vec<Matrix2D>,
}
impl Matrix3D {
    pub fn new(size: usize, dim: usize) -> Self {
        let mut data = Vec::with_capacity(size);
        for _ in 0..size {
            data.push(Matrix2D::new(dim));
        }
        Matrix3D { data }
    }
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Matrix2D> {
        if idx >= self.data.len() {
            return None;
        }
        Some(&mut self.data[idx])
    }
}
pub fn matrix2d_multiply(m: &Matrix2D, invm: &Matrix2D, modulus: u64) -> Result<Matrix2D> {
    if m.dim != invm.dim {
        return Err(AcesError::GenericError(
            "matrix dimensions do not match".to_string(),
        ));
    }
    if modulus == 0 {
        return Err(AcesError::GenericError("modulus is zero".to_string()));
    }
    let dim = m.dim;
    let mut result = Matrix2D::new(dim);
    for i in 0..dim {
        for j in 0..dim {
            let mut sum: u64 = 0;
            for k in 0..dim {
                let a = m.data[i * dim + k];
                let b = invm.data[k * dim + j];
                sum = sum.wrapping_add(a.wrapping_mul(b));
            }
            result.data[i * dim + j] = sum % modulus;
        }
    }
    Ok(result)
}
pub fn swap_transform(m: &mut Matrix2D, invm: &mut Matrix2D, _modulus: u64) -> Result<()> {
    let dim = m.dim;
    if dim < 2 {
        return Ok(());
    }
    let source = randrange(1, (dim - 1) as u64) as usize;
    let target = randrange(0, (source - 1) as u64) as usize;

    // Swap columns `source` and `target` in m.
    for row in 0..dim {
        let a = m.data[row * dim + source];
        let b = m.data[row * dim + target];
        m.data[row * dim + source] = b;
        m.data[row * dim + target] = a;
    }
    // Swap rows `source` and `target` in invm.
    for col in 0..dim {
        let a = invm.data[source * dim + col];
        let b = invm.data[target * dim + col];
        invm.data[source * dim + col] = b;
        invm.data[target * dim + col] = a;
    }
    Ok(())
}
pub fn linear_mix_transform(m: &mut Matrix2D, invm: &mut Matrix2D, modulus: u64) -> Result<()> {
    let dim = m.dim;
    if dim < 2 || modulus < 2 {
        return Ok(());
    }
    let scale = randrange(1, modulus - 1);
    let source = randrange(1, (dim - 1) as u64) as usize;
    let target = randrange(0, (source - 1) as u64) as usize;

    for row in 0..dim {
        // m[row][target] = (m[row][target] + m[row][source] * scale) mod
        let mt = m.data[row * dim + target];
        let ms = m.data[row * dim + source];
        m.data[row * dim + target] =
            (mt.wrapping_add(ms.wrapping_mul(scale))) % modulus;

        // invm[source][row] = (invm[source][row] + invm[target][row] * (mod - scale)) mod
        let is = invm.data[source * dim + row];
        let it = invm.data[target * dim + row];
        invm.data[source * dim + row] =
            (is.wrapping_add(it.wrapping_mul(modulus - scale))) % modulus;
    }
    Ok(())
}
pub fn scale_transform(m: &mut Matrix2D, invm: &mut Matrix2D, modulus: u64) -> Result<()> {
    if modulus < 3 {
        return Ok(());
    }
    let dim = m.dim;
    let scale = randinverse(modulus);
    for idx in 0..(dim * dim) {
        m.data[idx] = (m.data[idx].wrapping_mul(scale.first)) % modulus;
        invm.data[idx] = (invm.data[idx].wrapping_mul(scale.second)) % modulus;
    }
    Ok(())
}
pub fn matrix2d_eye(m: &mut Matrix2D) -> Result<()> {
    let dim = m.dim;
    for cell in m.data.iter_mut() {
        *cell = 0;
    }
    for row in 0..dim {
        m.data[row * dim + row] = 1;
    }
    Ok(())
}
pub fn fill_random_invertible_pairs(
    m: &mut Matrix2D,
    invm: &mut Matrix2D,
    modulus: u64,
    iterations: usize,
) -> Result<()> {
    matrix2d_eye(m)?;
    matrix2d_eye(invm)?;

    for _ in 0..iterations {
        let select = randrange(0, 2);
        match select {
            0 => swap_transform(m, invm, modulus)?,
            1 => scale_transform(m, invm, modulus)?,
            _ => linear_mix_transform(m, invm, modulus)?,
        }
    }
    Ok(())
}
