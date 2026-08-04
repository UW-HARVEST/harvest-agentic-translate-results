use std::cmp::Ordering;
use std::fmt;
#[derive(Clone, Debug)]
pub struct BigInt {
    negative: bool,
    digits: Vec<u8>,
}

fn parse_digits(s: &str) -> Vec<u8> {
    s.bytes().map(|b| b.wrapping_sub(b'0')).collect()
}

fn normalized(mut n: BigInt) -> BigInt {
    n.remove_leading_zeros();
    n
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
        let leading_zeros = self.digits.iter().take_while(|&&digit| digit == 0).count();
        if leading_zeros > 0 {
            if leading_zeros >= self.digits.len() {
                self.digits.clear();
                self.digits.push(0);
                return;
            }
            self.digits.drain(..leading_zeros);
        }
    }
    pub fn from_int(n: i64) -> Self {
        let negative = n < 0;
        let digits_str = if negative {
            n.to_string().trim_start_matches('-').to_owned()
        } else {
            n.to_string()
        };
        let mut result = Self {
            negative,
            digits: parse_digits(&digits_str),
        };
        result.remove_leading_zeros();
        result
    }
    pub fn to_int(&self) -> i64 {
        let mut result = 0_i64;
        for &digit in &self.digits {
            result = result * 10 + i64::from(digit);
        }
        if self.negative { -result } else { result }
    }
    pub fn from_str(s: &str) -> Self {
        let (negative, rest) = if let Some(rest) = s.strip_prefix('-') {
            (true, rest)
        } else {
            (false, s)
        };
        Self {
            negative,
            digits: parse_digits(rest),
        }
    }
    pub fn copy(&self) -> BigInt {
        self.clone()
    }
    pub fn print(&self) {
        print!("{self}");
    }
    pub fn is_zero(&self) -> bool {
        let n = normalized(self.clone());
        n.digits.len() == 1 && n.digits[0] == 0
    }
    pub fn lt_zero(&self) -> bool {
        let n = normalized(self.clone());
        !(n.digits.len() == 1 && n.digits[0] == 0) && n.negative
    }
    pub fn gt_zero(&self) -> bool {
        let n = normalized(self.clone());
        !(n.digits.len() == 1 && n.digits[0] == 0) && !n.negative
    }
    pub fn lezero(&self) -> bool {
        let n = normalized(self.clone());
        (n.digits.len() == 1 && n.digits[0] == 0) || n.negative
    }
    pub fn gezero(&self) -> bool {
        let n = normalized(self.clone());
        (n.digits.len() == 1 && n.digits[0] == 0) || !n.negative
    }
    pub fn abs(&self) -> Self {
        let mut n = self.clone();
        n.negative = false;
        n
    }
    pub fn add(&self, other: &Self) -> Self {
        let mut a = self.clone();
        let mut b = other.clone();
        let mut result;

        if a.negative && b.negative {
            result = BigInt {
                negative: true,
                digits: Vec::new(),
            };
        } else if a.negative {
            a.negative = false;
            return b.sub(&a);
        } else if b.negative {
            b.negative = false;
            return a.sub(&b);
        } else {
            result = BigInt {
                negative: false,
                digits: Vec::new(),
            };
        }

        let result_size = a.digits.len().max(b.digits.len());
        result.digits = vec![0; result_size];
        let mut carry = 0_u16;

        for i in 0..result_size {
            let mut sum = carry;
            if i < a.digits.len() {
                sum += u16::from(a.digits[a.digits.len() - i - 1]);
            }
            if i < b.digits.len() {
                sum += u16::from(b.digits[b.digits.len() - i - 1]);
            }
            result.digits[result_size - i - 1] = (sum % 10) as u8;
            carry = sum / 10;
        }

        if carry > 0 {
            result.digits.insert(0, carry as u8);
        }
        result.remove_leading_zeros();
        result
    }
    pub fn sub(&self, other: &Self) -> Self {
        let mut a = self.clone();
        let mut b = other.clone();

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

        let result_size = a.digits.len().max(b.digits.len());
        let mut result = BigInt {
            negative: lt(&a, &b),
            digits: vec![0; result_size],
        };

        let a_gt_b = gt(&a, &b);
        let (larger, smaller) = if a_gt_b { (&a, &b) } else { (&b, &a) };
        let mut carry = 0_i16;

        for i in 0..result_size {
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
            result.digits[result_size - i - 1] = diff as u8;
        }

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
        if self.is_64_bit() && other.is_64_bit() {
            return BigInt::from_int(self.to_int() * other.to_int());
        }

        let mut accum = vec![0_u32; self.digits.len() + other.digits.len()];

        for i in 0..self.digits.len() {
            for j in 0..other.digits.len() {
                let index = accum.len() - i - j - 1;
                accum[index] += u32::from(self.digits[self.digits.len() - i - 1])
                    * u32::from(other.digits[other.digits.len() - j - 1]);
            }
        }

        let mut result = BigInt {
            negative: self.negative != other.negative,
            digits: vec![0; accum.len()],
        };
        let mut carry = 0_u32;
        for i in 0..accum.len() {
            let index = accum.len() - i - 1;
            let sum = accum[index] + carry;
            result.digits[index] = (sum % 10) as u8;
            carry = sum / 10;
        }

        if carry > 0 {
            let mut carry_digits = carry.to_string().bytes().map(|b| b - b'0').collect::<Vec<_>>();
            carry_digits.extend(result.digits);
            result.digits = carry_digits;
        }

        result.remove_leading_zeros();
        result
    }
    pub fn divmod(&self, other: &Self) -> (Self, Self) {
        if self.is_64_bit() && other.is_64_bit() {
            let quotient = self.to_int() / other.to_int();
            let remainder = self.to_int() % other.to_int();
            return (BigInt::from_int(quotient), BigInt::from_int(remainder));
        }

        let mut quotient = BigInt::from_str("0");
        let negative = self.negative != other.negative;
        let mut numerator = self.clone();
        let mut denominator = other.clone();
        numerator.negative = false;
        denominator.negative = false;
        numerator = numerator.copy();

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
            return BigInt::from_str("0");
        }

        let mut result = BigInt::from_str("1");
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
            let mut result = 1_i64;
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
            return BigInt::from_int(result);
        }

        if exponent.negative {
            return BigInt::from_str("0");
        }
        if exponent.is_zero() {
            return BigInt::from_str("1");
        }

        let a = self.r#mod(modulo);
        let mut b = exponent.copy();
        let b_save = b.copy();
        let mut result = a.copy();
        b.dec();

        let mut pow = BigInt::from_str("1");
        loop {
            let half = b.div(&BigInt::from_str("2"));
            if !half.gt_zero() {
                break;
            }

            result = result.mul(&result).r#mod(modulo);
            b = b.div(&BigInt::from_str("2"));
            pow = pow.add(&pow);
        }

        let mut b_remaining = b_save.sub(&pow);

        if b_remaining.is_odd() {
            result = result.mul(&a).r#mod(modulo);
            b_remaining.dec();
            pow.inc();
        }

        if lt(&b_remaining, &BigInt::from_int(5)) {
            while !b_remaining.is_zero() {
                result = result.mul(&a).r#mod(modulo);
                b_remaining.dec();
                pow.inc();
            }
            return result;
        }

        let subpart = a.fast_pow(&b_remaining, modulo);
        result = result.mul(&subpart).r#mod(modulo);
        result
    }
    pub fn modinv(&self, modulo: &Self) -> Self {
        let m0 = modulo.copy();
        let mut y = BigInt::from_str("0");
        let mut x = BigInt::from_str("1");
        let one = BigInt::from_str("1");
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
            BigInt::from_int(0)
        }
    }
    pub fn sqrt(&self) -> Self {
        let one = BigInt::from_str("1");
        let two = BigInt::from_str("2");
        let mut low = BigInt::from_str("0");
        let mut high = self.copy();

        while lt(&low, &high) {
            let mid = low.add(&high).div(&two);
            if lt(&mid.mul(&mid), self) {
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

        if self.digits.len() > 1 {
            let last = self.digits[self.digits.len() - 1];
            if last == 5 || last == 0 {
                return false;
            }
        }

        let mut sum = BigInt::from_str("0");
        for &digit in &self.digits {
            sum = sum.add(&BigInt::from_int(i64::from(digit)));
        }

        if sum.r#mod(&BigInt::from_str("3")).is_zero() {
            return false;
        }

        let sqrt_n = self.sqrt();
        let mut current = BigInt::from_str("2");
        while le(&current, &sqrt_n) {
            if self.r#mod(&current).is_zero() {
                return false;
            }
            current.inc();
        }
        true
    }
}
pub fn gt(a: &BigInt, b: &BigInt) -> bool {
    let a = normalized(a.clone());
    let b = normalized(b.clone());
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
    *a = BigInt::zero();
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
        let a = normalized(self.clone());
        let b = normalized(other.clone());
        a.digits.len() == b.digits.len()
            && a.negative == b.negative
            && a.digits == b.digits
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
        for digit in &self.digits {
            write!(f, "{digit}")?;
        }
        Ok(())
    }
}
