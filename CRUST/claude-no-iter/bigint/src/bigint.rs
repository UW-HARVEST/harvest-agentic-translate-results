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
        // C version: return false if size > 10 or size == 10, else true.
        // i.e., size < 10
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
        if leading_zeros >= self.digits.len() {
            // All zeros: collapse to a single 0 and clear sign
            self.digits = vec![0];
            self.negative = false;
            return;
        }
        if leading_zeros > 0 {
            self.digits.drain(0..leading_zeros);
        }
    }
    pub fn from_int(n: i64) -> Self {
        let (negative, mut m) = if n < 0 {
            (true, -(n as i128) as u128)
        } else {
            (false, n as u128)
        };
        let mut digits = Vec::new();
        if m == 0 {
            digits.push(0);
        } else {
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
            result = result.wrapping_neg();
        }
        result
    }
    pub fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let (negative, start) = if !bytes.is_empty() && bytes[0] == b'-' {
            (true, 1usize)
        } else {
            (false, 0usize)
        };
        let mut digits: Vec<u8> = Vec::with_capacity(bytes.len() - start);
        for &b in &bytes[start..] {
            digits.push(b - b'0');
        }
        if digits.is_empty() {
            digits.push(0);
        }
        BigInt { negative, digits }
    }
    pub fn copy(&self) -> BigInt {
        self.clone()
    }
    pub fn print(&self) {
        print!("{}", self);
    }
    pub fn is_zero(&self) -> bool {
        let mut x = self.clone();
        x.remove_leading_zeros();
        x.digits.len() == 1 && x.digits[0] == 0
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
        if self.negative && other.negative {
            // result.is_negative = true; then add digits as below
            let max_size = self.digits.len().max(other.digits.len());
            let mut digits = vec![0u8; max_size];
            let mut carry: u32 = 0;
            for i in 0..max_size {
                let mut sum: u32 = carry;
                if i < self.digits.len() {
                    sum += self.digits[self.digits.len() - i - 1] as u32;
                }
                if i < other.digits.len() {
                    sum += other.digits[other.digits.len() - i - 1] as u32;
                }
                digits[max_size - i - 1] = (sum % 10) as u8;
                carry = sum / 10;
            }
            if carry > 0 {
                let mut prepend: Vec<u8> = Vec::new();
                let mut c = carry;
                while c > 0 {
                    prepend.push((c % 10) as u8);
                    c /= 10;
                }
                prepend.reverse();
                let mut new_digits = prepend;
                new_digits.extend(digits);
                digits = new_digits;
            }
            let mut r = BigInt {
                negative: true,
                digits,
            };
            r.remove_leading_zeros();
            r
        } else if self.negative {
            // a is negative, b is not: result = b - |a|
            let a_abs = self.abs();
            other.sub(&a_abs)
        } else if other.negative {
            // b is negative, a is not: result = a - |b|
            let b_abs = other.abs();
            self.sub(&b_abs)
        } else {
            // Both non-negative
            let max_size = self.digits.len().max(other.digits.len());
            let mut digits = vec![0u8; max_size];
            let mut carry: u32 = 0;
            for i in 0..max_size {
                let mut sum: u32 = carry;
                if i < self.digits.len() {
                    sum += self.digits[self.digits.len() - i - 1] as u32;
                }
                if i < other.digits.len() {
                    sum += other.digits[other.digits.len() - i - 1] as u32;
                }
                digits[max_size - i - 1] = (sum % 10) as u8;
                carry = sum / 10;
            }
            if carry > 0 {
                let mut prepend: Vec<u8> = Vec::new();
                let mut c = carry;
                while c > 0 {
                    prepend.push((c % 10) as u8);
                    c /= 10;
                }
                prepend.reverse();
                let mut new_digits = prepend;
                new_digits.extend(digits);
                digits = new_digits;
            }
            let mut r = BigInt {
                negative: false,
                digits,
            };
            r.remove_leading_zeros();
            r
        }
    }
    pub fn sub(&self, other: &Self) -> Self {
        if self.negative && other.negative {
            // -a - (-b): C code computes |b| + |a| and negates (matches C bug behavior)
            let a_abs = self.abs();
            let b_abs = other.abs();
            let mut r = b_abs.add(&a_abs);
            r.negative = true;
            r.remove_leading_zeros();
            r
        } else if self.negative {
            // -a - b = -(a+b): C does b.is_negative=true; add(a,b) [both negative]
            let a = self.clone();
            let mut b = other.clone();
            b.negative = true;
            a.add(&b)
        } else if other.negative {
            // a - (-b) = a + b
            let a = self.clone();
            let b = other.abs();
            a.add(&b)
        } else {
            // Both non-negative
            let max_size = self.digits.len().max(other.digits.len());
            let mut digits = vec![0u8; max_size];
            let is_negative = lt(self, other);
            let (larger, smaller) = if gt(self, other) {
                (self, other)
            } else {
                (other, self)
            };
            let mut carry: i32 = 0;
            for i in 0..max_size {
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
                digits[max_size - i - 1] = diff as u8;
            }
            let mut r = BigInt {
                negative: is_negative,
                digits,
            };
            r.remove_leading_zeros();
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
        let result_size = self.digits.len() + other.digits.len();
        let mut acc: Vec<u64> = vec![0u64; result_size];
        for i in 0..self.digits.len() {
            for j in 0..other.digits.len() {
                let idx = result_size - i - j - 1;
                acc[idx] += (self.digits[self.digits.len() - i - 1] as u64)
                    * (other.digits[other.digits.len() - j - 1] as u64);
            }
        }
        // Normalize from least to most significant
        let mut digits = vec![0u8; result_size];
        let mut carry: u64 = 0;
        for i in 0..result_size {
            let sum = acc[result_size - i - 1] + carry;
            digits[result_size - i - 1] = (sum % 10) as u8;
            carry = sum / 10;
        }
        if carry > 0 {
            let mut prepend: Vec<u8> = Vec::new();
            let mut c = carry;
            while c > 0 {
                prepend.push((c % 10) as u8);
                c /= 10;
            }
            prepend.reverse();
            let mut new_digits = prepend;
            new_digits.extend(digits);
            digits = new_digits;
        }
        let negative = self.negative != other.negative;
        let mut r = BigInt { negative, digits };
        r.remove_leading_zeros();
        r
    }
    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        if self.is_64_bit() && other.is_64_bit() {
            let n = self.to_int();
            let d = other.to_int();
            let q = n / d;
            let r = n % d;
            return (BigInt::from_int(q), BigInt::from_int(r));
        }
        let mut quotient = BigInt::from_str("0");
        let negative = self.negative != other.negative;
        let mut numerator = self.abs();
        let denominator = other.abs();

        // Use long-division approach for efficiency on large numbers.
        // The C code uses repeated subtraction; we implement long division
        // (digit-by-digit) which is equivalent in result but much faster.
        if !numerator.is_zero() && !denominator.is_zero() {
            let mut current = BigInt::from_int(0);
            let mut q_digits: Vec<u8> = Vec::with_capacity(numerator.digits.len());
            for i in 0..numerator.digits.len() {
                // current = current * 10 + numerator.digits[i]
                let mut new_digits = current.digits.clone();
                new_digits.push(numerator.digits[i]);
                current = BigInt {
                    negative: false,
                    digits: new_digits,
                };
                current.remove_leading_zeros();

                // Find the largest k such that denominator * k <= current
                let mut k: u8 = 0;
                while k < 9 {
                    let next_k = k + 1;
                    let candidate = denominator.mul(&BigInt::from_int(next_k as i64));
                    if gt(&candidate, &current) {
                        break;
                    }
                    k = next_k;
                }
                if k > 0 {
                    let sub_amount = denominator.mul(&BigInt::from_int(k as i64));
                    current = current.sub(&sub_amount);
                }
                q_digits.push(k);
            }
            quotient = BigInt {
                negative: false,
                digits: q_digits,
            };
            quotient.remove_leading_zeros();
            numerator = current;
        } else if denominator.is_zero() {
            // Match C behavior: division by zero (undefined). Return 0/0.
            return (BigInt::from_int(0), BigInt::from_int(0));
        }

        let mut remainder = numerator;
        if negative {
            quotient.negative = true;
            remainder.negative = true;
        }
        // Avoid -0
        if quotient.is_zero() {
            quotient.negative = false;
        }
        if remainder.is_zero() {
            remainder.negative = false;
        }
        (quotient, remainder)
    }
    pub fn div(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() / other.to_int());
        }
        let (q, _r) = self.divmod(other);
        q
    }
    pub fn r#mod(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() % other.to_int());
        }
        let (_q, r) = self.divmod(other);
        r
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
            let mut result: i128 = 1;
            let mut base: i128 = self.to_int() as i128;
            let mut exp: i128 = exponent.to_int() as i128;
            let modulus: i128 = modulo.to_int() as i128;
            base = base.rem_euclid(modulus);
            while exp > 0 {
                if exp % 2 == 1 {
                    result = (result * base).rem_euclid(modulus);
                }
                exp >>= 1;
                base = (base * base).rem_euclid(modulus);
            }
            // Match C semantics for sign of result
            // C uses signed arithmetic; for non-negative inputs the result is non-negative.
            return BigInt::from_int(result as i64);
        }
        if exponent.negative {
            return BigInt::from_str("0");
        }
        if exponent.is_zero() {
            return BigInt::from_str("1");
        }

        // Square-and-multiply (right-to-left binary exponentiation)
        let mut base = self.r#mod(modulo);
        let mut exp = exponent.clone();
        let mut result = BigInt::from_int(1);
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
        let mut y = BigInt::from_str("0");
        let mut x = BigInt::from_str("1");
        let one = BigInt::from_str("1");
        let mut a = self.clone();
        let mut m = modulo.clone();

        while !a.is_zero() {
            let q = m.div(&a);
            let qa = q.mul(&a);
            let t1 = m.sub(&qa);

            // m = a; a = t1
            m = a.clone();
            a = t1;

            // t = y - q*x
            let qx = q.mul(&x);
            let t2 = y.sub(&qx);

            // y = x; x = t2
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
        let last = *self.digits.last().unwrap_or(&0);
        last % 2 == 0
    }
    pub fn is_odd(&self) -> bool {
        let last = *self.digits.last().unwrap_or(&0);
        last % 2 == 1
    }
    pub fn is_prime(&self) -> bool {
        // Even check
        if self.is_even() {
            return false;
        }
        // Last digit divisible by 5 or 0 with size > 1
        if self.digits.len() > 1 {
            let last = *self.digits.last().unwrap();
            if last == 5 || last == 0 {
                return false;
            }
        }
        // Sum of digits divisible by 3
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
    // After remove_leading_zeros, sign might have been normalized to false for zero.
    if a.digits.len() > b.digits.len() {
        return !a.negative;
    }
    if a.digits.len() < b.digits.len() {
        return a.negative;
    }
    // Same length; if signs differ, positive > negative
    if a.negative != b.negative {
        // a is greater iff a is non-negative
        return !a.negative;
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
        let mut r = self;
        if !r.is_zero() {
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
        if a.negative != b.negative {
            return false;
        }
        for i in 0..a.digits.len() {
            if a.digits[i] != b.digits[i] {
                return false;
            }
        }
        true
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
        let mut x = self.clone();
        x.remove_leading_zeros();
        if x.negative && !(x.digits.len() == 1 && x.digits[0] == 0) {
            write!(f, "-")?;
        }
        for d in &x.digits {
            write!(f, "{}", d)?;
        }
        Ok(())
    }
}
