use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Debug)]
pub struct BigInt {
    negative: bool,
    digits: Vec<u8>,
}

fn divmod_small_positive(n: &BigInt, divisor: u8) -> (BigInt, u8) {
    let mut quotient = Vec::with_capacity(n.digits.len());
    let mut remainder: u16 = 0;

    for &digit in &n.digits {
        let value = remainder * 10 + u16::from(digit);
        quotient.push((value / u16::from(divisor)) as u8);
        remainder = value % u16::from(divisor);
    }

    let mut result = BigInt {
        negative: n.negative,
        digits: quotient,
    };
    result.remove_leading_zeros();
    (result, remainder as u8)
}

impl BigInt {
    pub fn zero() -> Self {
        Self {
            negative: false,
            digits: vec![0],
        }
    }

    pub fn is_64_bit(&self) -> bool {
        self.digits.len() < 10
    }

    pub fn remove_leading_zeros(&mut self) {
        if self.digits.is_empty() {
            self.digits.push(0);
            return;
        }

        let leading_zeros = self.digits.iter().take_while(|&&d| d == 0).count();
        if leading_zeros == 0 {
            return;
        }

        if leading_zeros >= self.digits.len() {
            self.digits.clear();
            self.digits.push(0);
            return;
        }

        self.digits.drain(0..leading_zeros);
    }

    pub fn from_int(n: i64) -> Self {
        let negative = n < 0;
        let mut m = n.unsigned_abs();
        let mut size = 1usize;
        let mut tmp = m;

        while tmp > 0 {
            tmp /= 10;
            size += 1;
        }

        let mut digits = vec![0; size];
        for i in 0..size {
            digits[size - i - 1] = (m % 10) as u8;
            m /= 10;
        }

        let mut result = Self { negative, digits };
        result.remove_leading_zeros();
        result
    }

    pub fn to_int(&self) -> i64 {
        let mut result = 0i64;
        for &digit in &self.digits {
            result *= 10;
            result += i64::from(digit);
        }
        if self.negative { -result } else { result }
    }

    pub fn from_str(s: &str) -> Self {
        let (negative, digits_str) = if let Some(rest) = s.strip_prefix('-') {
            (true, rest)
        } else {
            (false, s)
        };

        let digits = digits_str.bytes().map(|b| b - b'0').collect();
        Self { negative, digits }
    }

    pub fn copy(&self) -> BigInt {
        self.clone()
    }

    pub fn print(&self) {
        print!("{self}");
    }

    pub fn is_zero(&self) -> bool {
        let mut n = self.copy();
        n.remove_leading_zeros();
        n.digits.len() == 1 && n.digits[0] == 0
    }

    pub fn lt_zero(&self) -> bool {
        let mut n = self.copy();
        n.remove_leading_zeros();
        if n.digits.len() == 1 && n.digits[0] == 0 {
            return false;
        }
        n.negative
    }

    pub fn gt_zero(&self) -> bool {
        let mut n = self.copy();
        n.remove_leading_zeros();
        if n.digits.len() == 1 && n.digits[0] == 0 {
            return false;
        }
        !n.negative
    }

    pub fn lezero(&self) -> bool {
        let mut n = self.copy();
        n.remove_leading_zeros();
        if n.digits.len() == 1 && n.digits[0] == 0 {
            return true;
        }
        n.negative
    }

    pub fn gezero(&self) -> bool {
        let mut n = self.copy();
        n.remove_leading_zeros();
        if n.digits.len() == 1 && n.digits[0] == 0 {
            return true;
        }
        !n.negative
    }

    pub fn abs(&self) -> Self {
        let mut result = self.copy();
        result.negative = false;
        result
    }

    pub fn add(&self, other: &Self) -> Self {
        let mut a = self.copy();
        let mut b = other.copy();

        let negative = if a.negative && b.negative {
            true
        } else if a.negative {
            a.negative = false;
            return b.sub(&a);
        } else if b.negative {
            b.negative = false;
            return a.sub(&b);
        } else {
            false
        };

        let size = a.digits.len().max(b.digits.len());
        let mut digits = vec![0u8; size];
        let mut carry = 0u16;

        for i in 0..size {
            let mut sum = carry;
            if i < a.digits.len() {
                sum += u16::from(a.digits[a.digits.len() - i - 1]);
            }
            if i < b.digits.len() {
                sum += u16::from(b.digits[b.digits.len() - i - 1]);
            }
            digits[size - i - 1] = (sum % 10) as u8;
            carry = sum / 10;
        }

        if carry > 0 {
            let mut with_carry = Vec::with_capacity(digits.len() + 1);
            with_carry.push(carry as u8);
            with_carry.extend(digits);
            digits = with_carry;
        }

        let mut result = Self { negative, digits };
        result.remove_leading_zeros();
        result
    }

    pub fn sub(&self, other: &Self) -> Self {
        let mut a = self.copy();
        let mut b = other.copy();

        if a.negative && b.negative {
            a.negative = false;
            b.negative = false;
            let mut result = b.add(&a);
            result.negative = true;
            return result;
        }

        if a.negative {
            b.negative = true;
            return a.add(&b);
        }

        if b.negative {
            b.negative = false;
            return a.add(&b);
        }

        let size = a.digits.len().max(b.digits.len());
        let mut digits = vec![0u8; size];
        let is_negative = lt(&a, &b);

        let (larger, smaller) = if gt(&a, &b) { (a, b) } else { (b, a) };
        let mut carry = 0i16;

        for i in 0..size {
            let mut diff = carry;
            if i < larger.digits.len() {
                diff += i16::from(larger.digits[larger.digits.len() - i - 1]);
            }
            if i < smaller.digits.len() {
                diff -= i16::from(smaller.digits[smaller.digits.len() - i - 1]);
            }
            if diff < 0 {
                diff += 10;
                carry = -1;
            } else {
                carry = 0;
            }
            digits[size - i - 1] = diff as u8;
        }

        let mut result = Self {
            negative: is_negative,
            digits,
        };
        result.remove_leading_zeros();
        result
    }

    pub fn inc(&mut self) {
        *self = self.add(&Self::from_str("1"));
    }

    pub fn dec(&mut self) {
        *self = self.sub(&Self::from_str("1"));
    }

    pub fn mul(&self, other: &Self) -> Self {
        if self.is_64_bit() && other.is_64_bit() {
            return Self::from_int(self.to_int() * other.to_int());
        }

        let mut digits = vec![0u16; self.digits.len() + other.digits.len()];

        for i in 0..self.digits.len() {
            for j in 0..other.digits.len() {
                let idx = digits.len() - i - j - 1;
                digits[idx] +=
                    u16::from(self.digits[self.digits.len() - i - 1])
                    * u16::from(other.digits[other.digits.len() - j - 1]);
            }
        }

        let mut carry = 0u16;
        for i in 0..digits.len() {
            let idx = digits.len() - i - 1;
            let sum = digits[idx] + carry;
            digits[idx] = sum % 10;
            carry = sum / 10;
        }

        let mut out: Vec<u8> = digits.into_iter().map(|d| d as u8).collect();
        if carry > 0 {
            let mut with_carry = Vec::with_capacity(out.len() + 1);
            with_carry.push(carry as u8);
            with_carry.extend(out);
            out = with_carry;
        }

        let mut result = Self {
            negative: self.negative != other.negative,
            digits: out,
        };
        result.remove_leading_zeros();
        result
    }

    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        if self.is_64_bit() && other.is_64_bit() {
            let quotient = self.to_int() / other.to_int();
            let remainder = self.to_int() % other.to_int();
            return (Self::from_int(quotient), Self::from_int(remainder));
        }

        let mut quotient = Self::from_str("0");
        let negative = self.negative != other.negative;
        let mut numerator = self.abs().copy();
        let denominator = other.abs();

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
        self.divmod(other).0
    }

    pub fn r#mod(&self, other: &Self) -> Self {
        self.divmod(other).1
    }

    pub fn pow(&self, exponent: &Self) -> Self {
        if exponent.negative {
            return Self::from_str("0");
        }

        let mut result = Self::from_str("1");
        if exponent.is_zero() {
            return result;
        }

        let mut exp = exponent.copy();
        while !exp.is_zero() {
            result = result.mul(self);
            exp.dec();
        }
        result
    }

    pub fn fast_pow(&self, exponent: &Self, modulo: &Self) -> Self {
        if self.is_64_bit() && exponent.is_64_bit() && modulo.is_64_bit() {
            let mut result = 1i64;
            let mut base = self.to_int();
            let mut exp = exponent.to_int();
            let modu = modulo.to_int();

            base %= modu;
            while exp > 0 {
                if exp % 2 == 1 {
                    result = (result * base) % modu;
                }
                exp >>= 1;
                base = (base * base) % modu;
            }
            return Self::from_int(result);
        }

        if exponent.negative {
            return Self::from_str("0");
        }
        if exponent.is_zero() {
            return Self::from_str("1");
        }

        let mut result = Self::from_str("1");
        let mut base = self.r#mod(modulo);
        let mut exp = exponent.copy();

        while exp.gt_zero() {
            if exp.is_odd() {
                result = result.mul(&base).r#mod(modulo);
            }
            let (next_exp, _) = divmod_small_positive(&exp.abs(), 2);
            exp = next_exp;
            if exp.gt_zero() {
                base = base.mul(&base).r#mod(modulo);
            }
        }

        result
    }

    pub fn modinv(&self, modulo: &Self) -> Self {
        let m0 = modulo.copy();
        let mut y = Self::from_str("0");
        let mut x = Self::from_str("1");
        let one = Self::from_str("1");
        let mut a = self.copy();
        let mut m = modulo.copy();

        while !a.is_zero() {
            let q = m.div(&a);
            let mut t = m.sub(&q.mul(&a));
            m = a.copy();
            a = t.copy();

            t = y.sub(&q.mul(&x));
            y = x.copy();
            x = t.copy();
        }

        if m.lt_zero() {
            m = m.add(&m0);
        }

        if m == one {
            let mut result = y.r#mod(&m0);
            if result.lt_zero() {
                result = result.add(&m0);
            }
            result
        } else {
            Self::from_int(0)
        }
    }

    pub fn sqrt(&self) -> Self {
        let one = Self::from_str("1");
        let two = Self::from_str("2");
        let mut low = Self::from_str("0");
        let mut high = self.copy();
        while lt(&low, &high) {
            let mid = low.add(&high).div(&two);
            if mid.mul(&mid) < *self {
                low = mid.add(&one);
            } else {
                high = mid.copy();
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

        let last = self.digits[self.digits.len() - 1];
        if self.digits.len() > 1 && (last == 5 || last == 0) {
            return false;
        }

        let mut sum = Self::from_str("0");
        for &digit in &self.digits {
            sum = sum.add(&Self::from_int(i64::from(digit)));
        }

        if sum.r#mod(&Self::from_str("3")).is_zero() {
            return false;
        }

        let sqrt_n = self.sqrt();
        let mut i = Self::from_str("2");
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
    let mut a = a.copy();
    let mut b = b.copy();
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

    fn neg(mut self) -> BigInt {
        self.negative = !self.negative;
        self
    }
}

impl PartialEq for BigInt {
    fn eq(&self, other: &Self) -> bool {
        let mut a = self.copy();
        let mut b = other.copy();
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
        } else if lt(self, other) {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.negative {
            write!(f, "-")?;
        }
        for &digit in &self.digits {
            write!(f, "{digit}")?;
        }
        Ok(())
    }
}
