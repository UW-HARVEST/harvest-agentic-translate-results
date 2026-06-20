use crate::common::{randinverse, randrange};
use crate::error::{AcesError, Result};

fn generic_error(message: impl Into<String>) -> AcesError {
    AcesError::GenericError(message.into())
}

#[derive(Debug, Clone)]
pub struct Matrix2D {
    pub dim: usize,
    pub data: Vec<u64>,
}
impl Matrix2D {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            data: vec![0; dim * dim],
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
        Some(&mut self.data[start..start + self.dim])
    }
    pub fn get(&self, row: usize, col: usize) -> Option<u64> {
        if row >= self.dim || col >= self.dim {
            return None;
        }
        Some(self.data[row * self.dim + col])
    }
    pub fn set(&mut self, row: usize, col: usize, value: u64) -> Result<()> {
        if row >= self.dim || col >= self.dim {
            return Err(generic_error("matrix index out of bounds"));
        }
        self.data[row * self.dim + col] = value;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct Matrix3D {
    pub data: Vec<Matrix2D>,
}
impl Matrix3D {
    pub fn new(size: usize, dim: usize) -> Self {
        Self {
            data: (0..size).map(|_| Matrix2D::new(dim)).collect(),
        }
    }
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Matrix2D> {
        self.data.get_mut(idx)
    }
}
pub fn matrix2d_multiply(m: &Matrix2D, invm: &Matrix2D, modulus: u64) -> Result<Matrix2D> {
    if m.dim != invm.dim {
        return Err(generic_error("matrix dimension mismatch"));
    }

    let dim = m.dim;
    let mut result = Matrix2D::new(dim);
    for i in 0..dim {
        for j in 0..dim {
            let mut value = 0_u64;
            for k in 0..dim {
                value = value.wrapping_add(
                    m.get(i, k)
                        .ok_or_else(|| generic_error("matrix index out of bounds"))?
                        .wrapping_mul(
                            invm.get(k, j)
                                .ok_or_else(|| generic_error("matrix index out of bounds"))?,
                        ),
                );
            }
            result.set(i, j, value % modulus)?;
        }
    }

    Ok(result)
}
pub fn swap_transform(m: &mut Matrix2D, invm: &mut Matrix2D, modulus: u64) -> Result<()> {
    let _ = modulus;
    if m.dim != invm.dim || m.dim < 2 {
        return Err(generic_error("matrix dimension mismatch"));
    }

    let source = randrange(1, (m.dim - 1) as u64) as usize;
    let target = randrange(0, (source - 1) as u64) as usize;

    for row in 0..m.dim {
        let left = row * m.dim + source;
        let right = row * m.dim + target;
        m.data.swap(left, right);
    }

    for col in 0..invm.dim {
        let top = source * invm.dim + col;
        let bottom = target * invm.dim + col;
        invm.data.swap(top, bottom);
    }

    Ok(())
}
pub fn linear_mix_transform(m: &mut Matrix2D, invm: &mut Matrix2D, modulus: u64) -> Result<()> {
    if m.dim != invm.dim || m.dim < 2 {
        return Err(generic_error("matrix dimension mismatch"));
    }

    let scale = randrange(1, modulus - 1);
    let source = randrange(1, (m.dim - 1) as u64) as usize;
    let target = randrange(0, (source - 1) as u64) as usize;

    for row in 0..m.dim {
        let target_value = m.get(row, target).ok_or_else(|| generic_error("matrix index out of bounds"))?;
        let source_value = m.get(row, source).ok_or_else(|| generic_error("matrix index out of bounds"))?;
        m.set(
            row,
            target,
            target_value
                .wrapping_add(source_value.wrapping_mul(scale))
                % modulus,
        )?;

        let source_inv = invm
            .get(source, row)
            .ok_or_else(|| generic_error("matrix index out of bounds"))?;
        let target_inv = invm
            .get(target, row)
            .ok_or_else(|| generic_error("matrix index out of bounds"))?;
        invm.set(
            source,
            row,
            source_inv
                .wrapping_add(target_inv.wrapping_mul(modulus - scale))
                % modulus,
        )?;
    }

    Ok(())
}
pub fn scale_transform(m: &mut Matrix2D, invm: &mut Matrix2D, modulus: u64) -> Result<()> {
    if m.dim != invm.dim {
        return Err(generic_error("matrix dimension mismatch"));
    }

    let scale = randinverse(modulus);
    for idx in 0..(m.dim * m.dim) {
        m.data[idx] = m.data[idx].wrapping_mul(scale.first) % modulus;
        invm.data[idx] = invm.data[idx].wrapping_mul(scale.second) % modulus;
    }
    Ok(())
}
pub fn matrix2d_eye(m: &mut Matrix2D) -> Result<()> {
    m.data.fill(0);
    for row in 0..m.dim {
        m.set(row, row, 1)?;
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
        match randrange(0, 2) {
            0 => swap_transform(m, invm, modulus)?,
            1 => scale_transform(m, invm, modulus)?,
            _ => linear_mix_transform(m, invm, modulus)?,
        }
    }

    Ok(())
}
