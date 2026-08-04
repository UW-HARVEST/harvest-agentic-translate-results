use crate::error::{AcesError, Result};

pub type Coeff = i64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Polynomial {
    pub coeffs: Vec<Coeff>,
}
impl Polynomial {
    pub fn new(coeffs: Vec<Coeff>) -> Self {
        Polynomial { coeffs }
    }
    pub fn degree(&self) -> usize {
        if self.coeffs.is_empty() {
            return 0;
        }
        for i in 0..self.coeffs.len() {
            if self.coeffs[i] != 0 {
                return self.coeffs.len() - i - 1;
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
        self.coeffs.iter().fold(0i64, |a, b| a.wrapping_add(*b))
    }
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError("empty polynomial".into()));
        }
        let degree = self.degree();
        let size = self.coeffs.len();
        let idx = size - 1 - degree;
        // Shift left by idx to drop the leading zeros and apply modulus.
        let m = modulus as i64;
        for i in 0..(size - idx) {
            self.coeffs[i] = self.coeffs[i + idx] % m;
        }
        // Truncate.
        self.coeffs.truncate(size - idx);
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is 0".into()));
        }
        // Result size is max of two sizes (mirroring C semantics with result
        // size equal to max). We choose result = max(self,other).
        let size = if self.coeffs.len() > other.coeffs.len() {
            self.coeffs.len()
        } else {
            other.coeffs.len()
        };
        let m = modulus as i64;
        let mut result = vec![0i64; size];
        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = size - self.coeffs.len();
            let diff2 = size - other.coeffs.len();
            for i in 0..size {
                let a = if i >= diff1 {
                    self.coeffs[i - diff1]
                } else {
                    0
                };
                let b = if i >= diff2 {
                    other.coeffs[i - diff2]
                } else {
                    0
                };
                result[i] = (a + b) % m;
            }
        } else {
            for i in 0..size {
                result[i] = (self.coeffs[i] + other.coeffs[i]) % m;
            }
        }
        Ok(Polynomial::new(result))
    }
    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is 0".into()));
        }
        let size = if self.coeffs.len() > other.coeffs.len() {
            self.coeffs.len()
        } else {
            other.coeffs.len()
        };
        let m = modulus as i64;
        let mut result = vec![0i64; size];
        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = size - self.coeffs.len();
            let diff2 = size - other.coeffs.len();
            for i in 0..size {
                let a = if i >= diff1 {
                    self.coeffs[i - diff1]
                } else {
                    0
                };
                let b = if i >= diff2 {
                    other.coeffs[i - diff2]
                } else {
                    0
                };
                result[i] = (a - b) % m;
            }
        } else {
            for i in 0..size {
                result[i] = (self.coeffs[i] - other.coeffs[i]) % m;
            }
        }
        Ok(Polynomial::new(result))
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        // The C code expects result.size to be large enough — at least
        // self.size + other.size - 1. We follow that and call fit() at the end.
        let m = modulus as i64;
        let size_needed = self.coeffs.len() + other.coeffs.len() - 1;
        let mut result = vec![0i64; size_needed];
        // res_deg in C: result->size - deg2 - deg1 - 1
        // Here result->size == size_needed == deg1 + deg2 + 1, so res_deg == 0.
        let res_deg = 0usize;
        for i in 0..self.coeffs.len() {
            for j in 0..other.coeffs.len() {
                result[res_deg + i + j] += (self.coeffs[i] * other.coeffs[j]) % m;
            }
        }
        let mut p = Polynomial::new(result);
        p.fit(modulus)?;
        Ok(p)
    }
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if other.coeffs.is_empty() || other.coeffs[0] != 1 {
            return Err(AcesError::GenericError(
                "leading coefficient of divisor must be 1".into(),
            ));
        }
        let degree1 = self.degree();
        let degree2 = other.degree();
        if degree1 < degree2 {
            return Err(AcesError::GenericError("degree(poly1) < degree(poly2)".into()));
        }
        let a_d = self.coeffs[0];
        let m = modulus as i64;
        let size = self.coeffs.len();
        let mut result = vec![0i64; size];
        for i in 0..size {
            if i < other.coeffs.len() {
                let res = (self.coeffs[i] - other.coeffs[i] * a_d) % m;
                result[i] = if res < 0 { res + m } else { res };
            } else {
                result[i] = self.coeffs[i];
            }
        }
        let mut out = Polynomial::new(result);
        out.fit(modulus)?;
        Ok(out)
    }
    pub fn poly_mod(&mut self, divisor: &Polynomial, modulus: u64) -> Result<()> {
        // Repeatedly lshift while it succeeds.
        loop {
            match self.lshift(divisor, modulus) {
                Ok(new_self) => {
                    *self = new_self;
                }
                Err(_) => break,
            }
        }
        Ok(())
    }
    pub fn sub_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        let s = scaler as i64;
        let result: Vec<Coeff> = self.coeffs.iter().map(|c| (c - s) % m).collect();
        Ok(Polynomial::new(result))
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        let s = scaler as i64;
        let result: Vec<Coeff> = self.coeffs.iter().map(|c| (c + s) % m).collect();
        Ok(Polynomial::new(result))
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
