use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Debug)]
pub struct BigInt {
    negative: bool,
    digits: Vec<u8>,
}

// Helper: returns slice with leading zeros skipped, but always retaining at least one digit.
fn effective_slice(d: &[u8]) -> &[u8] {
    if d.is_empty() {
        return d;
    }
    let mut i = 0;
    while i + 1 < d.len() && d[i] == 0 {
        i += 1;
    }
    &d[i..]
}

impl BigInt {
    pub fn zero() -> Self {
        BigInt {
            negative: false,
            digits: vec![0],
        }
    }

    pub fn is_64_bit(&self) -> bool {
        // Match C semantics: returns true iff size < 10.
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
            self.digits.drain(..leading);
        }
    }

    pub fn from_int(n: i64) -> Self {
        let negative = n < 0;
        let m: u64 = if negative { n.unsigned_abs() } else { n as u64 };

        // Match C: starts size=1, while (m > 0) { m/=10; size++; }
        let mut size: usize = 1;
        let mut tmp = m;
        while tmp > 0 {
            tmp /= 10;
            size += 1;
        }
        let mut digits = vec![0u8; size];
        tmp = m;
        for i in 0..size {
            digits[size - i - 1] = (tmp % 10) as u8;
            tmp /= 10;
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
        let (negative, rest) = if !bytes.is_empty() && bytes[0] == b'-' {
            (true, &bytes[1..])
        } else {
            (false, bytes)
        };
        let digits: Vec<u8> = rest.iter().map(|&c| c.wrapping_sub(b'0')).collect();
        BigInt { negative, digits }
    }

    pub fn copy(&self) -> BigInt {
        self.clone()
    }

    pub fn print(&self) {
        print!("{}", self);
    }

    pub fn is_zero(&self) -> bool {
        // After removing leading zeros, size==1 and digits[0]==0,
        // equivalent to: all digits are zero.
        self.digits.iter().all(|&d| d == 0)
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
        BigInt::add_internal(self.clone(), other.clone())
    }

    pub fn sub(&self, other: &Self) -> Self {
        BigInt::sub_internal(self.clone(), other.clone())
    }

    fn add_internal(a: BigInt, b: BigInt) -> BigInt {
        // Match the C bigint_add control flow.
        if a.negative && b.negative {
            // both negative: take normal add, mark negative.
        } else if a.negative {
            let mut a2 = a;
            a2.negative = false;
            return BigInt::sub_internal(b, a2);
        } else if b.negative {
            let mut b2 = b;
            b2.negative = false;
            return BigInt::sub_internal(a, b2);
        }

        let result_negative = a.negative && b.negative;
        let size = a.digits.len().max(b.digits.len());
        let mut digits = vec![0u8; size];

        let mut carry: i64 = 0;
        for i in 0..size {
            let mut sum = carry;
            if i < a.digits.len() {
                sum += a.digits[a.digits.len() - i - 1] as i64;
            }
            if i < b.digits.len() {
                sum += b.digits[b.digits.len() - i - 1] as i64;
            }
            digits[size - i - 1] = (sum % 10) as u8;
            carry = sum / 10;
        }

        if carry > 0 {
            digits.insert(0, carry as u8);
        }

        let mut result = BigInt {
            negative: result_negative,
            digits,
        };
        result.remove_leading_zeros();
        result
    }

    fn sub_internal(a: BigInt, b: BigInt) -> BigInt {
        if a.negative && b.negative {
            let mut a2 = a;
            a2.negative = false;
            let mut b2 = b;
            b2.negative = false;
            let mut result = BigInt::add_internal(b2, a2);
            result.negative = true;
            return result;
        }

        if a.negative {
            // -a - b = -(a + b). C does: b.is_negative=true; add(a,b).
            let mut b2 = b;
            b2.negative = true;
            return BigInt::add_internal(a, b2);
        }

        if b.negative {
            // a - (-b) = a + b
            let mut b2 = b;
            b2.negative = false;
            return BigInt::add_internal(a, b2);
        }

        // Both non-negative.
        let size = a.digits.len().max(b.digits.len());
        let mut digits = vec![0u8; size];

        let is_negative = lt(&a, &b);

        let a_gt_b = gt(&a, &b);
        let (larger, smaller) = if a_gt_b { (&a, &b) } else { (&b, &a) };

        let mut carry: i64 = 0;
        for i in 0..size {
            let mut diff = carry;
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
            digits[size - i - 1] = diff as u8;
        }

        let mut result = BigInt {
            negative: false,
            digits,
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
            return BigInt::from_int(self.to_int().wrapping_mul(other.to_int()));
        }

        let a = self;
        let b = other;
        let size = a.digits.len() + b.digits.len();
        let mut buf = vec![0i64; size];

        for i in 0..a.digits.len() {
            for j in 0..b.digits.len() {
                let idx = size - i - j - 1;
                buf[idx] += (a.digits[a.digits.len() - i - 1] as i64)
                    * (b.digits[b.digits.len() - j - 1] as i64);
            }
        }

        // Normalize from right to left.
        let mut carry: i64 = 0;
        for i in 0..size {
            let idx = size - i - 1;
            let sum = buf[idx] + carry;
            buf[idx] = sum % 10;
            carry = sum / 10;
        }

        let mut digits: Vec<u8> = buf.iter().map(|&v| v as u8).collect();

        if carry > 0 {
            // For a*b, total digits <= a.size + b.size, so carry should be 0 here.
            // Match C behaviour anyway.
            digits.insert(0, carry as u8);
        }

        let mut result = BigInt {
            negative: a.negative != b.negative,
            digits,
        };
        result.remove_leading_zeros();
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

        let mut quotient = BigInt::from_str("0");
        let negative = self.negative != other.negative;

        let mut numerator = self.clone();
        numerator.negative = false;
        let mut denominator = other.clone();
        denominator.negative = false;

        // Slow subtraction-based division to match C semantics exactly.
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
            let mut base = self.to_int();
            let mut exp = exponent.to_int();
            let m = modulo.to_int();
            base %= m;
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

        let mut b = exponent.clone();
        let a = self.r#mod(modulo);
        let mut result = a.clone();
        let mut b_save = b.clone();
        b.dec();

        let mut pow_acc = BigInt::from_str("1");
        let two = BigInt::from_str("2");

        loop {
            let div2 = b.div(&two);
            if !div2.gt_zero() {
                break;
            }

            let sq = result.mul(&result);
            result = sq.r#mod(modulo);

            b = b.div(&two);
            pow_acc = pow_acc.add(&pow_acc);
        }

        b_save = b_save.sub(&pow_acc);

        if b_save.is_odd() {
            let prod = result.mul(&a);
            result = prod.r#mod(modulo);
            b_save.dec();
            pow_acc.inc();
        }

        if lt(&b_save, &BigInt::from_int(5)) {
            while !b_save.is_zero() {
                let prod = result.mul(&a);
                result = prod.r#mod(modulo);
                b_save.dec();
                pow_acc.inc();
            }
            return result;
        }

        let subpart = a.fast_pow(&b_save, modulo);
        let prod = result.mul(&subpart);
        result = prod.r#mod(modulo);
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

            // t = m - q*a
            let t1 = q.mul(&a);
            let t = m.sub(&t1);

            // m = a; a = t
            m = a.clone();
            a = t.clone();

            // t = y - q*x
            let t2 = q.mul(&x);
            let t = y.sub(&t2);

            // y = x; x = t
            y = x.clone();
            x = t.clone();
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
            let mid = low.add(&high).div(&two);
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

        // Sum of digits divisible by 3?
        let mut sum = BigInt::from_str("0");
        for &d in &self.digits {
            let dig = BigInt::from_int(d as i64);
            sum = sum.add(&dig);
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
    let ad = effective_slice(&a.digits);
    let bd = effective_slice(&b.digits);
    if ad.len() > bd.len() {
        return !a.negative;
    }
    if ad.len() < bd.len() {
        return a.negative;
    }
    for i in 0..ad.len() {
        if ad[i] > bd[i] {
            return !a.negative;
        }
        if ad[i] < bd[i] {
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
    fn neg(mut self) -> BigInt {
        self.negative = !self.negative;
        self
    }
}

impl PartialEq for BigInt {
    fn eq(&self, other: &Self) -> bool {
        let a = effective_slice(&self.digits);
        let b = effective_slice(&other.digits);
        if a.len() != b.len() {
            return false;
        }
        if self.negative != other.negative {
            return false;
        }
        a == b
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
