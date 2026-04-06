use crate::error::Result;
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
        if self.coeffs.is_empty() { return 0; }
        for i in 0..self.coeffs.len() {
            if self.coeffs[i] != 0 {
                return self.coeffs.len() - i - 1;
            }
        }
        0
    }
    pub fn set_zero(&mut self) {
        for c in self.coeffs.iter_mut() { *c = 0; }
    }
    pub fn coef_sum(&self) -> Coeff {
        self.coeffs.iter().sum()
    }
    pub fn fit(&mut self, modulus: u64) -> Result<()> {
        if self.coeffs.is_empty() {
            return Err(crate::error::AcesError::GenericError("empty poly".into()));
        }
        let degree = self.degree();
        let idx = self.coeffs.len() - 1 - degree;
        let m = modulus as i64;
        let new: Vec<Coeff> = self.coeffs[idx..].iter().map(|&c| c % m).collect();
        self.coeffs = new;
        Ok(())
    }
    pub fn add(&self, other: &Polynomial, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        let size = self.coeffs.len().max(other.coeffs.len());
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        let mut result = vec![0i64; size];
        if self.coeffs.len() != other.coeffs.len() {
            for i in 0..size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
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
        let m = modulus as i64;
        let size = self.coeffs.len().max(other.coeffs.len());
        let diff1 = size - self.coeffs.len();
        let diff2 = size - other.coeffs.len();
        let mut result = vec![0i64; size];
        if self.coeffs.len() != other.coeffs.len() {
            for i in 0..size {
                let a = if i >= diff1 { self.coeffs[i - diff1] } else { 0 };
                let b = if i >= diff2 { other.coeffs[i - diff2] } else { 0 };
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
        let res_size = self.coeffs.len() + other.coeffs.len() - 1;
        let mut result = vec![0i64; res_size];
        let deg1 = self.coeffs.len() - 1;
        let deg2 = other.coeffs.len() - 1;
        let res_deg = res_size - deg2 - deg1 - 1;
        for i in 0..self.coeffs.len() {
            for j in 0..other.coeffs.len() {
                result[res_deg + i + j] += (self.coeffs[i] * other.coeffs[j]) % m;
            }
        }
        let mut p = Polynomial { coeffs: result };
        p.fit(modulus)?;
        Ok(p)
    }
    pub fn lshift(&self, _other: &Polynomial, _modulus: u64) -> Result<Polynomial> {
        if _other.coeffs[0] != 1 { return Err(crate::error::AcesError::GenericError("leading coeff must be 1".into())); }
        let degree1 = self.degree();
        let degree2 = _other.degree();
        if degree1 < degree2 { return Err(crate::error::AcesError::GenericError("degree1 < degree2".into())); }
        let a_d = self.coeffs[0];
        let m = _modulus as i64;
        let mut result = vec![0i64; self.coeffs.len()];
        for i in 0..self.coeffs.len() {
            if i < _other.coeffs.len() {
                let res = (self.coeffs[i] - _other.coeffs[i] * a_d) % m;
                result[i] = if res < 0 { res + m } else { res };
            } else {
                result[i] = self.coeffs[i];
            }
        }
        let mut p = Polynomial { coeffs: result };
        p.fit(_modulus)?;
        Ok(p)
    }
    pub fn poly_mod(&mut self, _divisor: &Polynomial, _modulus: u64) -> Result<()> {
        while let Ok(shifted) = self.lshift(_divisor, _modulus) {
            self.coeffs = shifted.coeffs;
        }
        Ok(())
    }
    pub fn sub_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        let s = scaler as i64;
        Ok(Polynomial { coeffs: self.coeffs.iter().map(|&c| (c - s) % m).collect() })
    }
    pub fn add_scaler(&self, scaler: u64, modulus: u64) -> Result<Polynomial> {
        let m = modulus as i64;
        let s = scaler as i64;
        Ok(Polynomial { coeffs: self.coeffs.iter().map(|&c| (c + s) % m).collect() })
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
