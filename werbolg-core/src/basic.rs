use alloc::{boxed::Box, string::String};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ident(pub String);

impl From<&str> for Ident {
    fn from(s: &str) -> Self {
        Self(String::from(s))
    }
}

impl From<String> for Ident {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Ident {
    pub fn matches(&self, s: &str) -> bool {
        self.0 == s
    }
}

#[derive(Clone, Debug)]
pub enum Literal {
    String(String),
    Number(Number),
    Decimal(Decimal),
    Bytes(Box<[u8]>),
}

#[cfg(feature = "backend-bignum")]
use num_traits::{Num, ToPrimitive};

#[cfg(feature = "backend-bignum")]
use core::str::FromStr;

#[cfg(feature = "backend-bignum")]
pub type NumberInner = num_bigint::BigInt;

#[cfg(feature = "backend-smallnum")]
pub type NumberInner = u64;

#[derive(Clone, Debug)]
pub struct Number(pub NumberInner);

impl Number {
    pub fn from_u64(v: u64) -> Self {
        Number(NumberInner::from(v))
    }

    pub fn from_str_radix(s: &str, n: u32) -> Result<Self, ()> {
        NumberInner::from_str_radix(s, n)
            .map(|n| Self(n))
            .map_err(|_| ())
    }
}

impl TryFrom<&Number> for u8 {
    type Error = ();

    fn try_from(num: &Number) -> Result<Self, Self::Error> {
        #[cfg(feature = "backend-bignum")]
        {
            num.0.to_u8().ok_or(())
        }
        #[cfg(feature = "backend-smallnum")]
        {
            num.0.try_into().map_err(|_| ())
        }
    }
}

impl TryFrom<&Number> for u16 {
    type Error = ();

    fn try_from(num: &Number) -> Result<Self, Self::Error> {
        #[cfg(feature = "backend-bignum")]
        {
            num.0.to_u16().ok_or(())
        }
        #[cfg(feature = "backend-smallnum")]
        {
            num.0.try_into().map_err(|_| ())
        }
    }
}

impl TryFrom<&Number> for u32 {
    type Error = ();

    fn try_from(num: &Number) -> Result<Self, Self::Error> {
        #[cfg(feature = "backend-bignum")]
        {
            num.0.to_u32().ok_or(())
        }
        #[cfg(feature = "backend-smallnum")]
        {
            num.0.try_into().map_err(|_| ())
        }
    }
}

impl TryFrom<&Number> for u64 {
    type Error = ();

    fn try_from(num: &Number) -> Result<Self, Self::Error> {
        #[cfg(feature = "backend-bignum")]
        {
            num.0.to_u64().ok_or(())
        }
        #[cfg(feature = "backend-smallnum")]
        {
            num.0.try_into().map_err(|_| ())
        }
    }
}

impl TryFrom<&Number> for u128 {
    type Error = ();

    fn try_from(num: &Number) -> Result<Self, Self::Error> {
        #[cfg(feature = "backend-bignum")]
        {
            num.0.to_u128().ok_or(())
        }
        #[cfg(feature = "backend-smallnum")]
        {
            Ok(num.0.into())
        }
    }
}

#[cfg(feature = "backend-bignum")]
pub type DecimalInner = bigdecimal::BigDecimal;

#[cfg(feature = "backend-smallnum")]
pub type DecimalInner = f64;

#[derive(Clone, Debug)]
pub struct Decimal(pub DecimalInner);

impl Decimal {
    pub fn from_str(s: &str) -> Result<Self, ()> {
        #[cfg(feature = "backend-bignum")]
        {
            DecimalInner::from_str(s).map(|n| Self(n)).map_err(|_| ())
        }
        #[cfg(feature = "backend-smallnum")]
        {
            use core::str::FromStr;
            DecimalInner::from_str(s).map(|n| Self(n)).map_err(|_| ())
        }
    }
}
