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
        let start = row * self.dim;
        let dim = self.dim;
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
                "matrix index out of range".to_string(),
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
        let data = (0..size).map(|_| Matrix2D::new(dim)).collect();
        Matrix3D { data }
    }
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Matrix2D> {
        self.data.get_mut(idx)
    }
}
pub fn matrix2d_multiply(m: &Matrix2D, invm: &Matrix2D, modulus: u64) -> Result<Matrix2D> {
    if m.dim != invm.dim {
        return Err(AcesError::GenericError(
            "matrices must have matching dimensions".to_string(),
        ));
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
    if m.dim != invm.dim {
        return Err(AcesError::GenericError(
            "matrices must have matching dimensions".to_string(),
        ));
    }
    let dim = m.dim;
    if dim < 2 {
        return Ok(());
    }
    let source = randrange(1, (dim - 1) as u64) as usize;
    let target = randrange(0, (source - 1) as u64) as usize;

    // Swap columns `source` and `target` in `m`.
    for row in 0..dim {
        let s_idx = row * dim + source;
        let t_idx = row * dim + target;
        m.data.swap(s_idx, t_idx);
    }

    // Swap rows `source` and `target` in `invm`.
    for col in 0..dim {
        let s_idx = source * dim + col;
        let t_idx = target * dim + col;
        invm.data.swap(s_idx, t_idx);
    }

    Ok(())
}
pub fn linear_mix_transform(m: &mut Matrix2D, invm: &mut Matrix2D, modulus: u64) -> Result<()> {
    if m.dim != invm.dim {
        return Err(AcesError::GenericError(
            "matrices must have matching dimensions".to_string(),
        ));
    }
    let dim = m.dim;
    if dim < 2 {
        return Ok(());
    }
    let scale = randrange(1, modulus - 1);
    let source = randrange(1, (dim - 1) as u64) as usize;
    let target = randrange(0, (source - 1) as u64) as usize;

    for row in 0..dim {
        let m_t = m.data[row * dim + target];
        let m_s = m.data[row * dim + source];
        m.data[row * dim + target] = (m_t.wrapping_add(m_s.wrapping_mul(scale))) % modulus;

        let inv_s = invm.data[source * dim + row];
        let inv_t = invm.data[target * dim + row];
        invm.data[source * dim + row] =
            (inv_s.wrapping_add(inv_t.wrapping_mul(modulus - scale))) % modulus;
    }
    Ok(())
}
pub fn scale_transform(m: &mut Matrix2D, invm: &mut Matrix2D, modulus: u64) -> Result<()> {
    if m.dim != invm.dim {
        return Err(AcesError::GenericError(
            "matrices must have matching dimensions".to_string(),
        ));
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
