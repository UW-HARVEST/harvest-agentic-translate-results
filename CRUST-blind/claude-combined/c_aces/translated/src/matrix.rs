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
        let dim = self.dim;
        Some(&self.data[row * dim..(row + 1) * dim])
    }
    pub fn row_mut(&mut self, row: usize) -> Option<&mut [u64]> {
        if row >= self.dim {
            return None;
        }
        let dim = self.dim;
        Some(&mut self.data[row * dim..(row + 1) * dim])
    }
    pub fn get(&self, row: usize, col: usize) -> Option<u64> {
        if row >= self.dim || col >= self.dim {
            return None;
        }
        let dim = self.dim;
        Some(self.data[row * dim + col])
    }
    pub fn set(&mut self, row: usize, col: usize, value: u64) -> Result<()> {
        if row >= self.dim || col >= self.dim {
            return Err(AcesError::GenericError("index out of bounds".to_string()));
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
        self.data.get_mut(idx)
    }
}
pub fn matrix2d_multiply(m: &Matrix2D, invm: &Matrix2D, modulus: u64) -> Result<Matrix2D> {
    if m.dim != invm.dim {
        return Err(AcesError::GenericError("dim mismatch".to_string()));
    }
    let dim = m.dim;
    let mut result = Matrix2D::new(dim);
    for i in 0..dim {
        for j in 0..dim {
            let mut sum: u64 = 0;
            for k in 0..dim {
                sum = sum.wrapping_add(
                    m.data[i * dim + k].wrapping_mul(invm.data[k * dim + j]),
                );
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
    // randrange(0, source - 1)
    let target = randrange(0, (source - 1) as u64) as usize;

    // Swap columns `source` and `target` of m.
    for row in 0..dim {
        let a = m.data[row * dim + source];
        let b = m.data[row * dim + target];
        m.data[row * dim + source] = b;
        m.data[row * dim + target] = a;
    }

    // Swap rows `source` and `target` of invm.
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
    if dim < 2 {
        return Ok(());
    }
    let scale = randrange(1, modulus - 1);
    let source = randrange(1, (dim - 1) as u64) as usize;
    let target = randrange(0, (source - 1) as u64) as usize;

    for row in 0..dim {
        let m_target = m.data[row * dim + target];
        let m_source = m.data[row * dim + source];
        m.data[row * dim + target] =
            (m_target.wrapping_add(m_source.wrapping_mul(scale))) % modulus;

        let inv_source = invm.data[source * dim + row];
        let inv_target = invm.data[target * dim + row];
        invm.data[source * dim + row] = (inv_source
            .wrapping_add(inv_target.wrapping_mul(modulus - scale)))
            % modulus;
    }
    Ok(())
}
pub fn scale_transform(m: &mut Matrix2D, invm: &mut Matrix2D, modulus: u64) -> Result<()> {
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
    for v in m.data.iter_mut() {
        *v = 0;
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

    const TRANSFORM_COUNT: u64 = 3;
    for _ in 0..iterations {
        let select = randrange(0, TRANSFORM_COUNT - 1);
        match select {
            0 => swap_transform(m, invm, modulus)?,
            1 => scale_transform(m, invm, modulus)?,
            2 => linear_mix_transform(m, invm, modulus)?,
            _ => unreachable!(),
        }
    }
    Ok(())
}
