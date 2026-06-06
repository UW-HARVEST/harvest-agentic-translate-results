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
        let mut sum: i64 = 0;
        for &c in self.coeffs.iter() {
            sum = sum.wrapping_add(c);
        }
        sum
    }

    /// Mirrors C poly_fit:
    ///  - Compute the index `idx` of the leading nonzero coeff.
    ///  - For all i s.t. i+idx < size: coeffs[i] = coeffs[i+idx] % mod.
    ///  - Truncate size by `idx`.
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError(
                "Polynomial fit on empty polynomial".to_string(),
            ));
        }
        if modulus == 0 {
            return Err(AcesError::GenericError(
                "Polynomial fit with zero modulus".to_string(),
            ));
        }

        let size = self.coeffs.len();
        let degree = self.degree();
        let idx = size - 1 - degree;

        let m = modulus as i64;
        let new_size = size - idx;
        let mut new_coeffs = Vec::with_capacity(new_size);
        for i in 0..new_size {
            let v = self.coeffs[i + idx];
            new_coeffs.push(v % m);
        }
        self.coeffs = new_coeffs;
        Ok(())
    }

    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is 0".to_string()));
        }
        // C uses result with size >= max(poly1, poly2). The Rust API needs to allocate.
        let result_size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let m = modulus as i64;

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = result_size - self.coeffs.len();
            let diff2 = result_size - other.coeffs.len();
            let mut result = vec![0 as Coeff; result_size];
            for i in 0..result_size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = (a.wrapping_add(b)) % m;
            }
            return Ok(Polynomial { coeffs: result });
        }

        let mut result = vec![0 as Coeff; result_size];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i].wrapping_add(other.coeffs[i])) % m;
        }
        Ok(Polynomial { coeffs: result })
    }

    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is 0".to_string()));
        }
        let result_size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let m = modulus as i64;

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = result_size - self.coeffs.len();
            let diff2 = result_size - other.coeffs.len();
            let mut result = vec![0 as Coeff; result_size];
            for i in 0..result_size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = (a.wrapping_sub(b)) % m;
            }
            return Ok(Polynomial { coeffs: result });
        }

        let mut result = vec![0 as Coeff; result_size];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i].wrapping_sub(other.coeffs[i])) % m;
        }
        Ok(Polynomial { coeffs: result })
    }

    /// Multiply two polynomials. The C code uses a `result` buffer that is
    /// expected to be at least `poly1.size + poly2.size - 1` and writes into
    /// the higher region, then calls poly_fit to trim leading zeros.
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is 0".to_string()));
        }
        let m = modulus as i64;
        // Use a buffer of size poly1.size + poly2.size - 1 (mirrors the result region used).
        let result_size = self.coeffs.len() + other.coeffs.len() - 1;
        let mut buf = vec![0 as Coeff; result_size];

        for i in 0..self.coeffs.len() {
            for j in 0..other.coeffs.len() {
                let prod = (self.coeffs[i].wrapping_mul(other.coeffs[j])) % m;
                buf[i + j] = buf[i + j].wrapping_add(prod);
            }
        }

        let mut result = Polynomial { coeffs: buf };
        result.fit(modulus)?;
        Ok(result)
    }

    /// Mirrors C poly_lshift. self == poly1, other == poly2.
    /// Returns Err if poly2.coeffs[0] != 1 or degree(poly1) < degree(poly2).
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if other.coeffs.is_empty() || other.coeffs[0] != 1 {
            return Err(AcesError::GenericError(
                "lshift requires divisor[0] == 1".to_string(),
            ));
        }

        let degree1 = self.degree();
        let degree2 = other.degree();

        if degree1 < degree2 {
            return Err(AcesError::GenericError(
                "lshift requires degree(self) >= degree(other)".to_string(),
            ));
        }

        let a_d = self.coeffs[0];
        let m = modulus as i64;

        let mut buf = vec![0 as Coeff; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            if i < other.coeffs.len() {
                let res = (self.coeffs[i].wrapping_sub(other.coeffs[i].wrapping_mul(a_d))) % m;
                buf[i] = if res < 0 { res + m } else { res };
            } else {
                buf[i] = self.coeffs[i];
            }
        }

        let mut result = Polynomial { coeffs: buf };
        result.fit(modulus)?;
        Ok(result)
    }

    /// poly_mod: while poly_lshift succeeds, keep applying it on self.
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
            return Err(AcesError::GenericError("modulus is 0".to_string()));
        }
        let m = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0 as Coeff; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i].wrapping_sub(s)) % m;
        }
        Ok(Polynomial { coeffs: result })
    }

    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is 0".to_string()));
        }
        let m = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0 as Coeff; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i].wrapping_add(s)) % m;
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
