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
        // Match C: returns true iff size < 10 (size == 10 returns false).
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
                self.digits = vec![0];
                return;
            }
            self.digits.drain(..leading_zeros);
        }
    }
    pub fn from_int(n: i64) -> Self {
        let (negative, mut m) = if n < 0 {
            // Use unsigned to handle i64::MIN safely.
            (true, (n as i128).unsigned_abs() as u128)
        } else {
            (false, n as u128)
        };
        let mut size: usize = 1;
        let mut tmp = m;
        while tmp > 0 {
            tmp /= 10;
            size += 1;
        }
        let mut digits = vec![0u8; size];
        for i in 0..size {
            digits[size - i - 1] = (m % 10) as u8;
            m /= 10;
        }
        let mut result = BigInt { negative, digits };
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
        let (negative, rest) = if let Some(stripped) = s.strip_prefix('-') {
            (true, stripped)
        } else {
            (false, s)
        };
        let digits: Vec<u8> = rest.bytes().map(|b| b - b'0').collect();
        BigInt { negative, digits }
    }
    pub fn copy(&self) -> BigInt {
        self.clone()
    }
    pub fn print(&self) {
        if self.negative {
            print!("-");
        }
        for &d in &self.digits {
            assert!(d <= 9);
            print!("{}", d);
        }
    }
    pub fn is_zero(&self) -> bool {
        let mut tmp = self.clone();
        tmp.remove_leading_zeros();
        tmp.digits.len() == 1 && tmp.digits[0] == 0
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
        let mut a = self.clone();
        let mut b = other.clone();
        let result_neg;
        if a.negative && b.negative {
            result_neg = true;
        } else if a.negative {
            // -|a| + b = b - |a|
            a.negative = false;
            return b.sub(&a);
        } else if b.negative {
            // a + -|b| = a - |b|
            b.negative = false;
            return a.sub(&b);
        } else {
            result_neg = false;
        }
        let n = a.digits.len().max(b.digits.len());
        let mut result_digits = vec![0u8; n];
        let mut carry: i64 = 0;
        for i in 0..n {
            let mut sum: i64 = carry;
            if i < a.digits.len() {
                sum += a.digits[a.digits.len() - i - 1] as i64;
            }
            if i < b.digits.len() {
                sum += b.digits[b.digits.len() - i - 1] as i64;
            }
            result_digits[n - i - 1] = (sum % 10) as u8;
            carry = sum / 10;
        }
        if carry > 0 {
            let mut v = Vec::with_capacity(result_digits.len() + 1);
            v.push(carry as u8);
            v.extend(result_digits);
            result_digits = v;
        }
        let mut result = BigInt {
            negative: result_neg,
            digits: result_digits,
        };
        result.remove_leading_zeros();
        result
    }
    pub fn sub(&self, other: &Self) -> Self {
        let mut a = self.clone();
        let mut b = other.clone();
        if a.negative && b.negative {
            // Match C: result = bigint_add(b, a) with absolute values; then result.is_negative = true.
            a.negative = false;
            b.negative = false;
            let mut result = b.add(&a);
            result.negative = true;
            return result;
        }
        if a.negative {
            // -|a| - b: C does b.is_negative = true; return bigint_add(a, b).
            // a is still -|a|, b becomes -b. So returns -|a| + -b = -(|a| + b).
            b.negative = true;
            return a.add(&b);
        }
        if b.negative {
            // a - (-|b|) = a + |b|
            b.negative = false;
            return a.add(&b);
        }
        // both non-negative
        let n = a.digits.len().max(b.digits.len());
        let mut result_digits = vec![0u8; n];
        let is_negative = lt(&a, &b);
        let (larger, smaller) = if gt(&a, &b) {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        };
        let mut carry: i64 = 0;
        for i in 0..n {
            let mut diff: i64 = carry;
            if i < larger.digits.len() {
                diff += larger.digits[larger.digits.len() - i - 1] as i64;
            }
            if i < smaller.digits.len() {
                diff -= smaller.digits[smaller.digits.len() - i - 1] as i64;
            }
            if diff < 0 {
                diff += 10;
                carry = -1;
            } else {
                carry = 0;
            }
            result_digits[n - i - 1] = diff as u8;
        }
        let mut result = BigInt {
            negative: is_negative,
            digits: result_digits,
        };
        result.remove_leading_zeros();
        result.negative = is_negative;
        result
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
            let a = self.to_int();
            let b = other.to_int();
            return BigInt::from_int(a.wrapping_mul(b));
        }
        let an = self.digits.len();
        let bn = other.digits.len();
        let n = an + bn;
        let mut acc: Vec<i64> = vec![0; n];
        for i in 0..an {
            for j in 0..bn {
                acc[n - i - j - 1] +=
                    (self.digits[an - i - 1] as i64) * (other.digits[bn - j - 1] as i64);
            }
        }
        // Normalize
        let mut carry: i64 = 0;
        let mut final_digits = vec![0u8; n];
        for i in 0..n {
            let sum = acc[n - i - 1] + carry;
            final_digits[n - i - 1] = (sum % 10) as u8;
            carry = sum / 10;
        }
        if carry > 0 {
            // C only stores carry as a single digit; in well-formed multiplications
            // (size = a.size + b.size) carry should be 0. Be safe and prepend digits.
            let mut prepend = Vec::new();
            let mut c = carry;
            while c > 0 {
                prepend.push((c % 10) as u8);
                c /= 10;
            }
            prepend.reverse();
            let mut full = prepend;
            full.extend(final_digits);
            final_digits = full;
        }
        let mut result = BigInt {
            negative: self.negative != other.negative,
            digits: final_digits,
        };
        result.remove_leading_zeros();
        // Match C: is_negative is set after remove_leading_zeros
        result.negative = self.negative != other.negative;
        result
    }
    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        if self.is_64_bit() && other.is_64_bit() {
            let n = self.to_int();
            let d = other.to_int();
            let q = n / d;
            let r = n % d;
            return (BigInt::from_int(q), BigInt::from_int(r));
        }
        let mut numerator = self.clone();
        let mut denominator = other.clone();
        let negative = numerator.negative != denominator.negative;
        numerator.negative = false;
        denominator.negative = false;
        let mut numerator = numerator.clone();
        let mut quotient = BigInt::from_str("0");
        // Slow repeated subtraction (matching C).
        while numerator.gt_zero() {
            let next = numerator.sub(&denominator);
            numerator = next;
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
        }
        (quotient, remainder)
    }
    pub fn div(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            let a = self.to_int();
            let b = other.to_int();
            return BigInt::from_int(a / b);
        }
        let (q, _r) = self.divmod(other);
        q
    }
    pub fn r#mod(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            let a = self.to_int();
            let b = other.to_int();
            return BigInt::from_int(a % b);
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
            let mut result: i64 = 1;
            let mut base: i64 = self.to_int();
            let mut exp: i64 = exponent.to_int();
            let m: i64 = modulo.to_int();
            base %= m;
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
            return BigInt::from_str("0");
        }
        if exponent.is_zero() {
            return BigInt::from_str("1");
        }
        let mut b = exponent.clone();
        let a = self.r#mod(modulo);
        let mut result = a.clone();
        let mut b_save = b.clone();
        b.dec();
        let mut pow_v = BigInt::from_str("1");
        loop {
            let two = BigInt::from_str("2");
            let halved = b.div(&two);
            if !halved.gt_zero() {
                break;
            }
            result = result.mul(&result).r#mod(modulo);
            b = b.div(&two);
            pow_v = pow_v.add(&pow_v);
        }
        b_save = b_save.sub(&pow_v);
        if b_save.is_odd() {
            result = result.mul(&a).r#mod(modulo);
            b_save.dec();
            pow_v.inc();
        }
        let five = BigInt::from_int(5);
        if lt(&b_save, &five) {
            while !b_save.is_zero() {
                result = result.mul(&a).r#mod(modulo);
                b_save.dec();
                pow_v.inc();
            }
            return result;
        }
        let subpart = a.fast_pow(&b_save, modulo);
        result = result.mul(&subpart).r#mod(modulo);
        result
    }
    pub fn modinv(&self, modulo: &Self) -> Self {
        let m0 = modulo.clone();
        let mut y = BigInt::from_str("0");
        let mut x = BigInt::from_str("1");
        let mut a = self.clone();
        let mut m = modulo.clone();
        while !a.is_zero() {
            let q = m.div(&a);
            let qa = q.mul(&a);
            let t = m.sub(&qa);
            m = a.clone();
            a = t.clone();
            let qx = q.mul(&x);
            let t2 = y.sub(&qx);
            y = x.clone();
            x = t2.clone();
        }
        if m.lt_zero() {
            m = m.add(&m0);
        }
        let one = BigInt::from_str("1");
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
                high = mid.clone();
            }
        }
        low
    }
    pub fn is_even(&self) -> bool {
        self.digits[self.digits.len() - 1] % 2 == 0
    }
    pub fn is_odd(&self) -> bool {
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
            let tmp = BigInt::from_int(d as i64);
            sum = sum.add(&tmp);
        }
        let three = BigInt::from_str("3");
        let r = sum.r#mod(&three);
        if r.is_zero() {
            return false;
        }
        let sqrt_n = self.sqrt();
        let mut i = BigInt::from_str("2");
        while le(&i, &sqrt_n) {
            let r2 = self.r#mod(&i);
            if r2.is_zero() {
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
pub fn delete(_a: &mut BigInt) {
    // No-op in Rust — Vec is dropped automatically.
}
impl std::ops::Neg for BigInt {
    type Output = BigInt;
    fn neg(self) -> BigInt {
        let mut result = self.clone();
        result.negative = !result.negative;
        result
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
        if self.negative {
            write!(f, "-")?;
        }
        for &d in &self.digits {
            write!(f, "{}", d)?;
        }
        Ok(())
    }
}
