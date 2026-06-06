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
        let end = start + self.dim;
        Some(&mut self.data[start..end])
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
                "Matrix2D::set out of bounds".to_string(),
            ));
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
        return Err(AcesError::GenericError(
            "matrix2d_multiply: dimension mismatch".to_string(),
        ));
    }
    let dim = m.dim;
    let mut result = Matrix2D::new(dim);

    for i in 0..dim {
        for j in 0..dim {
            let mut acc: u64 = 0;
            for k in 0..dim {
                let a = m.get(i, k).unwrap_or(0);
                let b = invm.get(k, j).unwrap_or(0);
                acc = acc.wrapping_add(a.wrapping_mul(b));
            }
            result.set(i, j, acc % modulus)?;
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
    let target = if source == 0 {
        0
    } else {
        randrange(0, (source - 1) as u64) as usize
    };

    for row in 0..dim {
        let tmp = m.get(row, source).unwrap_or(0);
        let other = m.get(row, target).unwrap_or(0);
        m.set(row, source, other)?;
        m.set(row, target, tmp)?;
    }

    // Swap rows `source` and `target` in invm
    if source != target {
        // Need to do row swap without aliasing
        let row_size = dim;
        let s_start = source * row_size;
        let t_start = target * row_size;

        // Build temp copy
        let tmp_row: Vec<u64> = invm.data[s_start..s_start + row_size].to_vec();
        for k in 0..row_size {
            invm.data[s_start + k] = invm.data[t_start + k];
        }
        for k in 0..row_size {
            invm.data[t_start + k] = tmp_row[k];
        }
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
    let target = if source == 0 {
        0
    } else {
        randrange(0, (source - 1) as u64) as usize
    };

    for row in 0..dim {
        // m[row][target] = (m[row][target] + m[row][source] * scale) mod
        let cur = m.get(row, target).unwrap_or(0);
        let src = m.get(row, source).unwrap_or(0);
        let new_val = cur.wrapping_add(src.wrapping_mul(scale)) % modulus;
        m.set(row, target, new_val)?;

        // invm[source][row] = (invm[source][row] + invm[target][row] * (mod - scale)) mod
        let invm_src = invm.get(source, row).unwrap_or(0);
        let invm_tgt = invm.get(target, row).unwrap_or(0);
        let new_val = invm_src.wrapping_add(invm_tgt.wrapping_mul(modulus - scale)) % modulus;
        invm.set(source, row, new_val)?;
    }

    Ok(())
}

pub fn scale_transform(m: &mut Matrix2D, invm: &mut Matrix2D, modulus: u64) -> Result<()> {
    let dim = m.dim;
    let scale = randinverse(modulus);

    for idx in 0..(dim * dim) {
        m.data[idx] = m.data[idx].wrapping_mul(scale.first) % modulus;
        invm.data[idx] = invm.data[idx].wrapping_mul(scale.second) % modulus;
    }

    Ok(())
}

pub fn matrix2d_eye(m: &mut Matrix2D) -> Result<()> {
    let dim = m.dim;
    for i in 0..(dim * dim) {
        m.data[i] = 0;
    }
    for row in 0..dim {
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
    const TRANSFORM_COUNT: u64 = 3;

    matrix2d_eye(m)?;
    matrix2d_eye(invm)?;

    for _ in 0..iterations {
        let select = randrange(0, TRANSFORM_COUNT - 1);
        match select {
            0 => swap_transform(m, invm, modulus)?,
            1 => scale_transform(m, invm, modulus)?,
            2 => linear_mix_transform(m, invm, modulus)?,
            _ => {}
        }
    }

    Ok(())
}
