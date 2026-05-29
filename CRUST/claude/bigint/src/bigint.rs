use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Debug)]
pub struct BigInt {
    negative: bool,
    digits: Vec<u8>,
}

fn digit_add(a: &[u8], b: &[u8]) -> Vec<u8> {
    let result_size = a.len().max(b.len());
    let mut result = vec![0u8; result_size];
    let mut carry: u16 = 0;
    for i in 0..result_size {
        let mut sum = carry;
        if i < a.len() {
            sum += a[a.len() - i - 1] as u16;
        }
        if i < b.len() {
            sum += b[b.len() - i - 1] as u16;
        }
        result[result_size - i - 1] = (sum % 10) as u8;
        carry = sum / 10;
    }
    if carry > 0 {
        let mut new_result = Vec::with_capacity(result_size + 1);
        new_result.push(carry as u8);
        new_result.extend_from_slice(&result);
        new_result
    } else {
        result
    }
}

impl BigInt {
    pub fn zero() -> Self {
        BigInt {
            negative: false,
            digits: vec![0],
        }
    }
    pub fn is_64_bit(&self) -> bool {
        // C returns true iff size < 10
        self.digits.len() < 10
    }
    pub fn remove_leading_zeros(&mut self) {
        let mut leading = 0usize;
        for &d in &self.digits {
            if d == 0 {
                leading += 1;
            } else {
                break;
            }
        }
        if leading > 0 {
            if leading >= self.digits.len() {
                self.digits = vec![0];
                return;
            }
            self.digits.drain(0..leading);
        }
    }
    pub fn from_int(n: i64) -> Self {
        let (negative, mag) = if n < 0 {
            (true, n.unsigned_abs())
        } else {
            (false, n as u64)
        };
        let mut digits = Vec::new();
        if mag == 0 {
            digits.push(0u8);
        } else {
            let mut m = mag;
            while m > 0 {
                digits.push((m % 10) as u8);
                m /= 10;
            }
            digits.reverse();
        }
        let mut result = BigInt { negative, digits };
        result.remove_leading_zeros();
        result
    }
    pub fn to_int(&self) -> i64 {
        let mut result: i64 = 0;
        for &d in &self.digits {
            result = result.wrapping_mul(10).wrapping_add(d as i64);
        }
        if self.negative {
            result.wrapping_neg()
        } else {
            result
        }
    }
    pub fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let (negative, start) = if !bytes.is_empty() && bytes[0] == b'-' {
            (true, 1)
        } else {
            (false, 0)
        };
        let digits: Vec<u8> = bytes[start..].iter().map(|&b| b - b'0').collect();
        BigInt { negative, digits }
    }
    pub fn copy(&self) -> BigInt {
        self.clone()
    }
    pub fn print(&self) {
        print!("{}", self);
    }
    pub fn is_zero(&self) -> bool {
        let mut t = self.clone();
        t.remove_leading_zeros();
        t.digits.len() == 1 && t.digits[0] == 0
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
        BigInt {
            negative: false,
            digits: self.digits.clone(),
        }
    }
    pub fn add(&self, other: &Self) -> Self {
        let a = self;
        let b = other;
        if a.negative && b.negative {
            let digits = digit_add(&a.digits, &b.digits);
            let mut r = BigInt {
                negative: true,
                digits,
            };
            r.remove_leading_zeros();
            return r;
        }
        if a.negative {
            // -|a| + b = b - |a|
            let pa = BigInt {
                negative: false,
                digits: a.digits.clone(),
            };
            return b.sub(&pa);
        }
        if b.negative {
            // a + (-|b|) = a - |b|
            let pb = BigInt {
                negative: false,
                digits: b.digits.clone(),
            };
            return a.sub(&pb);
        }
        let digits = digit_add(&a.digits, &b.digits);
        let mut r = BigInt {
            negative: false,
            digits,
        };
        r.remove_leading_zeros();
        r
    }
    pub fn sub(&self, other: &Self) -> Self {
        let mut a = self.clone();
        let mut b = other.clone();
        if a.negative && b.negative {
            // Match C's (buggy) behavior: returns -(|a| + |b|)
            a.negative = false;
            b.negative = false;
            let mut result = b.add(&a);
            result.negative = true;
            return result;
        }
        if a.negative {
            // -|a| - b = -(|a| + b): make b negative and call add
            b.negative = true;
            return a.add(&b);
        }
        if b.negative {
            // a - (-|b|) = a + |b|
            b.negative = false;
            return a.add(&b);
        }
        // Both positive
        let result_size = a.digits.len().max(b.digits.len());
        let mut result_digits = vec![0u8; result_size];
        let is_negative = lt(&a, &b);
        let (larger, smaller) = if gt(&a, &b) { (&a, &b) } else { (&b, &a) };
        let mut carry: i32 = 0;
        for i in 0..result_size {
            let mut diff: i32 = carry;
            if i < larger.digits.len() {
                diff += larger.digits[larger.digits.len() - i - 1] as i32;
            }
            if i < smaller.digits.len() {
                diff -= smaller.digits[smaller.digits.len() - i - 1] as i32;
            }
            if diff < 0 {
                diff += 10;
                carry = -1;
            } else {
                carry = 0;
            }
            result_digits[result_size - i - 1] = diff as u8;
        }
        let mut r = BigInt {
            negative: is_negative,
            digits: result_digits,
        };
        r.remove_leading_zeros();
        r
    }
    pub fn inc(&mut self) {
        let one = BigInt::from_str("1");
        *self = self.add(&one);
    }
    pub fn dec(&mut self) {
        let one = BigInt::from_str("1");
        *self = self.sub(&one);
    }
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int().wrapping_mul(other.to_int()));
        }
        let a = &self.digits;
        let b = &other.digits;
        let result_size = a.len() + b.len();
        let mut work: Vec<i64> = vec![0; result_size];
        for i in 0..a.len() {
            for j in 0..b.len() {
                let idx = result_size - i - j - 1;
                work[idx] += (a[a.len() - i - 1] as i64) * (b[b.len() - j - 1] as i64);
            }
        }
        // Normalize
        let mut carry: i64 = 0;
        for i in 0..result_size {
            let sum = work[result_size - i - 1] + carry;
            work[result_size - i - 1] = sum % 10;
            carry = sum / 10;
        }
        let mut digits: Vec<u8> = work.iter().map(|&x| x as u8).collect();
        if carry > 0 {
            let mut new_digits = Vec::with_capacity(digits.len() + 1);
            new_digits.push(carry as u8);
            new_digits.extend_from_slice(&digits);
            digits = new_digits;
        }
        let mut r = BigInt {
            negative: self.negative != other.negative,
            digits,
        };
        r.remove_leading_zeros();
        r
    }
    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        if self.is_64_bit() && other.is_64_bit() {
            let n = self.to_int();
            let d = other.to_int();
            return (BigInt::from_int(n / d), BigInt::from_int(n % d));
        }
        let mut quotient = BigInt::from_str("0");
        let negative = self.negative != other.negative;
        let mut numerator = self.clone();
        let mut denominator = other.clone();
        numerator.negative = false;
        denominator.negative = false;

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
        quotient.remove_leading_zeros();
        if negative {
            quotient.negative = true;
            numerator.negative = true;
        }
        (quotient, numerator)
    }
    pub fn div(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() / other.to_int());
        }
        self.divmod(other).0
    }
    pub fn r#mod(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() % other.to_int());
        }
        self.divmod(other).1
    }
    pub fn pow(&self, exponent: &Self) -> Self {
        if exponent.negative {
            return BigInt::from_str("0");
        }
        let mut result = BigInt::from_str("1");
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
            let m = modulo.to_int();
            let mut base = self.to_int() % m;
            let mut exp = exponent.to_int();
            while exp > 0 {
                if exp % 2 == 1 {
                    result = (result * base) % m;
                }
                exp >>= 1;
                base = (base * base) % m;
            }
            return BigInt::from_int(result);
        }

        if exponent.negative {
            return BigInt::from_str("0");
        }
        if exponent.is_zero() {
            return BigInt::from_str("1");
        }
        let a = self.r#mod(modulo);
        let mut b = exponent.clone();
        let mut result = a.clone();
        let mut b_save = b.clone();
        b.dec();
        let mut pow_count = BigInt::from_str("1");
        let two = BigInt::from_str("2");

        loop {
            let half_b = b.div(&two);
            if !half_b.gt_zero() {
                break;
            }
            let squared = result.mul(&result);
            result = squared.r#mod(modulo);
            b = half_b;
            pow_count = pow_count.add(&pow_count);
        }

        b_save = b_save.sub(&pow_count);

        if b_save.is_odd() {
            let prod = result.mul(&a);
            result = prod.r#mod(modulo);
            b_save.dec();
            pow_count.inc();
        }

        if lt(&b_save, &BigInt::from_int(5)) {
            while !b_save.is_zero() {
                let prod = result.mul(&a);
                result = prod.r#mod(modulo);
                b_save.dec();
                pow_count.inc();
            }
            return result;
        }

        let subpart = a.fast_pow(&b_save, modulo);
        result = result.mul(&subpart);
        result = result.r#mod(modulo);
        result
    }
    pub fn modinv(&self, modulo: &Self) -> Self {
        let m0 = modulo.clone();
        let mut y = BigInt::from_str("0");
        let mut x = BigInt::from_str("1");
        let one = BigInt::from_str("1");
        let mut a = self.clone();
        let mut m = modulo.clone();

        while !a.is_zero() {
            let q = m.div(&a);
            let t = m.sub(&q.mul(&a));
            m = a.clone();
            a = t;
            let t2 = y.sub(&q.mul(&x));
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
        let one = BigInt::from_str("1");
        let two = BigInt::from_str("2");
        let mut low = BigInt::from_str("0");
        let mut high = self.clone();
        while lt(&low, &high) {
            let sum = low.add(&high);
            let mid = sum.div(&two);
            let sq = mid.mul(&mid);
            if lt(&sq, self) {
                low = mid.add(&one);
            } else {
                high = mid;
            }
        }
        low
    }
    pub fn is_even(&self) -> bool {
        if self.digits.is_empty() {
            return true;
        }
        self.digits[self.digits.len() - 1] % 2 == 0
    }
    pub fn is_odd(&self) -> bool {
        if self.digits.is_empty() {
            return false;
        }
        self.digits[self.digits.len() - 1] % 2 == 1
    }
    pub fn is_prime(&self) -> bool {
        if self.is_even() {
            return false;
        }
        if self.digits.len() > 1 {
            let last = self.digits[self.digits.len() - 1];
            if last == 5 || last == 0 {
                return false;
            }
        }
        let mut sum = BigInt::from_str("0");
        for &d in &self.digits {
            sum = sum.add(&BigInt::from_int(d as i64));
        }
        let three = BigInt::from_str("3");
        if sum.r#mod(&three).is_zero() {
            return false;
        }
        let sqrt_n = self.sqrt();
        let mut i = BigInt::from_str("2");
        while le(&i, &sqrt_n) {
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
        BigInt {
            negative: !self.negative,
            digits: self.digits,
        }
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
            write!(f, "-")?;
        }
        for &d in &self.digits {
            write!(f, "{}", d)?;
        }
        Ok(())
    }
}
