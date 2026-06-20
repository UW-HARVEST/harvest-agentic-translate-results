use crate::error::Result;
pub type Coeff = i64;

fn err(msg: &str) -> crate::error::AcesError {
    crate::error::AcesError::GenericError(msg.to_string())
}

fn mod_u64(value: Coeff) -> u64 {
    value as u64
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
        if self.coeffs.is_empty() {
            return 0;
        }

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
        if self.coeffs.is_empty() || modulus == 0 {
            return Err(err("invalid polynomial or modulus"));
        }

        let degree = self.degree();
        let idx = self.coeffs.len() - 1 - degree;
        let new_len = self.coeffs.len() - idx;

        for i in 0..new_len {
            self.coeffs[i] = (mod_u64(self.coeffs[i + idx]) % modulus) as Coeff;
        }
        self.coeffs.truncate(new_len);
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(err("modulus must be non-zero"));
        }

        let size = self.coeffs.len().max(other.coeffs.len());
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        let mut coeffs = vec![0; size];

        for (i, slot) in coeffs.iter_mut().enumerate() {
            let left = if i >= diff1 {
                mod_u64(self.coeffs[i - diff1])
            } else {
                0
            };
            let right = if i >= diff2 {
                mod_u64(other.coeffs[i - diff2])
            } else {
                0
            };
            *slot = left.wrapping_add(right).wrapping_rem(modulus) as Coeff;
        }

        Ok(Polynomial::new(coeffs))
    }
    pub fn sub(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(err("modulus must be non-zero"));
        }

        let size = self.coeffs.len().max(other.coeffs.len());
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        let mut coeffs = vec![0; size];

        for (i, slot) in coeffs.iter_mut().enumerate() {
            let left = if i >= diff1 {
                mod_u64(self.coeffs[i - diff1])
            } else {
                0
            };
            let right = if i >= diff2 {
                mod_u64(other.coeffs[i - diff2])
            } else {
                0
            };
            *slot = left.wrapping_sub(right).wrapping_rem(modulus) as Coeff;
        }

        Ok(Polynomial::new(coeffs))
    }
    pub fn mul(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if self.coeffs.is_empty() || other.coeffs.is_empty() || modulus == 0 {
            return Err(err("invalid polynomial or modulus"));
        }

        let mut result = Polynomial::new(vec![0; self.coeffs.len() + other.coeffs.len()]);
        let deg1 = self.coeffs.len() - 1;
        let deg2 = other.coeffs.len() - 1;
        let res_deg = result.coeffs.len() - deg2 - deg1 - 1;

        for i in 0..self.coeffs.len() {
            for j in 0..other.coeffs.len() {
                let term = mod_u64(self.coeffs[i]).wrapping_mul(mod_u64(other.coeffs[j])) % modulus;
                result.coeffs[res_deg + i + j] += term as Coeff;
            }
        }

        result.fit(modulus)?;
        Ok(result)
    }
    pub fn lshift(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        if self.coeffs.is_empty() || other.coeffs.is_empty() || modulus == 0 {
            return Err(err("invalid polynomial or modulus"));
        }
        if other.coeffs[0] != 1 {
            return Err(err("divisor must be monic"));
        }

        let degree1 = self.degree();
        let degree2 = other.degree();
        if degree1 < degree2 {
            return Err(err("degree too small"));
        }

        let a_d = self.coeffs[0];
        let mut result = Polynomial::new(vec![0; self.coeffs.len()]);

        for i in 0..self.coeffs.len() {
            if i < other.coeffs.len() {
                let res = (self.coeffs[i] - (other.coeffs[i] * a_d)).wrapping_rem(modulus as Coeff);
                result.coeffs[i] = if res < 0 { res + modulus as Coeff } else { res };
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
        if modulus == 0 {
            return Err(err("modulus must be non-zero"));
        }

        Ok(Polynomial::new(
            self.coeffs
                .iter()
                .map(|coeff| mod_u64(*coeff).wrapping_sub(scaler).wrapping_rem(modulus) as Coeff)
                .collect(),
        ))
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        if modulus == 0 {
            return Err(err("modulus must be non-zero"));
        }

        Ok(Polynomial::new(
            self.coeffs
                .iter()
                .map(|coeff| mod_u64(*coeff).wrapping_add(scaler).wrapping_rem(modulus) as Coeff)
                .collect(),
        ))
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
