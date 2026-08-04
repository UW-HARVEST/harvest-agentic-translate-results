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

    /// Calculate the degree of the polynomial.
    /// Coefficients are stored in big-endian order: coeffs[0] is the highest
    /// degree term, coeffs[size-1] is the constant term.
    /// Degree returns `size - 1 - i` for the first non-zero coefficient.
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
            sum = sum.wrapping_add(c);
        }
        sum
    }

    /// Adjust the polynomial in-place to fit within the modulus and remove
    /// leading zeros (highest degree positions).
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(AcesError::GenericError(
                "empty polynomial".to_string(),
            ));
        }

        let degree = self.degree();
        let idx = self.coeffs.len() - 1 - degree;
        let len = self.coeffs.len();
        let m = modulus as i64;
        for i in 0..(len - idx) {
            // C: coeffs[i] = coeffs[i+idx] % mod (preserves sign)
            self.coeffs[i] = self.coeffs[i + idx] % m;
        }
        self.coeffs.truncate(len - idx);
        Ok(())
    }

    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is 0".to_string()));
        }
        // Result size matches the larger of the two
        let size1 = self.coeffs.len();
        let size2 = other.coeffs.len();
        let m = modulus as i64;
        if size1 != size2 {
            let size = std::cmp::max(size1, size2);
            let diff1 = size - size1;
            let diff2 = size - size2;
            let mut result = vec![0i64; size];
            for i in 0..size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = (a + b) % m;
            }
            Ok(Polynomial::new(result))
        } else {
            let mut result = vec![0i64; size1];
            for i in 0..size1 {
                result[i] = (self.coeffs[i] + other.coeffs[i]) % m;
            }
            Ok(Polynomial::new(result))
        }
    }

    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is 0".to_string()));
        }
        let size1 = self.coeffs.len();
        let size2 = other.coeffs.len();
        let m = modulus as i64;
        if size1 != size2 {
            let size = std::cmp::max(size1, size2);
            let diff1 = size - size1;
            let diff2 = size - size2;
            let mut result = vec![0i64; size];
            for i in 0..size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
                result[i] = (a - b) % m;
            }
            Ok(Polynomial::new(result))
        } else {
            let mut result = vec![0i64; size1];
            for i in 0..size1 {
                result[i] = (self.coeffs[i] - other.coeffs[i]) % m;
            }
            Ok(Polynomial::new(result))
        }
    }

    /// Multiply two polynomials.
    /// Mirrors C's poly_mul which initializes a result buffer, computes
    /// product into the lower bits with offset, then calls poly_fit at end.
    /// In our case, output Polynomial size = size1 + size2 - 1 by default,
    /// then poly_fit is applied.
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(AcesError::GenericError("modulus is 0".to_string()));
        }
        let m = modulus as i64;
        let size1 = self.coeffs.len();
        let size2 = other.coeffs.len();
        // The C code produces a result with size = result->size which is provided
        // by the caller and should be at least size1 + size2 - 1.
        // We allocate the minimum needed.
        let result_size = size1 + size2 - 1;
        let mut result = vec![0i64; result_size];
        // poly_fit shaves leading zeros, so we just produce coefficients in
        // big-endian polynomial multiplication.
        for i in 0..size1 {
            for j in 0..size2 {
                // C: result[res_deg + i + j] += poly1[i] * poly2[j] % mod
                // where res_deg = result_size - deg2 - deg1 - 1 = 0 here.
                let prod = (self.coeffs[i] * other.coeffs[j]) % m;
                result[i + j] += prod;
            }
        }
        let mut poly = Polynomial::new(result);
        poly.fit(modulus)?;
        Ok(poly)
    }

    /// Mirror C's poly_lshift. Both arguments use the same big-endian layout.
    /// For the test compatibility, we allow result of the same size as poly1.
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if other.coeffs.is_empty() || other.coeffs[0] != 1 {
            return Err(AcesError::GenericError(
                "lshift: leading coefficient of divisor must be 1".to_string(),
            ));
        }
        let degree1 = self.degree();
        let degree2 = other.degree();
        if degree1 < degree2 {
            return Err(AcesError::GenericError(
                "lshift: degree1 < degree2".to_string(),
            ));
        }
        let m = modulus as i64;
        let a_d = self.coeffs[0];
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            if i < other.coeffs.len() {
                let res = (self.coeffs[i] - other.coeffs[i] * a_d) % m;
                result[i] = if res < 0 { res + m } else { res };
            } else {
                result[i] = self.coeffs[i];
            }
        }
        let mut poly = Polynomial::new(result);
        poly.fit(modulus)?;
        Ok(poly)
    }

    /// Compute self mod divisor in-place.
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
        let m = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i] - s) % m;
        }
        Ok(Polynomial::new(result))
    }

    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        let s = scaler as i64;
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            result[i] = (self.coeffs[i] + s) % m;
        }
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
