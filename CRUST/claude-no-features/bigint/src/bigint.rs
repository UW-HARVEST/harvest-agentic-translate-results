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
        // Match C bigint_is_64_bit: returns true if size < 10, false otherwise.
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
                self.digits.clear();
                self.digits.push(0);
            } else {
                self.digits.drain(0..leading);
            }
        }
    }

    pub fn from_int(n: i64) -> Self {
        let negative = n < 0;
        // Compute absolute value as u64 (handles i64::MIN).
        let mut m: u64 = if n < 0 {
            (n as i128).unsigned_abs() as u64
        } else {
            n as u64
        };
        if m == 0 {
            return BigInt {
                negative: false,
                digits: vec![0],
            };
        }
        let mut digits: Vec<u8> = Vec::new();
        while m > 0 {
            digits.push((m % 10) as u8);
            m /= 10;
        }
        digits.reverse();
        let mut r = BigInt { negative, digits };
        r.remove_leading_zeros();
        r
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
        let (negative, rest) = if let Some(stripped) = s.strip_prefix('-') {
            (true, stripped)
        } else {
            (false, s)
        };
        let mut digits: Vec<u8> = rest.bytes().map(|b| b - b'0').collect();
        if digits.is_empty() {
            digits.push(0);
        }
        let mut r = BigInt { negative, digits };
        r.remove_leading_zeros();
        r
    }

    pub fn copy(&self) -> BigInt {
        self.clone()
    }

    pub fn print(&self) {
        print!("{}", self);
    }

    pub fn is_zero(&self) -> bool {
        let mut a = self.clone();
        a.remove_leading_zeros();
        a.digits.len() == 1 && a.digits[0] == 0
    }

    pub fn lt_zero(&self) -> bool {
        if self.is_zero() {
            false
        } else {
            self.negative
        }
    }

    pub fn gt_zero(&self) -> bool {
        if self.is_zero() {
            false
        } else {
            !self.negative
        }
    }

    pub fn lezero(&self) -> bool {
        if self.is_zero() {
            true
        } else {
            self.negative
        }
    }

    pub fn gezero(&self) -> bool {
        if self.is_zero() {
            true
        } else {
            !self.negative
        }
    }

    pub fn abs(&self) -> Self {
        let mut r = self.clone();
        r.negative = false;
        r
    }

    fn add_magnitudes(a: &[u8], b: &[u8]) -> Vec<u8> {
        let asz = a.len();
        let bsz = b.len();
        let size = asz.max(bsz);
        let mut digits = vec![0u8; size];
        let mut carry: u32 = 0;
        for i in 0..size {
            let mut sum: u32 = carry;
            if i < asz {
                sum += a[asz - i - 1] as u32;
            }
            if i < bsz {
                sum += b[bsz - i - 1] as u32;
            }
            digits[size - i - 1] = (sum % 10) as u8;
            carry = sum / 10;
        }
        if carry > 0 {
            // carry is small (max 1 for digit addition).
            digits.insert(0, carry as u8);
        }
        digits
    }

    fn cmp_magnitudes(a: &[u8], b: &[u8]) -> Ordering {
        // Compare digit slices ignoring any leading zeros.
        let mut a_start = 0;
        while a_start < a.len() && a[a_start] == 0 {
            a_start += 1;
        }
        let mut b_start = 0;
        while b_start < b.len() && b[b_start] == 0 {
            b_start += 1;
        }
        let a_len = a.len() - a_start;
        let b_len = b.len() - b_start;
        if a_len != b_len {
            return a_len.cmp(&b_len);
        }
        for i in 0..a_len {
            if a[a_start + i] != b[b_start + i] {
                return a[a_start + i].cmp(&b[b_start + i]);
            }
        }
        Ordering::Equal
    }

    fn sub_magnitudes(a: &[u8], b: &[u8]) -> (Vec<u8>, bool) {
        // Returns ( ||a| - |b|| , a_lt_b ).
        let cmp = Self::cmp_magnitudes(a, b);
        let is_negative = cmp == Ordering::Less;
        let (larger, smaller) = if cmp == Ordering::Greater {
            (a, b)
        } else {
            (b, a)
        };
        let lsz = larger.len();
        let ssz = smaller.len();
        let size = lsz;
        let mut digits = vec![0u8; size];
        let mut carry: i32 = 0;
        for i in 0..size {
            let mut diff: i32 = carry;
            if i < lsz {
                diff += larger[lsz - i - 1] as i32;
            }
            if i < ssz {
                diff -= smaller[ssz - i - 1] as i32;
            }
            if diff < 0 {
                diff += 10;
                carry = -1;
            } else {
                carry = 0;
            }
            digits[size - i - 1] = diff as u8;
        }
        (digits, is_negative)
    }

    pub fn add(&self, other: &Self) -> Self {
        let r = match (self.negative, other.negative) {
            (true, true) => {
                let digits = Self::add_magnitudes(&self.digits, &other.digits);
                BigInt {
                    negative: true,
                    digits,
                }
            }
            (true, false) => {
                // -|a| + |b| = |b| - |a|
                let (digits, b_lt_a) = Self::sub_magnitudes(&other.digits, &self.digits);
                BigInt {
                    negative: b_lt_a,
                    digits,
                }
            }
            (false, true) => {
                // |a| + (-|b|) = |a| - |b|
                let (digits, a_lt_b) = Self::sub_magnitudes(&self.digits, &other.digits);
                BigInt {
                    negative: a_lt_b,
                    digits,
                }
            }
            (false, false) => {
                let digits = Self::add_magnitudes(&self.digits, &other.digits);
                BigInt {
                    negative: false,
                    digits,
                }
            }
        };
        let mut out = r;
        out.remove_leading_zeros();
        if out.digits.len() == 1 && out.digits[0] == 0 {
            out.negative = false;
        }
        out
    }

    pub fn sub(&self, other: &Self) -> Self {
        let r = match (self.negative, other.negative) {
            (true, true) => {
                // Match the C implementation exactly: result = -(|a| + |b|).
                let digits = Self::add_magnitudes(&self.digits, &other.digits);
                BigInt {
                    negative: true,
                    digits,
                }
            }
            (true, false) => {
                // -|a| - |b| = -(|a| + |b|)
                let digits = Self::add_magnitudes(&self.digits, &other.digits);
                BigInt {
                    negative: true,
                    digits,
                }
            }
            (false, true) => {
                // |a| - (-|b|) = |a| + |b|
                let digits = Self::add_magnitudes(&self.digits, &other.digits);
                BigInt {
                    negative: false,
                    digits,
                }
            }
            (false, false) => {
                let (digits, a_lt_b) = Self::sub_magnitudes(&self.digits, &other.digits);
                BigInt {
                    negative: a_lt_b,
                    digits,
                }
            }
        };
        let mut out = r;
        out.remove_leading_zeros();
        if out.digits.len() == 1 && out.digits[0] == 0 {
            out.negative = false;
        }
        out
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
            return BigInt::from_int(self.to_int() * other.to_int());
        }
        let asz = self.digits.len();
        let bsz = other.digits.len();
        let rsz = asz + bsz;
        let mut buf: Vec<i64> = vec![0; rsz];
        for i in 0..asz {
            for j in 0..bsz {
                buf[rsz - i - j - 1] +=
                    (self.digits[asz - i - 1] as i64) * (other.digits[bsz - j - 1] as i64);
            }
        }
        // Normalize
        let mut carry: i64 = 0;
        for i in 0..rsz {
            let sum = buf[rsz - i - 1] + carry;
            buf[rsz - i - 1] = sum % 10;
            carry = sum / 10;
        }
        let mut digits: Vec<u8> = buf.into_iter().map(|x| x as u8).collect();
        if carry > 0 {
            // Prepend remaining digits from carry.
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
            negative: self.negative != other.negative,
            digits,
        };
        r.remove_leading_zeros();
        if r.digits.len() == 1 && r.digits[0] == 0 {
            r.negative = false;
        }
        r
    }

    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        if self.is_64_bit() && other.is_64_bit() {
            let n = self.to_int();
            let d = other.to_int();
            let q = n / d;
            let rm = n % d;
            return (BigInt::from_int(q), BigInt::from_int(rm));
        }

        let mut quotient = BigInt::from_str("0");
        let negative = self.negative != other.negative;
        let mut numerator = self.abs();
        let denominator = other.abs();

        // Slow subtraction-based division (matches C reference algorithm).
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
            if !(quotient.digits.len() == 1 && quotient.digits[0] == 0) {
                quotient.negative = true;
            }
            if !(numerator.digits.len() == 1 && numerator.digits[0] == 0) {
                numerator.negative = true;
            }
        }

        (quotient, numerator)
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
            return BigInt::from_str("0");
        }
        let mut result = BigInt::from_str("1");
        if exponent.is_zero() {
            return result;
        }
        let mut e = exponent.clone();
        let one = BigInt::from_int(1);
        while !e.is_zero() {
            result = result.mul(self);
            e = e.sub(&one);
        }
        result
    }

    pub fn fast_pow(&self, exponent: &Self, modulo: &Self) -> Self {
        if self.is_64_bit() && exponent.is_64_bit() && modulo.is_64_bit() {
            let mut result: i64 = 1;
            let mut base = self.to_int();
            let mut exp = exponent.to_int();
            let m = modulo.to_int();
            base %= m;
            if base < 0 {
                base += m;
            }
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
        let one = BigInt::from_str("1");
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
        while low < high {
            let sum = low.add(&high);
            let mid = sum.div(&two);
            let sq = mid.mul(&mid);
            if sq < *self {
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
        // Sum of digits divisibility-by-three check.
        let mut sum = BigInt::from_str("0");
        for &d in &self.digits {
            let dd = BigInt::from_int(d as i64);
            sum = sum.add(&dd);
        }
        let three = BigInt::from_str("3");
        let s_mod3 = sum.r#mod(&three);
        if s_mod3.is_zero() {
            return false;
        }

        let sqrt_n = self.sqrt();
        let mut i = BigInt::from_str("2");
        while i <= sqrt_n {
            let r = self.r#mod(&i);
            if r.is_zero() {
                return false;
            }
            i.inc();
        }
        true
    }
}

fn gt_internal(a: &BigInt, b: &BigInt) -> bool {
    let mut a = a.clone();
    a.remove_leading_zeros();
    let mut b = b.clone();
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

fn eq_internal(a: &BigInt, b: &BigInt) -> bool {
    let mut a = a.clone();
    a.remove_leading_zeros();
    let mut b = b.clone();
    b.remove_leading_zeros();
    if a.digits.len() != b.digits.len() {
        return false;
    }
    if a.negative != b.negative {
        return false;
    }
    a.digits == b.digits
}

pub fn gt(a: &BigInt, b: &BigInt) -> bool {
    gt_internal(a, b)
}

pub fn lt(a: &BigInt, b: &BigInt) -> bool {
    !ge(a, b)
}

pub fn ge(a: &BigInt, b: &BigInt) -> bool {
    gt_internal(a, b) || eq_internal(a, b)
}

pub fn le(a: &BigInt, b: &BigInt) -> bool {
    !gt_internal(a, b)
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
        eq_internal(self, other)
    }
}

impl Eq for BigInt {}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if eq_internal(self, other) {
            return Some(Ordering::Equal);
        }
        if gt_internal(self, other) {
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
