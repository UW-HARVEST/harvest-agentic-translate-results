use crate::error::{AcesError, Result};
pub type Coeff = i64;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Polynomial {
    pub coeffs: Vec<Coeff>,
}

/// Mirrors C semantics where `int64_t % uint64_t` is computed as unsigned.
/// The i64 value is reinterpreted as u64 (two's-complement wrap), then `% modulus`,
/// then converted back to i64 (the result fits in i64 when modulus <= i64::MAX).
fn rem_c(value: i64, modulus: u64) -> i64 {
    if modulus == 0 {
        return value;
    }
    ((value as u64) % modulus) as i64
}

/// Mirrors C semantics for signed mod (used by `poly_lshift`):
/// res = value % (int64_t)modulus, then if res < 0, res += modulus.
fn rem_signed_pos(value: i64, modulus: u64) -> i64 {
    if modulus == 0 {
        return value;
    }
    let m = modulus as i64;
    let r = value % m;
    if r < 0 {
        r + m
    } else {
        r
    }
}

impl Polynomial {
    pub fn new(coeffs: Vec<Coeff>) -> Self {
        Polynomial { coeffs }
    }
    pub fn degree(&self) -> usize {
        if self.coeffs.is_empty() {
            return 0;
        }
        let size = self.coeffs.len();
        for i in 0..size {
            if self.coeffs[i] != 0 {
                return size - i - 1;
            }
        }
        0
    }
    pub fn set_zero(&mut self) {
        for c in self.coeffs.iter_mut() {
            *c = 0;
        }
    }
    pub fn coef_sum(&self) -> Coeff {
        if self.coeffs.is_empty() {
            return 0;
        }
        let mut sum: i64 = 0;
        for &c in self.coeffs.iter() {
            sum = sum.wrapping_add(c);
        }
        sum
    }
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError("empty polynomial".to_string()));
        }
        let degree = self.degree();
        let size = self.coeffs.len();
        let idx = size - 1 - degree;
        let new_size = size - idx;
        for i in 0..new_size {
            self.coeffs[i] = rem_c(self.coeffs[i + idx], modulus);
        }
        self.coeffs.truncate(new_size);
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let result_size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let mut result = vec![0i64; result_size];

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = result_size - self.coeffs.len();
            let diff2 = result_size - other.coeffs.len();
            for i in 0..result_size {
                let v1 = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let v2 = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = rem_c(v1.wrapping_add(v2), modulus);
            }
        } else {
            for i in 0..self.coeffs.len() {
                result[i] = rem_c(self.coeffs[i].wrapping_add(other.coeffs[i]), modulus);
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let result_size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let mut result = vec![0i64; result_size];

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = result_size - self.coeffs.len();
            let diff2 = result_size - other.coeffs.len();
            for i in 0..result_size {
                let v1 = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let v2 = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = rem_c(v1.wrapping_sub(v2), modulus);
            }
        } else {
            for i in 0..self.coeffs.len() {
                result[i] = rem_c(self.coeffs[i].wrapping_sub(other.coeffs[i]), modulus);
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if self.coeffs.is_empty() || other.coeffs.is_empty() {
            return Err(AcesError::GenericError("empty polynomial".to_string()));
        }
        let size1 = self.coeffs.len();
        let size2 = other.coeffs.len();
        let result_size = size1 + size2 - 1;
        let mut result = vec![0i64; result_size];

        // Mirrors C: result[res_deg + i + j] += (poly1[i] * poly2[j]) % mod
        // res_deg = result_size - deg2 - deg1 - 1 = 0 here (since we sized to size1+size2-1)
        for i in 0..size1 {
            for j in 0..size2 {
                let prod = self.coeffs[i].wrapping_mul(other.coeffs[j]);
                let m = rem_c(prod, modulus);
                result[i + j] = result[i + j].wrapping_add(m);
            }
        }
        let mut poly = Polynomial { coeffs: result };
        poly.fit(modulus)?;
        Ok(poly)
    }
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if other.coeffs.is_empty() || other.coeffs[0] != 1 {
            return Err(AcesError::GenericError(
                "leading coeff of divisor must be 1".to_string(),
            ));
        }

        let degree1 = self.degree();
        let degree2 = other.degree();

        if degree1 < degree2 {
            return Err(AcesError::GenericError(
                "degree of dividend less than divisor".to_string(),
            ));
        }

        let a_d = self.coeffs[0];
        let mut result = vec![0i64; self.coeffs.len()];

        for i in 0..self.coeffs.len() {
            if i < other.coeffs.len() {
                let val = self.coeffs[i].wrapping_sub(other.coeffs[i].wrapping_mul(a_d));
                result[i] = rem_signed_pos(val, modulus);
            } else {
                result[i] = self.coeffs[i];
            }
        }

        let mut poly = Polynomial { coeffs: result };
        poly.fit(modulus)?;
        Ok(poly)
    }
    pub fn poly_mod(&mut self, divisor: &Polynomial, modulus: u64) -> Result<()> {
        loop {
            match self.lshift(divisor, modulus) {
                Ok(p) => {
                    *self = p;
                }
                Err(_) => break,
            }
        }
        Ok(())
    }
    pub fn sub_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        // C: result->coeffs[i] = (poly->coeffs[i] - scaler) % mod
        // The subtraction promotes int64_t to uint64_t.
        let mut result = Vec::with_capacity(self.coeffs.len());
        for &c in self.coeffs.iter() {
            let diff = (c as u64).wrapping_sub(scaler);
            result.push(rem_c(diff as i64, modulus));
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let mut result = Vec::with_capacity(self.coeffs.len());
        for &c in self.coeffs.iter() {
            let sum = (c as u64).wrapping_add(scaler);
            result.push(rem_c(sum as i64, modulus));
        }
        Ok(Polynomial { coeffs: result })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyArray {
    pub polies: Vec<Polynomial>,
}
impl PolyArray {
    pub fn new(polies: Vec<Polynomial>) -> Self {
        PolyArray { polies }
    }
}
