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
        // Mirrors the C function: returns true iff digit count < 10.
        self.digits.len() < 10
    }
    pub fn remove_leading_zeros(&mut self) {
        let mut leading_zeros = 0usize;
        for d in &self.digits {
            if *d == 0 {
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
            self.digits.drain(0..leading_zeros);
        }
    }
    pub fn from_int(n: i64) -> Self {
        let (negative, mut m) = if n < 0 {
            // Use wrapping_neg to handle i64::MIN gracefully, though unlikely here.
            (true, n.unsigned_abs())
        } else {
            (false, n as u64)
        };
        let mut digits: Vec<u8> = Vec::new();
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
        for d in &self.digits {
            result = result.wrapping_mul(10).wrapping_add(*d as i64);
        }
        if self.negative {
            result = -result;
        }
        result
    }
    pub fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut idx = 0;
        let negative = if !bytes.is_empty() && bytes[0] == b'-' {
            idx = 1;
            true
        } else {
            false
        };
        let digits: Vec<u8> = bytes[idx..].iter().map(|c| c - b'0').collect();
        BigInt { negative, digits }
    }
    pub fn copy(&self) -> BigInt {
        self.clone()
    }
    pub fn print(&self) {
        if self.negative {
            print!("-");
        }
        for d in &self.digits {
            assert!(*d <= 9);
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
        let mut r = self.clone();
        r.negative = false;
        r
    }
    pub fn add(&self, other: &Self) -> Self {
        let a = self;
        let b = other;
        // Mixed-sign cases delegate to sub.
        if a.negative && b.negative {
            // Both negative: -|a| + -|b| = -(|a| + |b|)
            let pa = a.abs();
            let pb = b.abs();
            let mut r = pa.add(&pb);
            r.negative = true;
            r.remove_leading_zeros();
            return r;
        } else if a.negative {
            // -|a| + b = b - |a|
            let pa = a.abs();
            return b.sub(&pa);
        } else if b.negative {
            // a + -|b| = a - |b|
            let pb = b.abs();
            return a.sub(&pb);
        }

        // Both non-negative.
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
            negative: false,
            digits,
        };
        result.remove_leading_zeros();
        result
    }
    pub fn sub(&self, other: &Self) -> Self {
        let a = self;
        let b = other;
        if a.negative && b.negative {
            // -|a| - -|b| = |b| - |a|
            let pa = a.abs();
            let pb = b.abs();
            let mut r = pb.add(&pa);
            // Replicate C: result = bigint_add(b_pos, a_pos); result.is_negative = true
            // This makes -|a| - -|b| = -(|a| + |b|), but it actually should be |b| - |a|.
            // Actually the C code says:
            //   a.is_negative = false; b.is_negative = false;
            //   result = bigint_add(b, a);  // = |b| + |a|
            //   result.is_negative = true;  // = -(|a|+|b|)
            // That's a bug in the C, but we mirror behavior to match tests.
            // Wait, let me re-check: -a - (-b) = -a + b = b - a. So expected = b - a.
            // C gives -(a+b). Hmm. But we must match C exactly per instructions.
            // Actually re-reading: tests don't seem to hit this path (both negative), so let's
            // just match the C behavior literally.
            r.negative = true;
            r.remove_leading_zeros();
            return r;
        }
        if a.negative {
            // -a - b = -(a+b). C does: b.is_negative = true; result = bigint_add(a, b);
            // i.e., result = a + (-b) where a is also negative. That equals -(|a|+|b|).
            let neg_b = BigInt {
                negative: true,
                digits: b.digits.clone(),
            };
            return a.add(&neg_b);
        }
        if b.negative {
            // a - (-b) = a + b
            let pos_b = b.abs();
            return a.add(&pos_b);
        }

        // Both non-negative.
        let size = a.digits.len().max(b.digits.len());
        let mut digits = vec![0u8; size];
        let is_negative = lt(a, b);
        let (larger, smaller) = if gt(a, b) { (a, b) } else { (b, a) };
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
            negative: is_negative,
            digits,
        };
        result.remove_leading_zeros();
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
        let a = self;
        let b = other;
        if a.is_64_bit() && b.is_64_bit() {
            return BigInt::from_int(a.to_int().wrapping_mul(b.to_int()));
        }

        let size = a.digits.len() + b.digits.len();
        // Use i64 buffer to avoid u8 overflow during accumulation.
        let mut buf: Vec<i64> = vec![0; size];
        for i in 0..a.digits.len() {
            for j in 0..b.digits.len() {
                let pos = size - i - j - 1;
                buf[pos] +=
                    (a.digits[a.digits.len() - i - 1] as i64) * (b.digits[b.digits.len() - j - 1] as i64);
            }
        }
        // Normalize.
        let mut carry: i64 = 0;
        for i in 0..size {
            let pos = size - i - 1;
            let sum = buf[pos] + carry;
            buf[pos] = sum % 10;
            carry = sum / 10;
        }
        let mut digits: Vec<u8> = buf.iter().map(|x| *x as u8).collect();
        if carry > 0 {
            // Push leading carry digits.
            let mut carry_digits: Vec<u8> = Vec::new();
            let mut c = carry;
            while c > 0 {
                carry_digits.push((c % 10) as u8);
                c /= 10;
            }
            carry_digits.reverse();
            let mut new_digits = carry_digits;
            new_digits.extend(digits.iter());
            digits = new_digits;
        }
        let mut result = BigInt {
            negative: a.negative != b.negative,
            digits,
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
            let q = n / d;
            let r = n % d;
            return (BigInt::from_int(q), BigInt::from_int(r));
        }

        let mut quotient = BigInt::from_str("0");
        let negative = numerator.negative != denominator.negative;
        let mut num = numerator.abs();
        let mut denom = denominator.clone();
        denom.negative = false;

        // Slow repeated subtraction.
        while num.gt_zero() {
            let sub_result = num.sub(&denom);
            num = sub_result;
            if num.lt_zero() {
                break;
            }
            quotient.inc();
        }
        if num.lt_zero() {
            num = num.add(&denom);
        }

        let mut remainder = num;
        quotient.remove_leading_zeros();

        if negative {
            quotient.negative = true;
            remainder.negative = true;
        }
        (quotient, remainder)
    }
    pub fn div(&self, other: &Self) -> Self {
        let a = self;
        let b = other;
        if a.is_64_bit() && b.is_64_bit() {
            return BigInt::from_int(a.to_int() / b.to_int());
        }
        let (q, _r) = a.divmod(b);
        q
    }
    pub fn r#mod(&self, other: &Self) -> Self {
        let a = self;
        let b = other;
        if a.is_64_bit() && b.is_64_bit() {
            return BigInt::from_int(a.to_int() % b.to_int());
        }
        let (_q, r) = a.divmod(b);
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
        let a = self;
        let b = exponent;
        let m = modulo;
        if a.is_64_bit() && b.is_64_bit() && m.is_64_bit() {
            let mut result: i64 = 1;
            let mut base: i64 = a.to_int();
            let mut exp: i64 = b.to_int();
            let mod_v: i64 = m.to_int();
            base %= mod_v;
            while exp > 0 {
                if exp % 2 == 1 {
                    result = (result * base) % mod_v;
                }
                exp >>= 1;
                base = (base * base) % mod_v;
            }
            return BigInt::from_int(result);
        }

        if b.negative {
            return BigInt::from_str("0");
        }
        if b.is_zero() {
            return BigInt::from_str("1");
        }

        let mut b_local = b.clone();
        let a_mod = a.r#mod(m);
        let mut result = a_mod.clone();
        let mut b_save = b.clone();
        b_local.dec();

        let mut pow = BigInt::from_str("1");
        loop {
            let two = BigInt::from_str("2");
            let halved = b_local.div(&two);
            if !halved.gt_zero() {
                break;
            }

            let sq = result.mul(&result);
            result = sq.r#mod(m);

            b_local = b_local.div(&two);
            pow = pow.add(&pow);
        }

        b_save = b_save.sub(&pow);

        if b_save.is_odd() {
            let prod = result.mul(&a_mod);
            result = prod.r#mod(m);
            b_save.dec();
            pow.inc();
        }

        let five = BigInt::from_int(5);
        if lt(&b_save, &five) {
            while !b_save.is_zero() {
                let prod = result.mul(&a_mod);
                result = prod.r#mod(m);
                b_save.dec();
                pow.inc();
            }
            return result;
        }

        let subpart = a_mod.fast_pow(&b_save, m);
        let prod = result.mul(&subpart);
        result = prod.r#mod(m);
        result
    }
    pub fn modinv(&self, modulo: &Self) -> Self {
        let a_orig = self;
        let m_orig = modulo;
        let m0 = m_orig.clone();
        let mut y = BigInt::from_str("0");
        let mut x = BigInt::from_str("1");
        let one = BigInt::from_str("1");
        let mut a = a_orig.clone();
        let mut m = m_orig.clone();

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
        let n = self;
        let one = BigInt::from_str("1");
        let two = BigInt::from_str("2");
        let mut low = BigInt::from_str("0");
        let mut high = n.clone();
        while lt(&low, &high) {
            let sum = low.add(&high);
            let mid = sum.div(&two);
            let sq = mid.mul(&mid);
            if lt(&sq, n) {
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
        // Sum of digits divisible by 3?
        let mut sum = BigInt::from_str("0");
        for d in &n.digits {
            let dig = BigInt::from_int(*d as i64);
            sum = sum.add(&dig);
        }
        let three = BigInt::from_str("3");
        let modres = sum.r#mod(&three);
        if modres.is_zero() {
            return false;
        }

        let sqrt_n = n.sqrt();
        let mut i = BigInt::from_str("2");
        while le(&i, &sqrt_n) {
            let m = n.r#mod(&i);
            if m.is_zero() {
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
    if a.negative != b.negative {
        // Same length; same sign matters now.
        // Actually C didn't check this, but if signs differ and lengths equal,
        // the digit comparison will be biased by sign. Let's mirror exactly.
        // Re-reading C: it just iterates digits and returns based on first
        // differing digit. So we mirror that.
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
        for d in &self.digits {
            write!(f, "{}", d)?;
        }
        Ok(())
    }
}
