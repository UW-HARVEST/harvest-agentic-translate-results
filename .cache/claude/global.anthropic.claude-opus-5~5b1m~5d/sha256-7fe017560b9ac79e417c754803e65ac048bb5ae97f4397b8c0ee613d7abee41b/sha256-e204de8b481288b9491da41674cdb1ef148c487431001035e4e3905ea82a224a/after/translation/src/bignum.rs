//! Minimal arbitrary-precision unsigned integer support.
//!
//! Only the handful of operations needed to decide, exactly, whether a decimal
//! literal is tiny (below `2^-1022`) and whether it is exactly representable as
//! a `f64` subnormal (an integer multiple of `2^-1074`).

use std::cmp::Ordering;

/// Unsigned big integer, little-endian base 2^32 limbs, no trailing zero limbs.
#[derive(Clone, Debug, Default)]
pub struct Big {
    limbs: Vec<u32>,
}

impl Big {
    pub fn zero() -> Big {
        Big { limbs: Vec::new() }
    }

    pub fn one() -> Big {
        Big { limbs: vec![1] }
    }

    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    fn trim(&mut self) {
        while let Some(&0) = self.limbs.last() {
            self.limbs.pop();
        }
    }

    /// self = self * m + a   (m, a < 2^32)
    pub fn mul_add_small(&mut self, m: u32, a: u32) {
        let mut carry = a as u64;
        for limb in self.limbs.iter_mut() {
            let cur = (*limb as u64) * (m as u64) + carry;
            *limb = (cur & 0xffff_ffff) as u32;
            carry = cur >> 32;
        }
        while carry != 0 {
            self.limbs.push((carry & 0xffff_ffff) as u32);
            carry >>= 32;
        }
        self.trim();
    }

    /// self = self / d, returning the remainder.  (0 < d < 2^32)
    pub fn divmod_small(&mut self, d: u32) -> u32 {
        let mut rem: u64 = 0;
        for limb in self.limbs.iter_mut().rev() {
            let cur = (rem << 32) | (*limb as u64);
            *limb = (cur / (d as u64)) as u32;
            rem = cur % (d as u64);
        }
        self.trim();
        rem as u32
    }

    /// self = self * 2^n
    pub fn shl_bits(&mut self, n: usize) {
        if self.is_zero() || n == 0 {
            return;
        }
        let limb_shift = n / 32;
        let bit_shift = (n % 32) as u32;
        if bit_shift != 0 {
            let mut carry: u32 = 0;
            for limb in self.limbs.iter_mut() {
                let v = *limb;
                *limb = (v << bit_shift) | carry;
                carry = v >> (32 - bit_shift);
            }
            if carry != 0 {
                self.limbs.push(carry);
            }
        }
        if limb_shift != 0 {
            let mut new_limbs = vec![0u32; limb_shift];
            new_limbs.extend_from_slice(&self.limbs);
            self.limbs = new_limbs;
        }
        self.trim();
    }

    /// Build from a sequence of ASCII decimal digits.
    pub fn from_digits(digits: &[u8]) -> Big {
        let mut b = Big::zero();
        let mut idx = 0;
        while idx < digits.len() {
            let chunk = (digits.len() - idx).min(9);
            let mut val: u32 = 0;
            for &c in &digits[idx..idx + chunk] {
                val = val * 10 + (c - b'0') as u32;
            }
            let mut scale: u32 = 1;
            for _ in 0..chunk {
                scale *= 10;
            }
            b.mul_add_small(scale, val);
            idx += chunk;
        }
        b
    }

    /// 10^e
    pub fn pow10(e: usize) -> Big {
        let mut b = Big::one();
        let mut left = e;
        while left > 0 {
            let chunk = left.min(9);
            let mut scale: u32 = 1;
            for _ in 0..chunk {
                scale *= 10;
            }
            b.mul_add_small(scale, 0);
            left -= chunk;
        }
        b
    }

    pub fn cmp_big(&self, other: &Big) -> Ordering {
        if self.limbs.len() != other.limbs.len() {
            return self.limbs.len().cmp(&other.limbs.len());
        }
        for i in (0..self.limbs.len()).rev() {
            if self.limbs[i] != other.limbs[i] {
                return self.limbs[i].cmp(&other.limbs[i]);
            }
        }
        Ordering::Equal
    }
}
