use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Debug)]
pub struct BigInt {
    negative: bool,
    digits: Vec<u8>,
}

impl BigInt {
    pub fn zero() -> Self {
        BigInt {
            negative: false,
            digits: vec![0],
        }
    }

    pub fn is_64_bit(&self) -> bool {
        // Mirrors the C implementation: returns true iff size < 10
        self.digits.len() < 10
    }

    pub fn remove_leading_zeros(&mut self) {
        let mut leading_zeros = 0usize;
        for &d in &self.digits {
            if d == 0 {
                leading_zeros += 1;
            } else {
                break;
            }
        }
        if leading_zeros > 0 {
            if leading_zeros >= self.digits.len() {
                self.digits.clear();
                self.digits.push(0);
                return;
            }
            self.digits.drain(..leading_zeros);
        }
    }

    pub fn from_int(n: i64) -> Self {
        let (negative, mut m) = if n < 0 {
            (true, (n as i128).unsigned_abs() as u64)
        } else {
            (false, n as u64)
        };
        let mut digits_rev: Vec<u8> = Vec::new();
        if m == 0 {
            digits_rev.push(0);
        } else {
            while m > 0 {
                digits_rev.push((m % 10) as u8);
                m /= 10;
            }
        }
        digits_rev.reverse();
        let mut result = BigInt {
            negative,
            digits: digits_rev,
        };
        result.remove_leading_zeros();
        result
    }

    pub fn to_int(&self) -> i64 {
        let mut result: i64 = 0;
        for &d in &self.digits {
            result = result.wrapping_mul(10);
            result = result.wrapping_add(d as i64);
        }
        if self.negative {
            result = result.wrapping_neg();
        }
        result
    }

    pub fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let (negative, start) = if !bytes.is_empty() && bytes[0] == b'-' {
            (true, 1)
        } else {
            (false, 0)
        };
        let digits: Vec<u8> = bytes[start..].iter().map(|b| b - b'0').collect();
        BigInt { negative, digits }
    }

    pub fn copy(&self) -> BigInt {
        self.clone()
    }

    pub fn print(&self) {
        print!("{}", self);
    }

    pub fn is_zero(&self) -> bool {
        let mut cleaned = self.clone();
        cleaned.remove_leading_zeros();
        cleaned.digits.len() == 1 && cleaned.digits[0] == 0
    }

    pub fn lt_zero(&self) -> bool {
        if self.is_zero() {
            return false;
        }
        self.negative
    }

    pub fn gt_zero(&self) -> bool {
        if self.is_zero() {
            return false;
        }
        !self.negative
    }

    pub fn lezero(&self) -> bool {
        if self.is_zero() {
            return true;
        }
        self.negative
    }

    pub fn gezero(&self) -> bool {
        if self.is_zero() {
            return true;
        }
        !self.negative
    }

    pub fn abs(&self) -> Self {
        let mut r = self.clone();
        r.negative = false;
        r
    }

    pub fn add(&self, other: &Self) -> Self {
        // Handle sign cases like the C implementation
        if self.negative && other.negative {
            // -a + -b = -(a + b) where a, b >= 0
            let a = self.abs();
            let b = other.abs();
            let mut r = a.add_unsigned(&b);
            r.negative = true;
            r.remove_leading_zeros();
            // -0 should be normalized to 0
            if r.digits.len() == 1 && r.digits[0] == 0 {
                r.negative = false;
            }
            r
        } else if self.negative {
            // -a + b = b - a
            let a = self.abs();
            other.sub(&a)
        } else if other.negative {
            // a + -b = a - b
            let b = other.abs();
            self.sub(&b)
        } else {
            self.add_unsigned(other)
        }
    }

    fn add_unsigned(&self, other: &Self) -> Self {
        let size = self.digits.len().max(other.digits.len());
        let mut result_digits = vec![0u8; size];
        let mut carry: u32 = 0;
        for i in 0..size {
            let mut sum: u32 = carry;
            if i < self.digits.len() {
                sum += self.digits[self.digits.len() - i - 1] as u32;
            }
            if i < other.digits.len() {
                sum += other.digits[other.digits.len() - i - 1] as u32;
            }
            result_digits[size - i - 1] = (sum % 10) as u8;
            carry = sum / 10;
        }
        if carry > 0 {
            // Prepend carry digits
            let mut prefix: Vec<u8> = Vec::new();
            let mut c = carry;
            while c > 0 {
                prefix.push((c % 10) as u8);
                c /= 10;
            }
            prefix.reverse();
            prefix.extend_from_slice(&result_digits);
            result_digits = prefix;
        }
        let mut r = BigInt {
            negative: false,
            digits: result_digits,
        };
        r.remove_leading_zeros();
        r
    }

    pub fn sub(&self, other: &Self) -> Self {
        // Handle sign cases like the C implementation
        if self.negative && other.negative {
            // -a - (-b) = b - a
            let a = self.abs();
            let b = other.abs();
            b.sub(&a)
        } else if self.negative {
            // -a - b = -(a + b)
            let a = self.abs();
            let mut r = a.add_unsigned(other);
            r.negative = true;
            r.remove_leading_zeros();
            if r.digits.len() == 1 && r.digits[0] == 0 {
                r.negative = false;
            }
            r
        } else if other.negative {
            // a - (-b) = a + b
            let b = other.abs();
            self.add_unsigned(&b)
        } else {
            // Both non-negative
            let is_negative = self < other;
            let (larger, smaller) = if self > other {
                (self, other)
            } else {
                (other, self)
            };
            let size = self.digits.len().max(other.digits.len());
            let mut result_digits = vec![0u8; size];
            let mut borrow: i32 = 0;
            for i in 0..size {
                let mut diff: i32 = borrow;
                if i < larger.digits.len() {
                    diff += larger.digits[larger.digits.len() - i - 1] as i32;
                }
                if i < smaller.digits.len() {
                    diff -= smaller.digits[smaller.digits.len() - i - 1] as i32;
                }
                if diff < 0 {
                    diff += 10;
                    borrow = -1;
                } else {
                    borrow = 0;
                }
                result_digits[size - i - 1] = diff as u8;
            }
            let mut r = BigInt {
                negative: is_negative,
                digits: result_digits,
            };
            r.remove_leading_zeros();
            // -0 normalization
            if r.digits.len() == 1 && r.digits[0] == 0 {
                r.negative = false;
            }
            r
        }
    }

    pub fn inc(&mut self) {
        let one = BigInt::from_int(1);
        *self = self.add(&one);
    }

    pub fn dec(&mut self) {
        let one = BigInt::from_int(1);
        *self = self.sub(&one);
    }

    pub fn mul(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int().wrapping_mul(other.to_int()));
        }

        let a_len = self.digits.len();
        let b_len = other.digits.len();
        let n = a_len + b_len;
        let mut result_digits: Vec<u64> = vec![0u64; n];

        for i in 0..a_len {
            for j in 0..b_len {
                let pos = n - i - j - 1;
                result_digits[pos] = result_digits[pos]
                    + (self.digits[a_len - i - 1] as u64) * (other.digits[b_len - j - 1] as u64);
            }
        }

        // Normalize
        let mut carry: u64 = 0;
        let mut final_digits = vec![0u8; n];
        for i in 0..n {
            let pos = n - i - 1;
            let sum = result_digits[pos] + carry;
            final_digits[pos] = (sum % 10) as u8;
            carry = sum / 10;
        }

        let mut digits = Vec::with_capacity(n + 4);
        if carry > 0 {
            // Prepend remaining carry digits (shouldn't happen normally, but be safe)
            let mut c = carry;
            let mut prefix: Vec<u8> = Vec::new();
            while c > 0 {
                prefix.push((c % 10) as u8);
                c /= 10;
            }
            prefix.reverse();
            digits.extend(prefix);
        }
        digits.extend(final_digits);

        let mut result = BigInt {
            negative: self.negative != other.negative,
            digits,
        };
        result.remove_leading_zeros();
        // -0 normalization
        if result.digits.len() == 1 && result.digits[0] == 0 {
            result.negative = false;
        }
        result
    }

    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        if self.is_64_bit() && other.is_64_bit() {
            let n_int = self.to_int();
            let d_int = other.to_int();
            let q = n_int / d_int;
            let r = n_int % d_int;
            return (BigInt::from_int(q), BigInt::from_int(r));
        }

        let mut quotient = BigInt::from_int(0);
        let negative = self.negative != other.negative;

        let mut numerator = self.clone();
        numerator.negative = false;
        let mut denominator = other.clone();
        denominator.negative = false;

        // Slow algorithm: subtract denominator until numerator < denominator
        while numerator.gt_zero() {
            numerator = numerator.sub(&denominator);
            if numerator.lt_zero() {
                break;
            }
            quotient.inc();
        }
        if numerator.lt_zero() {
            numerator = numerator.add(&denominator);
        }

        let mut remainder = numerator;
        quotient.remove_leading_zeros();

        if negative {
            quotient.negative = true;
            remainder.negative = true;
            // Normalize -0
            if quotient.digits.len() == 1 && quotient.digits[0] == 0 {
                quotient.negative = false;
            }
            if remainder.digits.len() == 1 && remainder.digits[0] == 0 {
                remainder.negative = false;
            }
        }

        (quotient, remainder)
    }

    pub fn div(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() / other.to_int());
        }
        let (q, _) = self.divmod(other);
        q
    }

    pub fn r#mod(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() % other.to_int());
        }
        let (_, r) = self.divmod(other);
        r
    }

    pub fn pow(&self, exponent: &Self) -> Self {
        if exponent.negative {
            return BigInt::from_int(0);
        }
        let mut result = BigInt::from_int(1);
        if exponent.is_zero() {
            return result;
        }
        let mut b = exponent.clone();
        while !b.is_zero() {
            result = result.mul(self);
            b.dec();
        }
        result
    }

    pub fn fast_pow(&self, exponent: &Self, modulo: &Self) -> Self {
        if self.is_64_bit() && exponent.is_64_bit() && modulo.is_64_bit() {
            let mut result: i64 = 1;
            let mut exp: i64 = exponent.to_int();
            let m: i64 = modulo.to_int();
            // Match C's `%` semantics (truncating).
            let mut base: i64 = self.to_int() % m;
            while exp > 0 {
                if exp % 2 == 1 {
                    result = (result.wrapping_mul(base)) % m;
                }
                exp >>= 1;
                base = (base.wrapping_mul(base)) % m;
            }
            return BigInt::from_int(result);
        }

        if exponent.negative {
            return BigInt::from_int(0);
        }
        if exponent.is_zero() {
            return BigInt::from_int(1);
        }

        // Square-and-multiply
        let mut result = BigInt::from_int(1);
        let mut base = self.r#mod(modulo);
        let mut exp = exponent.clone();
        let two = BigInt::from_int(2);
        while !exp.is_zero() {
            if exp.is_odd() {
                result = result.mul(&base).r#mod(modulo);
            }
            exp = exp.div(&two);
            base = base.mul(&base).r#mod(modulo);
        }
        result
    }

    pub fn modinv(&self, modulo: &Self) -> Self {
        let m0 = modulo.clone();
        let mut y = BigInt::from_int(0);
        let mut x = BigInt::from_int(1);
        let one = BigInt::from_int(1);
        let mut a = self.clone();
        let mut m = modulo.clone();

        while !a.is_zero() {
            let q = m.div(&a);
            let qa = q.mul(&a);
            let t = m.sub(&qa);

            m = a.clone();
            a = t;

            let qx = q.mul(&x);
            let t2 = y.sub(&qx);
            y = x.clone();
            x = t2;
        }

        if m.lt_zero() {
            m = m.add(&m0);
        }

        if m == one {
            let mut result = y.r#mod(&m0);
            if result.lt_zero() {
                result = result.add(&m0);
            }
            return result;
        }
        BigInt::from_int(0)
    }

    pub fn sqrt(&self) -> Self {
        let one = BigInt::from_int(1);
        let two = BigInt::from_int(2);
        let mut low = BigInt::from_int(0);
        let mut high = self.clone();
        while low < high {
            let sum = low.add(&high);
            let mid = sum.div(&two);
            let sq = mid.mul(&mid);
            if sq < *self {
                low = mid.add(&one);
            } else {
                high = mid;
            }
        }
        low
    }

    pub fn is_even(&self) -> bool {
        let last = self.digits[self.digits.len() - 1];
        last % 2 == 0
    }

    pub fn is_odd(&self) -> bool {
        let last = self.digits[self.digits.len() - 1];
        last % 2 == 1
    }

    pub fn is_prime(&self) -> bool {
        // Even check
        if self.is_even() {
            return false;
        }

        // If the last digit is 5 or 0, divisible by 5
        let last = self.digits[self.digits.len() - 1];
        if self.digits.len() > 1 && (last == 5 || last == 0) {
            return false;
        }

        // Sum of digits divisible by 3 -> divisible by 3
        let mut sum = BigInt::from_int(0);
        for &d in &self.digits {
            sum = sum.add(&BigInt::from_int(d as i64));
        }
        let three = BigInt::from_int(3);
        if sum.r#mod(&three).is_zero() {
            return false;
        }

        let sqrt_n = self.sqrt();
        let mut i = BigInt::from_int(2);
        while i <= sqrt_n {
            if self.r#mod(&i).is_zero() {
                return false;
            }
            i.inc();
        }
        true
    }
}

pub fn gt(a: &BigInt, b: &BigInt) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.remove_leading_zeros();
    b.remove_leading_zeros();
    if a.digits.len() > b.digits.len() {
        return !a.negative;
    }
    if a.digits.len() < b.digits.len() {
        return a.negative;
    }
    // Equal length
    for i in 0..a.digits.len() {
        if a.digits[i] > b.digits[i] {
            return !a.negative;
        }
        if a.digits[i] < b.digits[i] {
            return a.negative;
        }
    }
    false
}

pub fn lt(a: &BigInt, b: &BigInt) -> bool {
    !ge(a, b)
}

pub fn ge(a: &BigInt, b: &BigInt) -> bool {
    gt(a, b) || a == b
}

pub fn le(a: &BigInt, b: &BigInt) -> bool {
    !gt(a, b)
}

pub fn delete(a: &mut BigInt) {
    a.digits.clear();
    a.negative = false;
}

impl std::ops::Neg for BigInt {
    type Output = BigInt;
    fn neg(self) -> BigInt {
        let mut r = self;
        // Don't flip sign of zero
        if !(r.digits.len() == 1 && r.digits[0] == 0) {
            r.negative = !r.negative;
        }
        r
    }
}

impl PartialEq for BigInt {
    fn eq(&self, other: &Self) -> bool {
        let mut a = self.clone();
        let mut b = other.clone();
        a.remove_leading_zeros();
        b.remove_leading_zeros();
        if a.digits.len() != b.digits.len() {
            return false;
        }
        // Treat 0 specially: 0 == -0
        let a_is_zero = a.digits.len() == 1 && a.digits[0] == 0;
        let b_is_zero = b.digits.len() == 1 && b.digits[0] == 0;
        if a_is_zero && b_is_zero {
            return true;
        }
        if a.negative != b.negative {
            return false;
        }
        a.digits == b.digits
    }
}

impl Eq for BigInt {}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if gt(self, other) {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    }
}

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.negative {
            // Don't show negative sign for zero
            let is_zero = self.digits.iter().all(|&d| d == 0);
            if !is_zero {
                write!(f, "-")?;
            }
        }
        for &d in &self.digits {
            write!(f, "{}", d)?;
        }
        Ok(())
    }
}
