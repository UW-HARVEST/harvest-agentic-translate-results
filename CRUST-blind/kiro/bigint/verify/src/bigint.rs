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
        self.digits.len() < 10
    }
    pub fn remove_leading_zeros(&mut self) {
        let leading = self.digits.iter().take_while(|&&d| d == 0).count();
        if leading >= self.digits.len() {
            self.digits = vec![0];
        } else if leading > 0 {
            self.digits.drain(..leading);
        }
    }
    pub fn from_int(n: i64) -> Self {
        let negative = n < 0;
        let mut v = n.unsigned_abs();
        if v == 0 {
            return BigInt { negative: false, digits: vec![0] };
        }
        let mut digits = Vec::new();
        while v > 0 {
            digits.push((v % 10) as u8);
            v /= 10;
        }
        digits.reverse();
        BigInt { negative, digits }
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
        print!("{}", self);
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
        if self.negative && other.negative {
            let mut r = self.abs().add_unsigned(&other.abs());
            r.negative = true;
            r.remove_leading_zeros();
            return r;
        } else if self.negative {
            return other.sub_impl(&self.abs());
        } else if other.negative {
            return self.sub_impl(&other.abs());
        }
        let mut r = self.add_unsigned(other);
        r.remove_leading_zeros();
        r
    }
    pub fn sub(&self, other: &Self) -> Self {
        if self.negative && other.negative {
            // -a - (-b) = b - a (with both positive)
            let a_pos = self.abs();
            let b_pos = other.abs();
            let mut r = b_pos.add_unsigned(&a_pos);
            r.negative = true;
            r.remove_leading_zeros();
            return r;
        }
        if self.negative {
            // -a - b = -(a + b)
            let mut r = self.abs().add_unsigned(other);
            r.negative = true;
            r.remove_leading_zeros();
            return r;
        }
        if other.negative {
            // a - (-b) = a + b
            return self.add_unsigned(&other.abs());
        }
        self.sub_impl(other)
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
        let sz = self.digits.len() + other.digits.len();
        let mut result = vec![0i64; sz];
        for i in 0..self.digits.len() {
            for j in 0..other.digits.len() {
                let si = self.digits.len() - 1 - i;
                let oj = other.digits.len() - 1 - j;
                result[sz - 1 - i - j] += self.digits[si] as i64 * other.digits[oj] as i64;
            }
        }
        let mut carry: i64 = 0;
        for i in 0..sz {
            let sum = result[sz - 1 - i] + carry;
            result[sz - 1 - i] = sum % 10;
            carry = sum / 10;
        }
        let mut digits: Vec<u8> = result.into_iter().map(|d| d as u8).collect();
        if carry > 0 {
            digits.insert(0, carry as u8);
        }
        let mut r = BigInt { negative: self.negative != other.negative, digits };
        r.remove_leading_zeros();
        r
    }
    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        if self.is_64_bit() && other.is_64_bit() {
            let a = self.to_int();
            let b = other.to_int();
            return (BigInt::from_int(a / b), BigInt::from_int(a % b));
        }
        let negative = self.negative != other.negative;
        let mut num = self.abs();
        let den = other.abs();
        let mut quotient = BigInt::zero();
        while num.gt_zero() {
            num = num.sub_impl(&den);
            if num.lt_zero() {
                break;
            }
            quotient.inc();
        }
        if num.lt_zero() {
            num = num.add_unsigned(&den);
        }
        quotient.remove_leading_zeros();
        if negative {
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
        if exponent.is_zero() {
            return BigInt::from_str("1");
        }
        let mut result = BigInt::from_str("1");
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
            return BigInt::zero();
        }
        if exponent.is_zero() {
            return BigInt::from_str("1");
        }
        let two = BigInt::from_str("2");
        let mut b = exponent.clone();
        let a = self.r#mod(modulo);
        let mut result = a.clone();
        let b_save_orig = b.clone();
        b.dec();

        let mut pow = BigInt::from_str("1");
        loop {
            let half = b.div(&two);
            if !half.gt_zero() {
                break;
            }
            result = result.mul(&result).r#mod(modulo);
            b = b.div(&two);
            pow = pow.add(&pow);
        }

        let mut b_save = b_save_orig.sub_impl(&pow);

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
        let mut x = BigInt::from_str("1");
        let one = BigInt::from_str("1");

        while !a.is_zero() {
            let q = m.div(&a);
            let t_val = m.sub(&q.mul(&a));
            m = a.clone();
            a = t_val.clone();
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
        self.digits.last().map_or(true, |&d| d % 2 == 0)
    }
    pub fn is_odd(&self) -> bool {
        self.digits.last().map_or(false, |&d| d % 2 == 1)
    }
    pub fn is_prime(&self) -> bool {
        if self.is_even() { return false; }
        if self.digits.len() > 1 {
            let last = *self.digits.last().unwrap();
            if last == 5 || last == 0 { return false; }
        }
        let mut sum = BigInt::zero();
        for &d in &self.digits {
            sum = sum.add(&BigInt::from_int(d as i64));
        }
        let three = BigInt::from_str("3");
        if sum.r#mod(&three).is_zero() { return false; }

        let sqrt_n = self.sqrt();
        let mut i = BigInt::from_str("2");
        while le(&i, &sqrt_n) {
            if self.r#mod(&i).is_zero() { return false; }
            i.inc();
        }
        true
    }

    // Private helpers
    fn add_unsigned(&self, other: &Self) -> Self {
        let sz = self.digits.len().max(other.digits.len());
        let mut result = vec![0u8; sz];
        let mut carry: u8 = 0;
        for i in 0..sz {
            let mut sum = carry as u16;
            if i < self.digits.len() {
                sum += self.digits[self.digits.len() - 1 - i] as u16;
            }
            if i < other.digits.len() {
                sum += other.digits[other.digits.len() - 1 - i] as u16;
            }
            result[sz - 1 - i] = (sum % 10) as u8;
            carry = (sum / 10) as u8;
        }
        if carry > 0 {
            result.insert(0, carry);
        }
        let mut r = BigInt { negative: false, digits: result };
        r.remove_leading_zeros();
        r
    }

    // Subtraction of two non-negative values (self - other), result may be negative
    fn sub_impl(&self, other: &Self) -> Self {
        let sz = self.digits.len().max(other.digits.len());
        let mut result_digits = vec![0i8; sz];
        let is_negative = lt_abs(self, other);
        let (larger, smaller) = if gt_abs(self, other) { (self, other) } else { (other, self) };

        let mut carry: i8 = 0;
        for i in 0..sz {
            let mut diff = carry as i16;
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
            result_digits[sz - 1 - i] = diff as i8;
        }
        let digits: Vec<u8> = result_digits.into_iter().map(|d| d as u8).collect();
        let mut r = BigInt { negative: is_negative, digits };
        r.remove_leading_zeros();
        r
    }
}

fn abs_cmp(a: &BigInt, b: &BigInt) -> Ordering {
    let mut ac = a.clone(); ac.remove_leading_zeros();
    let mut bc = b.clone(); bc.remove_leading_zeros();
    match ac.digits.len().cmp(&bc.digits.len()) {
        Ordering::Equal => ac.digits.cmp(&bc.digits),
        o => o,
    }
}

fn gt_abs(a: &BigInt, b: &BigInt) -> bool { abs_cmp(a, b) == Ordering::Greater }
fn lt_abs(a: &BigInt, b: &BigInt) -> bool { abs_cmp(a, b) == Ordering::Less }

pub fn gt(a: &BigInt, b: &BigInt) -> bool {
    let mut ac = a.clone(); ac.remove_leading_zeros();
    let mut bc = b.clone(); bc.remove_leading_zeros();
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
        let mut ac = self.clone(); ac.remove_leading_zeros();
        let mut bc = other.clone(); bc.remove_leading_zeros();
        ac.digits == bc.digits && ac.negative == bc.negative
    }
}
impl Eq for BigInt {}
impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other { return Some(Ordering::Equal); }
        if gt(self, other) { Some(Ordering::Greater) } else { Some(Ordering::Less) }
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
