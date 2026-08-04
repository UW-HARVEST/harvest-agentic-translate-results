use crate::error::Result;
pub type Coeff = i64;

fn generic_error(message: impl Into<String>) -> crate::error::AcesError {
    crate::error::AcesError::GenericError(message.into())
}

fn c_unsigned_mod(value: i128, modulus: u64) -> Coeff {
    ((value as i64 as u64) % modulus) as Coeff
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Polynomial {
    pub coeffs: Vec<Coeff>,
}
impl Polynomial {
    pub fn new(coeffs: Vec<Coeff>) -> Self {
        Self { coeffs }
    }
    pub fn degree(&self) -> usize {
        for (idx, coeff) in self.coeffs.iter().enumerate() {
            if *coeff != 0 {
                return self.coeffs.len() - idx - 1;
            }
        }

        0
    }
    pub fn set_zero(&mut self) {
        self.coeffs.fill(0);
    }
    pub fn coef_sum(&self) -> Coeff {
        self.coeffs.iter().copied().sum()
    }
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(generic_error("polynomial is empty"));
        }

        let degree = self.degree();
        let idx = self.coeffs.len() - 1 - degree;
        let len = self.coeffs.len();
        for i in 0..(len - idx) {
            self.coeffs[i] = c_unsigned_mod(self.coeffs[i + idx] as i128, modulus);
        }
        self.coeffs.truncate(len - idx);
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(generic_error("modulus must be non-zero"));
        }

        let size = self.coeffs.len().max(other.coeffs.len());
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        let mut coeffs = vec![0; size];

        for (i, slot) in coeffs.iter_mut().enumerate() {
            let lhs = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
            let rhs = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
            *slot = c_unsigned_mod(lhs as i128 + rhs as i128, modulus);
        }

        Ok(Polynomial::new(coeffs))
    }
    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(generic_error("modulus must be non-zero"));
        }

        let size = self.coeffs.len().max(other.coeffs.len());
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        let mut coeffs = vec![0; size];

        for (i, slot) in coeffs.iter_mut().enumerate() {
            let lhs = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
            let rhs = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
            *slot = c_unsigned_mod(lhs as i128 - rhs as i128, modulus);
        }

        Ok(Polynomial::new(coeffs))
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if self.coeffs.is_empty() || other.coeffs.is_empty() {
            return Err(generic_error("polynomial is empty"));
        }
        if modulus == 0 {
            return Err(generic_error("modulus must be non-zero"));
        }

        let mut result = Polynomial::new(vec![0; self.coeffs.len() + other.coeffs.len()]);
        let deg1 = self.coeffs.len() - 1;
        let deg2 = other.coeffs.len() - 1;
        let res_deg = result.coeffs.len() - deg2 - deg1 - 1;

        for i in 0..self.coeffs.len() {
            for j in 0..other.coeffs.len() {
                let product = c_unsigned_mod(
                    self.coeffs[i] as i128 * other.coeffs[j] as i128,
                    modulus,
                );
                result.coeffs[res_deg + i + j] += product;
            }
        }

        result.fit(modulus)?;
        Ok(result)
    }
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if other.coeffs.first().copied() != Some(1) {
            return Err(generic_error("divisor must be monic"));
        }

        if self.degree() < other.degree() {
            return Err(generic_error("degree too small"));
        }

        let a_d = self.coeffs[0];
        let mut result = Polynomial::new(vec![0; self.coeffs.len()]);
        let modulus_i = modulus as i128;

        for i in 0..self.coeffs.len() {
            if i < other.coeffs.len() {
                let raw =
                    (self.coeffs[i] as i128 - (other.coeffs[i] as i128 * a_d as i128)) % modulus_i;
                result.coeffs[i] = if raw < 0 {
                    (raw + modulus_i) as Coeff
                } else {
                    raw as Coeff
                };
            } else {
                result.coeffs[i] = self.coeffs[i];
            }
        }

        result.fit(modulus)?;
        Ok(result)
    }
    pub fn poly_mod(&mut self, divisor: &Polynomial, modulus: u64) -> Result<()> {
        while let Ok(next) = self.lshift(divisor, modulus) {
            *self = next;
        }
        Ok(())
    }
    pub fn sub_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let coeffs = self
            .coeffs
            .iter()
            .map(|&coeff| c_unsigned_mod(coeff as i128 - scaler as i128, modulus))
            .collect();
        Ok(Polynomial::new(coeffs))
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let coeffs = self
            .coeffs
            .iter()
            .map(|&coeff| c_unsigned_mod(coeff as i128 + scaler as i128, modulus))
            .collect();
        Ok(Polynomial::new(coeffs))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyArray {
    pub polies: Vec<Polynomial>,
}
impl PolyArray {
    pub fn new(polies: Vec<Polynomial>) -> Self {
        Self { polies }
    }
}
