use std::cmp::Ordering;
use std::fmt;
#[derive(Clone, Debug)]
pub struct BigInt {
    negative: bool,
    digits: Vec<u8>,
}
impl BigInt {
    pub fn zero() -> Self {
        BigInt { negative: false, digits: vec![0] }
    }
    pub fn is_64_bit(&self) -> bool {
        if self.negative {
            self.digits.len() < 10
        } else {
            self.digits.len() < 10
        }
    }
    pub fn remove_leading_zeros(&mut self) {
        let leading = self.digits.iter().take_while(|&&d| d == 0).count();
        if leading > 0 {
            if leading >= self.digits.len() {
                self.digits = vec![0];
            } else {
                self.digits.drain(..leading);
            }
        }
    }
    pub fn from_int(n: i64) -> Self {
        let negative = n < 0;
        let mut val = if n < 0 { (n as i128).unsigned_abs() } else { n as u128 };
        let mut ds = Vec::new();
        if val == 0 {
            ds.push(0);
        } else {
            while val > 0 {
                ds.push((val % 10) as u8);
                val /= 10;
            }
            ds.reverse();
        }
        let mut r = BigInt { negative, digits: ds };
        r.remove_leading_zeros();
        r
    }
    pub fn to_int(&self) -> i64 {
        let mut result: i64 = 0;
        for &d in &self.digits {
            result = result * 10 + d as i64;
        }
        if self.negative { -result } else { result }
    }
    pub fn from_str(s: &str) -> Self {
        let (negative, s) = if s.starts_with('-') { (true, &s[1..]) } else { (false, s) };
        let digits: Vec<u8> = s.bytes().map(|b| b - b'0').collect();
        BigInt { negative, digits }
    }
    pub fn copy(&self) -> BigInt {
        self.clone()
    }
    pub fn print(&self) {
        if self.negative { print!("-"); }
        for &d in &self.digits {
            print!("{}", d);
        }
    }
    pub fn is_zero(&self) -> bool {
        let mut c = self.clone();
        c.remove_leading_zeros();
        c.digits.len() == 1 && c.digits[0] == 0
    }
    pub fn lt_zero(&self) -> bool {
        if self.is_zero() { return false; }
        self.negative
    }
    pub fn gt_zero(&self) -> bool {
        if self.is_zero() { return false; }
        !self.negative
    }
    pub fn lezero(&self) -> bool {
        if self.is_zero() { return true; }
        self.negative
    }
    pub fn gezero(&self) -> bool {
        if self.is_zero() { return true; }
        !self.negative
    }
    pub fn abs(&self) -> Self {
        BigInt { negative: false, digits: self.digits.clone() }
    }
    pub fn add(&self, other: &Self) -> Self {
        let a = self;
        let b = other;
        if a.negative && b.negative {
            let mut r = a.abs().add_unsigned(&b.abs());
            r.negative = true;
            r.remove_leading_zeros();
            return r;
        } else if a.negative {
            return b.sub(&a.abs());
        } else if b.negative {
            return a.sub(&b.abs());
        }
        let mut r = a.add_unsigned(b);
        r.negative = false;
        r.remove_leading_zeros();
        r
    }
    pub fn sub(&self, other: &Self) -> Self {
        let a = self;
        let b = other;
        if a.negative && b.negative {
            // -a - (-b) = b - a (with abs values, then negate add)
            let aa = a.abs();
            let ba = b.abs();
            let mut r = ba.add(&aa);
            r.negative = true;
            r.remove_leading_zeros();
            return r;
        }
        if a.negative {
            // -a - b = -(a + b)
            let ba = BigInt { negative: true, ..b.clone() };
            return a.add(&ba);
        }
        if b.negative {
            // a - (-b) = a + b
            return a.add(&b.abs());
        }
        let is_neg = lt(a, b);
        let (larger, smaller) = if gt(a, b) { (a, b) } else { (b, a) };
        let size = larger.digits.len().max(smaller.digits.len());
        let mut result = vec![0i16; size];
        let mut carry: i16 = 0;
        for i in 0..size {
            let mut diff = carry;
            if i < larger.digits.len() {
                diff += larger.digits[larger.digits.len() - 1 - i] as i16;
            }
            if i < smaller.digits.len() {
                diff -= smaller.digits[smaller.digits.len() - 1 - i] as i16;
            }
            if diff < 0 {
                diff += 10;
                carry = -1;
            } else {
                carry = 0;
            }
            result[size - 1 - i] = diff;
        }
        let digits: Vec<u8> = result.into_iter().map(|d| d as u8).collect();
        let mut r = BigInt { negative: is_neg, digits };
        r.remove_leading_zeros();
        r
    }
    pub fn inc(&mut self) {
        *self = self.add(&BigInt::from_int(1));
    }
    pub fn dec(&mut self) {
        *self = self.sub(&BigInt::from_int(1));
    }
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() * other.to_int());
        }
        let a = &self.digits;
        let b = &other.digits;
        let size = a.len() + b.len();
        let mut result = vec![0i64; size];
        for i in 0..a.len() {
            for j in 0..b.len() {
                result[size - i - j - 1] += (a[a.len() - 1 - i] as i64) * (b[b.len() - 1 - j] as i64);
            }
        }
        let mut carry: i64 = 0;
        for i in 0..size {
            let sum = result[size - 1 - i] + carry;
            result[size - 1 - i] = sum % 10;
            carry = sum / 10;
        }
        let mut digits: Vec<u8> = result.into_iter().map(|d| d as u8).collect();
        if carry > 0 {
            digits.insert(0, carry as u8);
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
            let a = self.to_int();
            let b = other.to_int();
            return (BigInt::from_int(a / b), BigInt::from_int(a % b));
        }
        let neg = self.negative != other.negative;
        let mut num = self.abs();
        let den = other.abs();
        let mut quotient = BigInt::zero();
        while num.gt_zero() {
            num = num.sub(&den);
            if num.lt_zero() { break; }
            quotient.inc();
        }
        if num.lt_zero() {
            num = num.add(&den);
        }
        quotient.remove_leading_zeros();
        if neg {
            quotient.negative = true;
            num.negative = true;
        }
        (quotient, num)
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
            return BigInt::zero();
        }
        let mut result = BigInt::from_int(1);
        if exponent.is_zero() { return result; }
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
        if exponent.negative { return BigInt::zero(); }
        if exponent.is_zero() { return BigInt::from_int(1); }

        let mut b = exponent.clone();
        let a = self.r#mod(modulo);
        let mut result = a.clone();
        let b_save_orig = b.clone();
        b.dec();

        let mut pow = BigInt::from_int(1);
        let two = BigInt::from_int(2);
        loop {
            let half = b.div(&two);
            if !half.gt_zero() { break; }
            result = result.mul(&result).r#mod(modulo);
            b = b.div(&two);
            pow = pow.add(&pow);
        }

        let mut b_save = b_save_orig.sub(&pow);

        if b_save.is_odd() {
            result = result.mul(&a).r#mod(modulo);
            b_save.dec();
            pow.inc();
        }

        let five = BigInt::from_int(5);
        if lt(&b_save, &five) {
            while !b_save.is_zero() {
                result = result.mul(&a).r#mod(modulo);
                b_save.dec();
                pow.inc();
            }
            return result;
        }

        let subpart = a.fast_pow(&b_save, modulo);
        result = result.mul(&subpart).r#mod(modulo);
        result
    }
    pub fn modinv(&self, modulo: &Self) -> Self {
        let mut a = self.clone();
        let mut m = modulo.clone();
        let m0 = modulo.clone();
        let mut y = BigInt::zero();
        let mut x = BigInt::from_int(1);
        let one = BigInt::from_int(1);

        while !a.is_zero() {
            let q = m.div(&a);
            let t = m.sub(&q.mul(&a));
            let (new_m, new_a) = (a.clone(), t.clone());
            m = new_m;
            a = new_a;

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
        let one = BigInt::from_int(1);
        let two = BigInt::from_int(2);
        let mut low = BigInt::zero();
        let mut high = self.clone();
        let mut mid = BigInt::zero();
        while lt(&low, &high) {
            mid = low.add(&high).div(&two);
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
        self.digits.last().unwrap() % 2 == 0
    }
    pub fn is_odd(&self) -> bool {
        self.digits.last().unwrap() % 2 == 1
    }
    pub fn is_prime(&self) -> bool {
        if self.is_even() { return false; }
        let last = *self.digits.last().unwrap();
        if self.digits.len() > 1 && (last == 5 || last == 0) { return false; }
        // Check divisibility by 3 via digit sum
        let sum: i64 = self.digits.iter().map(|&d| d as i64).sum();
        if sum % 3 == 0 { return false; }
        let sqrt_n = self.sqrt();
        let mut i = BigInt::from_int(2);
        while le(&i, &sqrt_n) {
            if self.r#mod(&i).is_zero() { return false; }
            i.inc();
        }
        true
    }

    // Private helper: add two non-negative BigInts (ignores sign)
    fn add_unsigned(&self, other: &Self) -> Self {
        let size = self.digits.len().max(other.digits.len());
        let mut result = vec![0u8; size];
        let mut carry: u8 = 0;
        for i in 0..size {
            let mut sum = carry;
            if i < self.digits.len() {
                sum += self.digits[self.digits.len() - 1 - i];
            }
            if i < other.digits.len() {
                sum += other.digits[other.digits.len() - 1 - i];
            }
            result[size - 1 - i] = sum % 10;
            carry = sum / 10;
        }
        let mut digits = result;
        if carry > 0 {
            digits.insert(0, carry);
        }
        BigInt { negative: false, digits }
    }
}
pub fn gt(a: &BigInt, b: &BigInt) -> bool {
    let mut ac = a.clone();
    let mut bc = b.clone();
    ac.remove_leading_zeros();
    bc.remove_leading_zeros();
    if ac.digits.len() > bc.digits.len() { return !ac.negative; }
    if ac.digits.len() < bc.digits.len() { return ac.negative; }
    for i in 0..ac.digits.len() {
        if ac.digits[i] > bc.digits[i] { return !ac.negative; }
        if ac.digits[i] < bc.digits[i] { return ac.negative; }
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
        BigInt { negative: !self.negative, digits: self.digits }
    }
}
impl PartialEq for BigInt {
    fn eq(&self, other: &Self) -> bool {
        let mut ac = self.clone();
        let mut bc = other.clone();
        ac.remove_leading_zeros();
        bc.remove_leading_zeros();
        ac.digits.len() == bc.digits.len() && ac.negative == bc.negative && ac.digits == bc.digits
    }
}
impl Eq for BigInt {}
impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other { Some(Ordering::Equal) }
        else if gt(self, other) { Some(Ordering::Greater) }
        else { Some(Ordering::Less) }
    }
}
impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.negative { write!(f, "-")?; }
        for &d in &self.digits {
            write!(f, "{}", d)?;
        }
        Ok(())
    }
}
