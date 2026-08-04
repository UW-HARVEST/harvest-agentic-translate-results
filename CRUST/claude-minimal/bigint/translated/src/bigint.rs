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
        // Match C: size > 10 -> false, size < 10 -> true, otherwise false.
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
            self.digits.drain(0..leading_zeros);
        }
    }
    pub fn from_int(n: i64) -> Self {
        let negative = n < 0;
        let original: u64 = n.unsigned_abs();

        // Match C: size starts at 1, increments while m > 0.
        let mut m = original;
        let mut size: usize = 1;
        while m > 0 {
            m /= 10;
            size += 1;
        }

        let mut digits = vec![0u8; size];
        let mut m = original;
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
            result = -result;
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
        let digits: Vec<u8> = bytes[start..].iter().map(|&b| b - b'0').collect();
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
        let mut copy = self.clone();
        copy.remove_leading_zeros();
        copy.digits.len() == 1 && copy.digits[0] == 0
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
        let mut result = self.clone();
        result.negative = false;
        result
    }
    pub fn add(&self, other: &Self) -> Self {
        let a = self;
        let b = other;

        let result_negative;
        if a.negative && b.negative {
            result_negative = true;
        } else if a.negative {
            let mut na = a.clone();
            na.negative = false;
            return b.sub(&na);
        } else if b.negative {
            let mut nb = b.clone();
            nb.negative = false;
            return a.sub(&nb);
        } else {
            result_negative = false;
        }

        let size = a.digits.len().max(b.digits.len());
        let mut digits = vec![0u8; size];
        let mut carry: u8 = 0;
        for i in 0..size {
            let mut sum: u8 = carry;
            if i < a.digits.len() {
                sum += a.digits[a.digits.len() - i - 1];
            }
            if i < b.digits.len() {
                sum += b.digits[b.digits.len() - i - 1];
            }
            digits[size - i - 1] = sum % 10;
            carry = sum / 10;
        }
        if carry > 0 {
            digits.insert(0, carry);
        }
        let mut result = BigInt {
            negative: result_negative,
            digits,
        };
        result.remove_leading_zeros();
        result
    }
    pub fn sub(&self, other: &Self) -> Self {
        let a = self;
        let b = other;

        if a.negative && b.negative {
            // Match the C implementation's branch literally.
            let mut na = a.clone();
            na.negative = false;
            let mut nb = b.clone();
            nb.negative = false;
            let mut result = nb.add(&na);
            result.negative = true;
            return result;
        }

        if a.negative {
            // -a - b = -(a + b)
            let mut nb = b.clone();
            nb.negative = true;
            return a.add(&nb);
        }

        if b.negative {
            // a - (-b) = a + b
            let mut nb = b.clone();
            nb.negative = false;
            return a.add(&nb);
        }

        let size = a.digits.len().max(b.digits.len());
        let mut digits = vec![0u8; size];

        let is_negative = lt(a, b);

        let (larger, smaller) = if gt(a, b) { (a, b) } else { (b, a) };

        let mut carry: i32 = 0;
        for i in 0..size {
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
            digits[size - i - 1] = diff as u8;
        }

        let mut result = BigInt {
            negative: is_negative,
            digits,
        };
        result.remove_leading_zeros();
        result
    }
    pub fn inc(&mut self) {
        let one = BigInt::from_str("1");
        let result = self.add(&one);
        *self = result;
    }
    pub fn dec(&mut self) {
        let one = BigInt::from_str("1");
        let result = self.sub(&one);
        *self = result;
    }
    pub fn mul(&self, other: &Self) -> Self {
        let a = self;
        let b = other;

        if a.is_64_bit() && b.is_64_bit() {
            return BigInt::from_int(a.to_int() * b.to_int());
        }

        let size = a.digits.len() + b.digits.len();
        let mut digits = vec![0i64; size];

        for i in 0..a.digits.len() {
            for j in 0..b.digits.len() {
                let idx = size - i - j - 1;
                digits[idx] += a.digits[a.digits.len() - i - 1] as i64
                    * b.digits[b.digits.len() - j - 1] as i64;
            }
        }

        let mut carry: i64 = 0;
        for i in 0..size {
            let sum = digits[size - i - 1] + carry;
            digits[size - i - 1] = sum % 10;
            carry = sum / 10;
        }

        let mut result_digits: Vec<u8> = digits.iter().map(|&x| x as u8).collect();
        if carry > 0 {
            result_digits.insert(0, carry as u8);
        }

        let mut result = BigInt {
            negative: a.negative != b.negative,
            digits: result_digits,
        };
        result.remove_leading_zeros();
        result
    }
    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        let numerator = self;
        let denominator = other;

        if numerator.is_64_bit() && denominator.is_64_bit() {
            let n = numerator.to_int();
            let d = denominator.to_int();
            return (BigInt::from_int(n / d), BigInt::from_int(n % d));
        }

        let mut quotient = BigInt::from_str("0");
        let negative = numerator.negative != denominator.negative;
        let mut num = numerator.clone();
        num.negative = false;
        let mut den = denominator.clone();
        den.negative = false;

        while num.gt_zero() {
            num = num.sub(&den);
            if num.lt_zero() {
                break;
            }
            quotient.inc();
        }
        if num.lt_zero() {
            num = num.add(&den);
        }

        quotient.remove_leading_zeros();

        let mut remainder = num;

        if negative {
            quotient.negative = true;
            remainder.negative = true;
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
        let a = self;
        let b = exponent;
        if b.negative {
            return BigInt::from_str("0");
        }
        let mut result = BigInt::from_str("1");
        if b.is_zero() {
            return result;
        }
        let mut b = b.clone();
        while !b.is_zero() {
            result = result.mul(a);
            b.dec();
        }
        result
    }
    pub fn fast_pow(&self, exponent: &Self, modulo: &Self) -> Self {
        let a = self;
        let b = exponent;
        let m = modulo;

        if a.is_64_bit() && b.is_64_bit() && m.is_64_bit() {
            let mut result: i64 = 1;
            let mut base: i64 = a.to_int();
            let mut exp: i64 = b.to_int();
            let mod_val: i64 = m.to_int();
            base %= mod_val;
            while exp > 0 {
                if exp % 2 == 1 {
                    result = (result * base) % mod_val;
                }
                exp >>= 1;
                base = (base * base) % mod_val;
            }
            return BigInt::from_int(result);
        }

        if b.negative {
            return BigInt::from_str("0");
        }
        if b.is_zero() {
            return BigInt::from_str("1");
        }

        let mut result = BigInt::from_str("1");
        let mut base = a.r#mod(m);
        let mut exp = b.clone();
        let two = BigInt::from_int(2);

        while exp.gt_zero() {
            if exp.is_odd() {
                let temp = result.mul(&base);
                result = temp.r#mod(m);
            }
            exp = exp.div(&two);
            let temp = base.mul(&base);
            base = temp.r#mod(m);
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
            let t = m.sub(&qa);
            m = a;
            a = t;
            let qx = q.mul(&x);
            let t2 = y.sub(&qx);
            y = x;
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
            let mid_sq = mid.mul(&mid);
            if lt(&mid_sq, self) {
                low = mid.add(&one);
            } else {
                high = mid;
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
        let n = self;

        if n.is_even() {
            return false;
        }

        if n.digits.len() > 1 {
            let last = n.digits[n.digits.len() - 1];
            if last == 5 || last == 0 {
                return false;
            }
        }

        let mut sum = BigInt::from_str("0");
        for &d in &n.digits {
            let dn = BigInt::from_int(d as i64);
            sum = sum.add(&dn);
        }
        let three = BigInt::from_str("3");
        if sum.r#mod(&three).is_zero() {
            return false;
        }

        let sqrt_n = n.sqrt();
        let mut tmp = BigInt::from_str("2");
        while le(&tmp, &sqrt_n) {
            if n.r#mod(&tmp).is_zero() {
                return false;
            }
            tmp.inc();
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
    // No-op: Rust manages memory automatically.
}
impl std::ops::Neg for BigInt {
    type Output = BigInt;
    fn neg(self) -> BigInt {
        let mut result = self;
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
        if self.eq(other) {
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
