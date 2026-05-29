use crate::error::{AcesError, Result};
pub type Coeff = i64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Polynomial {
    pub coeffs: Vec<Coeff>,
}

fn modulo(value: i64, modulus: u64) -> i64 {
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
        for c in &self.coeffs {
            sum = sum.wrapping_add(*c);
        }
        sum
    }
    /// Adjusts coefficients within `modulus` and removes leading zeros from the high
    /// end (left of the array).
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError("polynomial is empty".to_string()));
        }
        let size = self.coeffs.len();
        let degree = self.degree();
        let idx = size - 1 - degree;
        // Shift left by idx and apply modulus.
        for i in 0..(size - idx) {
            let val = self.coeffs[i + idx];
            self.coeffs[i] = if modulus == 0 {
                val
            } else {
                let m = modulus as i64;
                let r = val % m;
                r
            };
        }
        self.coeffs.truncate(size - idx);
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let mut result = vec![0i64; size];
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        for i in 0..size {
            let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
            let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
            result[i] = modulo(a + b, modulus);
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let mut result = vec![0i64; size];
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        for i in 0..size {
            let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
            let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
            result[i] = modulo(a - b, modulus);
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        if self.coeffs.is_empty() || other.coeffs.is_empty() {
            return Ok(Polynomial { coeffs: vec![] });
        }
        let size = self.coeffs.len() + other.coeffs.len() - 1;
        let mut result = vec![0i64; size];
        let m = modulus as i64;
        for i in 0..self.coeffs.len() {
            for j in 0..other.coeffs.len() {
                let prod = (self.coeffs[i].wrapping_mul(other.coeffs[j])) % m;
                result[i + j] = result[i + j].wrapping_add(prod);
            }
        }
        // Apply modulus and fit to remove leading zeros.
        let mut poly = Polynomial { coeffs: result };
        poly.fit(modulus)?;
        Ok(poly)
    }
    /// Performs a single left-shift step: subtracts the divisor (scaled to the
    /// leading coefficient of self) and shifts left by one term. The divisor must
    /// be monic (leading coefficient 1).
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        if other.coeffs.is_empty() || other.coeffs[0] != 1 {
            return Err(AcesError::GenericError(
                "divisor must be monic".to_string(),
            ));
        }
        let degree1 = self.degree();
        let degree2 = other.degree();
        if degree1 < degree2 {
            return Err(AcesError::GenericError(
                "self degree less than divisor degree".to_string(),
            ));
        }
        let a_d = self.coeffs[0];
        let m = modulus as i64;
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            if i < other.coeffs.len() {
                let res = (self.coeffs[i].wrapping_sub(other.coeffs[i].wrapping_mul(a_d))) % m;
                result[i] = if res < 0 { res + m } else { res };
            } else {
                result[i] = self.coeffs[i];
            }
        }
        let mut poly = Polynomial { coeffs: result };
        poly.fit(modulus)?;
        Ok(poly)
    }
    /// Compute the modulus (remainder) of self by `divisor`, in place.
    pub fn poly_mod(&mut self, divisor: &Polynomial, modulus: u64) -> Result<()> {
        loop {
            match self.lshift(divisor, modulus) {
                Ok(new_poly) => {
                    *self = new_poly;
                }
                Err(_) => break,
            }
        }
        Ok(())
    }
    pub fn sub_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let mut result = Vec::with_capacity(self.coeffs.len());
        for c in &self.coeffs {
            result.push(modulo(c - scaler as i64, modulus));
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let mut result = Vec::with_capacity(self.coeffs.len());
        for c in &self.coeffs {
            result.push(modulo(c + scaler as i64, modulus));
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
