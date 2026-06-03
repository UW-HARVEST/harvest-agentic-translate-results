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
        let mut sum: Coeff = 0;
        for &c in &self.coeffs {
            sum += c;
        }
        sum
    }
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError("empty polynomial".to_string()));
        }
        let degree = self.degree();
        let idx = self.coeffs.len() - 1 - degree;
        let len = self.coeffs.len();
        for i in 0..(len - idx) {
            let v = self.coeffs[i + idx];
            let m = modulus as i64;
            // The C code uses unsigned modulus on Coeff (int64_t). Mimic with rem_euclid for non-negative,
            // but since C just uses % which could be negative, replicate with `%` semantics.
            let r = v % m;
            self.coeffs[i] = r;
        }
        self.coeffs.truncate(len - idx);
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let m = modulus as i64;
        let result_size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let mut result = vec![0 as Coeff; result_size];

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = result_size - self.coeffs.len();
            let diff2 = result_size - other.coeffs.len();
            for i in 0..result_size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = (a + b) % m;
            }
        } else {
            for i in 0..result_size {
                result[i] = (self.coeffs[i] + other.coeffs[i]) % m;
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let m = modulus as i64;
        let result_size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let mut result = vec![0 as Coeff; result_size];

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = result_size - self.coeffs.len();
            let diff2 = result_size - other.coeffs.len();
            for i in 0..result_size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = (a - b) % m;
            }
        } else {
            for i in 0..result_size {
                result[i] = (self.coeffs[i] - other.coeffs[i]) % m;
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        let total = self.coeffs.len() + other.coeffs.len() - 1;
        // Match C: result has size from poly_mul behavior. The C version requires result->size to be
        // poly1->size + poly2->size - 1 typically, but since callers pass larger buffers it just uses
        // res_deg offset. Here we'll create a result whose size is exactly total then poly_fit it.
        let mut result_coeffs = vec![0 as Coeff; total];
        // Effectively no offset since total = deg1+deg2+1 means res_deg = 0.
        for i in 0..self.coeffs.len() {
            for j in 0..other.coeffs.len() {
                result_coeffs[i + j] += (self.coeffs[i] * other.coeffs[j]) % m;
            }
        }
        let mut result = Polynomial { coeffs: result_coeffs };
        result.fit(modulus)?;
        Ok(result)
    }
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if other.coeffs.is_empty() || other.coeffs[0] != 1 {
            return Err(AcesError::GenericError(
                "leading coefficient of divisor must be 1".to_string(),
            ));
        }
        let degree1 = self.degree();
        let degree2 = other.degree();
        if degree1 < degree2 {
            return Err(AcesError::GenericError(
                "degree of poly1 must be >= degree of poly2".to_string(),
            ));
        }
        let m = modulus as i64;
        let a_d = self.coeffs[0];
        let mut result = vec![0 as Coeff; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            if i < other.coeffs.len() {
                let res = (self.coeffs[i] - other.coeffs[i] * a_d) % m;
                result[i] = if res < 0 { res + m } else { res };
            } else {
                result[i] = self.coeffs[i];
            }
        }
        let mut poly = Polynomial { coeffs: result };
        poly.fit(modulus)?;
        Ok(poly)
    }
    pub fn poly_mod(&mut self, divisor: &Polynomial, modulus: u64) -> Result<()> {
        // Repeatedly call lshift on self until it fails (degree < divisor degree).
        loop {
            match self.lshift(divisor, modulus) {
                Ok(p) => *self = p,
                Err(_) => break,
            }
        }
        Ok(())
    }
    pub fn sub_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0 as Coeff; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i] - s) % m;
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0 as Coeff; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i] + s) % m;
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
