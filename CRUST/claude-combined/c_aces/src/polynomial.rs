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
        let mut sum: Coeff = 0;
        for &c in &self.coeffs {
            sum = sum.wrapping_add(c);
        }
        sum
    }
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError("empty polynomial".to_string()));
        }
        let degree = self.degree();
        let idx = self.coeffs.len() - 1 - degree;
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let m = modulus as i64;
        for i in 0..(self.coeffs.len() - idx) {
            self.coeffs[i] = self.coeffs[i + idx] % m;
        }
        let new_size = self.coeffs.len() - idx;
        self.coeffs.truncate(new_size);
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
                let v1 = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let v2 = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = (v1 + v2) % m;
            }
        } else {
            for i in 0..self.coeffs.len() {
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
                let v1 = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let v2 = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = (v1 - v2) % m;
            }
        } else {
            for i in 0..self.coeffs.len() {
                result[i] = (self.coeffs[i] - other.coeffs[i]) % m;
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let m = modulus as i64;
        // Replicate the C poly_mul behavior with a result buffer.
        // The C version requires a pre-allocated result. We'll mimic it by
        // constructing a large enough buffer that matches the test case's
        // result.size = poly1.size + poly2.size - 1, but actually in C it is
        // chosen by the caller. In the test, result_mem has size 15, and the
        // formula uses res_deg = result.size - deg2 - deg1 - 1 to compute the
        // starting index.
        //
        // After poly_fit, the result is trimmed to remove leading zeros.
        // We implement equivalent behavior: produce the standard multiplication
        // result of length size1 + size2 - 1, then trim.
        let size1 = self.coeffs.len();
        let size2 = other.coeffs.len();
        if size1 == 0 || size2 == 0 {
            return Ok(Polynomial { coeffs: vec![] });
        }
        let result_size = size1 + size2 - 1;
        let mut result = vec![0 as Coeff; result_size];
        for i in 0..size1 {
            for j in 0..size2 {
                let prod = (self.coeffs[i] * other.coeffs[j]) % m;
                result[i + j] = result[i + j] + prod;
            }
        }
        // Apply mod
        for c in result.iter_mut() {
            *c = *c % m;
        }
        // Apply fit (remove leading zeros)
        let mut p = Polynomial { coeffs: result };
        p.fit(modulus)?;
        Ok(p)
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
                "first polynomial degree less than second".to_string(),
            ));
        }
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
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
        let mut p = Polynomial { coeffs: result };
        p.fit(modulus)?;
        Ok(p)
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
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let m = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0 as Coeff; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i] - s) % m;
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
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
