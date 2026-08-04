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
        for c in &self.coeffs {
            sum += *c;
        }
        sum
    }
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError(
                "poly_fit: empty polynomial".to_string(),
            ));
        }
        let degree = self.degree();
        let idx = self.coeffs.len() - 1 - degree;
        let size = self.coeffs.len();
        for i in 0..(size - idx) {
            self.coeffs[i] = self.coeffs[i + idx] % (modulus as i64);
        }
        self.coeffs.truncate(size - idx);
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("poly_add: zero mod".to_string()));
        }
        let m = modulus as i64;

        // The C version takes a `result` whose size equals the maximum size
        // observed; it pads the smaller polynomial. Here we choose result size =
        // max(self.len(), other.len()).
        let result_size = if self.coeffs.len() > other.coeffs.len() {
            self.coeffs.len()
        } else {
            other.coeffs.len()
        };
        let mut result = vec![0 as Coeff; result_size];

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = result_size - self.coeffs.len();
            let diff2 = result_size - other.coeffs.len();
            for i in 0..result_size {
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
            for i in 0..self.coeffs.len() {
                result[i] = (self.coeffs[i] + other.coeffs[i]) % m;
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("poly_sub: zero mod".to_string()));
        }
        let m = modulus as i64;
        let result_size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let mut result = vec![0 as Coeff; result_size];

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = result_size - self.coeffs.len();
            let diff2 = result_size - other.coeffs.len();
            for i in 0..result_size {
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
            for i in 0..self.coeffs.len() {
                result[i] = (self.coeffs[i] - other.coeffs[i]) % m;
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        // The C version writes into a pre-allocated result whose size accommodates
        // the full product (size1 + size2 - 1 at minimum). After multiplying it
        // calls poly_fit which trims leading zeros and applies the modulus.
        let result_size = self.coeffs.len() + other.coeffs.len();
        let mut result = vec![0 as Coeff; result_size];
        if self.coeffs.is_empty() || other.coeffs.is_empty() {
            return Ok(Polynomial { coeffs: result });
        }
        let deg1 = self.coeffs.len() - 1;
        let deg2 = other.coeffs.len() - 1;
        let res_deg = result_size - deg2 - deg1 - 1;
        for i in 0..self.coeffs.len() {
            for j in 0..other.coeffs.len() {
                result[res_deg + i + j] += (self.coeffs[i] * other.coeffs[j]) % m;
            }
        }
        let mut p = Polynomial { coeffs: result };
        p.fit(modulus)?;
        Ok(p)
    }
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if other.coeffs.is_empty() || other.coeffs[0] != 1 {
            return Err(AcesError::GenericError(
                "poly_lshift: leading coef of divisor not 1".to_string(),
            ));
        }

        let degree1 = self.degree();
        let degree2 = other.degree();

        if degree1 < degree2 {
            return Err(AcesError::GenericError(
                "poly_lshift: degree1 < degree2".to_string(),
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
