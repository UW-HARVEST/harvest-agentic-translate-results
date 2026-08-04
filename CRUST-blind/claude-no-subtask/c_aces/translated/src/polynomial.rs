use crate::error::{AcesError, Result};
pub type Coeff = i64;

/// Compute `value mod modulus` for an i64, mirroring C's `%` semantics on i64
/// where the result keeps the sign of the dividend. Used for intermediate
/// arithmetic prior to wrapping.
fn c_mod(value: i64, modulus: i64) -> i64 {
    if modulus == 0 {
        return value;
    }
    value % modulus
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Polynomial {
    pub coeffs: Vec<Coeff>,
}
impl Polynomial {
    pub fn new(coeffs: Vec<Coeff>) -> Self {
        Polynomial { coeffs }
    }
    pub fn degree(&self) -> usize {
        // Coefficients are stored highest-degree first, matching the C
        // implementation. Degree is `size - 1 - i` for the first non-zero `i`.
        let size = self.coeffs.len();
        if size == 0 {
            return 0;
        }
        for (i, c) in self.coeffs.iter().enumerate() {
            if *c != 0 {
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
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError(
                "Cannot fit empty polynomial".to_string(),
            ));
        }
        let degree = self.degree();
        let size = self.coeffs.len();
        let idx = size - 1 - degree;
        let m = modulus as i64;
        let new_size = size - idx;
        for i in 0..new_size {
            self.coeffs[i] = c_mod(self.coeffs[i + idx], m);
        }
        self.coeffs.truncate(new_size);
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus must be non-zero".to_string()));
        }
        let m = modulus as i64;
        let size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let mut result = vec![0i64; size];

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = size - self.coeffs.len();
            let diff2 = size - other.coeffs.len();
            for i in 0..size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = c_mod(a.wrapping_add(b), m);
            }
        } else {
            for i in 0..self.coeffs.len() {
                result[i] = c_mod(self.coeffs[i].wrapping_add(other.coeffs[i]), m);
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus must be non-zero".to_string()));
        }
        let m = modulus as i64;
        let size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let mut result = vec![0i64; size];

        if self.coeffs.len() != other.coeffs.len() {
            let diff1 = size - self.coeffs.len();
            let diff2 = size - other.coeffs.len();
            for i in 0..size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = c_mod(a.wrapping_sub(b), m);
            }
        } else {
            for i in 0..self.coeffs.len() {
                result[i] = c_mod(self.coeffs[i].wrapping_sub(other.coeffs[i]), m);
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        // Mirrors C `poly_mul`: the result polynomial has a fixed pre-allocated
        // size. We choose the result size to be self.size + other.size - 1, then
        // run `fit` to remove leading zeros.
        if self.coeffs.is_empty() || other.coeffs.is_empty() {
            return Err(AcesError::GenericError(
                "Cannot multiply empty polynomials".to_string(),
            ));
        }
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus must be non-zero".to_string()));
        }
        let m = modulus as i64;
        let result_size = self.coeffs.len() + other.coeffs.len() - 1;
        let mut result = vec![0i64; result_size];
        // res_deg = result_size - deg2 - deg1 - 1
        // = result_size - (other.size - 1) - (self.size - 1) - 1
        // = result_size - other.size - self.size + 1 = 0
        // So no offset needed.
        for i in 0..self.coeffs.len() {
            for j in 0..other.coeffs.len() {
                let prod = c_mod(self.coeffs[i].wrapping_mul(other.coeffs[j]), m);
                result[i + j] = result[i + j].wrapping_add(prod);
            }
        }
        let mut poly = Polynomial { coeffs: result };
        poly.fit(modulus)?;
        Ok(poly)
    }
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        // Mirror C `poly_lshift`. The C version writes into `result` whose size
        // matches poly1->size; it returns -1 if poly2->coeffs[0] != 1 or if
        // degree(poly1) < degree(poly2). We mimic this by returning Err.
        if other.coeffs.is_empty() || other.coeffs[0] != 1 {
            return Err(AcesError::GenericError(
                "lshift requires divisor leading coefficient to be 1".to_string(),
            ));
        }
        let degree1 = self.degree();
        let degree2 = other.degree();
        if degree1 < degree2 {
            return Err(AcesError::GenericError(
                "lshift requires degree(self) >= degree(other)".to_string(),
            ));
        }
        let m = modulus as i64;
        if m == 0 {
            return Err(AcesError::GenericError("modulus must be non-zero".to_string()));
        }
        let a_d = self.coeffs[0];
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            if i < other.coeffs.len() {
                let res = c_mod(self.coeffs[i].wrapping_sub(other.coeffs[i].wrapping_mul(a_d)), m);
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
        // Repeatedly apply lshift until it returns an error (mirroring while-0
        // semantic in the C implementation). Always return Ok.
        loop {
            match self.lshift(divisor, modulus) {
                Ok(p) => *self = p,
                Err(_) => break,
            }
        }
        Ok(())
    }
    pub fn sub_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus must be non-zero".to_string()));
        }
        let m = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = c_mod(self.coeffs[i].wrapping_sub(s), m);
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus must be non-zero".to_string()));
        }
        let m = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = c_mod(self.coeffs[i].wrapping_add(s), m);
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
