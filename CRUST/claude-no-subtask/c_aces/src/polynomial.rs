use crate::error::{AcesError, Result};
pub type Coeff = i64;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Polynomial {
    pub coeffs: Vec<Coeff>,
}

fn imod(a: i64, m: u64) -> Coeff {
    // Match C truncated remainder semantics for `a % (Coeff)mod`.
    let mm = m as i64;
    a % mm
}

impl Polynomial {
    pub fn new(coeffs: Vec<Coeff>) -> Self {
        Polynomial { coeffs }
    }
    pub fn degree(&self) -> usize {
        if self.coeffs.is_empty() {
            return 0;
        }
        for (i, c) in self.coeffs.iter().enumerate() {
            if *c != 0 {
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
        for c in &self.coeffs {
            sum = sum.wrapping_add(*c);
        }
        sum
    }
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError(
                "polynomial is empty".to_string(),
            ));
        }
        let degree = self.degree();
        let size = self.coeffs.len();
        let idx = size - 1 - degree;
        // shift coeffs[idx..] to coeffs[0..]
        for i in 0..(size - idx) {
            self.coeffs[i] = imod(self.coeffs[i + idx], modulus);
        }
        // Drop the last `idx` elements.
        self.coeffs.truncate(size - idx);
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        // C uses `result->size`; here Rust has no caller-owned result buffer,
        // so we build a vector of `max(self.size, other.size)`. The C semantics
        // expand each operand by left-padding with zeros (high-order coeffs
        // are at low indices). Replicate that behavior.
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        let mut result = vec![0i64; size];
        if self.coeffs.len() != other.coeffs.len() {
            for i in 0..size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = imod(a + b, modulus);
            }
        } else {
            for i in 0..size {
                result[i] = imod(self.coeffs[i] + other.coeffs[i], modulus);
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is zero".to_string()));
        }
        let size = std::cmp::max(self.coeffs.len(), other.coeffs.len());
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        let mut result = vec![0i64; size];
        if self.coeffs.len() != other.coeffs.len() {
            for i in 0..size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = imod(a - b, modulus);
            }
        } else {
            for i in 0..size {
                result[i] = imod(self.coeffs[i] - other.coeffs[i], modulus);
            }
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        // Result polynomial size in C is provided by the caller. The
        // existing C code does `res_deg = result->size - deg2 - deg1 - 1` and
        // requires result to be large enough. Mirroring the semantics, the
        // tightest fit is `len1 + len2 - 1`, but the existing C tests rely on
        // a result that is at least that large, then `poly_fit` trims. For our
        // Rust API we allocate `len1 + len2 - 1` (the natural product size).
        let len1 = self.coeffs.len();
        let len2 = other.coeffs.len();
        if len1 == 0 || len2 == 0 {
            return Ok(Polynomial { coeffs: Vec::new() });
        }
        let size = len1 + len2 - 1;
        let mut result = vec![0i64; size];
        // C: deg1 = poly1->size - 1, deg2 = poly2->size - 1
        // res_deg = result->size - deg2 - deg1 - 1 = (len1+len2-1) - (len2-1) - (len1-1) - 1 = 0
        // So result[i+j] += (poly1[i] * poly2[j]) % mod
        let mm = modulus as i64;
        for i in 0..len1 {
            for j in 0..len2 {
                let prod = self.coeffs[i].wrapping_mul(other.coeffs[j]) % mm;
                result[i + j] = result[i + j].wrapping_add(prod);
            }
        }
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
                "degree(self) < degree(other)".to_string(),
            ));
        }
        let a_d = self.coeffs[0];
        let mut result = vec![0i64; self.coeffs.len()];
        let mm = modulus as i64;
        for i in 0..self.coeffs.len() {
            if i < other.coeffs.len() {
                let res = (self.coeffs[i] - other.coeffs[i].wrapping_mul(a_d)) % mm;
                result[i] = if res < 0 { res + mm } else { res };
            } else {
                result[i] = self.coeffs[i];
            }
        }
        let mut p = Polynomial { coeffs: result };
        p.fit(modulus)?;
        Ok(p)
    }
    pub fn poly_mod(&mut self, divisor: &Polynomial, modulus: u64) -> Result<()> {
        // Repeatedly apply lshift until it fails (i.e. degree < divisor.degree()).
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
        let mm = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i] - s) % mm;
        }
        Ok(Polynomial { coeffs: result })
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let mm = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i] + s) % mm;
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
