use paste::paste;
use std::fmt;
use std::fmt::Formatter;
use std::num::Wrapping;
use std::ops::{AddAssign, MulAssign};
use std::slice::Iter;
use is_prime::is_prime;
use itertools::Itertools;
use num_traits::ToBytes;

/// Default implementation of the AmpHasher
pub const AMP: AmpHasher = unsafe {
    AmpHasher::new_unchecked(
        131,
        50671,
        3647798773,
        18123145269993195119,
        230663825421896382524516261427012097267,
        337891335454201426143216793176103336809,
    )
};

/// AMP (Add, Multiply Prime) is a lightweight, fast, flexible, non-secure hashing algorithm.
/// AMP's primary use case is in situations where security isn't necessary, and quick hashes are
/// required. It works especially well in situations like compiler lookup tables, where hashes
/// can be small and are used to detect where changes have been made.
///
/// # Creating your own instance of AmpHasher:
/// An AMP instance contains two values for each hashing target: a prime number, and a seed.
///
/// # Examples
/// ## Creating your own AmpHasher instance
/// ```rust
/// let my_amp = AmpHasher::new(
///     131,                                    // prime u8
///     50671,                                  // prime u16
///     3647798773,                             // prime u32
///     18123145269993195119,                   // prime u64
///     230663825421896382524516261427012097267,// prime u128
///     337891335454201426143216793176103336809,// seed as u128
/// )
/// ```
#[derive(Debug, Copy, Clone)]
pub struct AmpHasher {
    prime_u8: u8,
    prime_u16: u16,
    prime_u32: u32,
    prime_u64: u64,
    prime_u128: u128,
    seed_u8: u8,
    seed_u16: u16,
    seed_u32: u32,
    seed_u64: u64,
    seed_u128: u128,
}

impl AmpHasher {
    pub fn new(prime_u8: u8, prime_u16: u16, prime_u32: u32, prime_u64: u64, prime_u128: u128, seed: u128) -> Result<Self, AmpHasherError> {
        let (seed_u8, seed_u16, seed_u32, seed_u64, seed_u128) = Self::get_seeds(seed);
        Ok(Self {
            prime_u8: if is_prime(&format!("{prime_u8}")) {
                prime_u8
            } else {
                return Err(AmpHasherError::U8(prime_u8));
            },
            prime_u16: if is_prime(&format!("{prime_u16}")) {
                prime_u16
            } else {
                return Err(AmpHasherError::U16(prime_u16));
            },
            prime_u32: if is_prime(&format!("{prime_u32}")) {
                prime_u32
            } else {
                return Err(AmpHasherError::U32(prime_u32));
            },
            prime_u64: if is_prime(&format!("{prime_u64}")) {
                prime_u64
            } else {
                return Err(AmpHasherError::U64(prime_u64));
            },
            prime_u128: if is_prime(&format!("{prime_u128}")) {
                prime_u128
            } else {
                return Err(AmpHasherError::U128(prime_u128));
            },
            seed_u8,
            seed_u16,
            seed_u32,
            seed_u64,
            seed_u128,
        })
    }

    /// Creates a new `AmpHasher` instance without verifying that all provided values are prime.
    /// # Safety
    /// In order to avoid faulty behavior, you must ensure that each parameter of this function call is prime.
    ///
    /// Note that if any of the values provided are `0`, all calls to [AmpHasher::digest] for that integer type will return `0`.
    pub const unsafe fn new_unchecked(prime_u8: u8, prime_u16: u16, prime_u32: u32, prime_u64: u64, prime_u128: u128, seed: u128) -> Self {
        let (seed_u8, seed_u16, seed_u32, seed_u64, seed_u128) = Self::get_seeds(seed);
        Self { prime_u8, prime_u16, prime_u32, prime_u64, prime_u128, seed_u8, seed_u16, seed_u32, seed_u64, seed_u128 }
    }

    const fn get_seeds(seed: u128) -> (u8, u16, u32, u64, u128) {
        let bytes = seed.to_be_bytes();
        let seed_u8 = bytes[0];
        let seed_u16 = u16::from_be_bytes([bytes[0], bytes[1]]);
        let seed_u32 = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let seed_u64 = u64::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
        (seed_u8, seed_u16, seed_u32, seed_u64, seed)
    }

    /// Digests data and returns a number T
    pub fn digest<T>(&self, data: impl AsRef<[u8]>) -> T
    where
        T: From<u8> + Copy,
        Wrapping<T>: AddAssign<T> + MulAssign<T>,
        Self: AmpDigest<T>,
    {
        let mut sum: Wrapping<T> = Wrapping(self.seed());
        data.as_ref().iter().for_each(|&b| {
            sum += b.into();
            sum *= self.prime();
        });
        sum.0
    }

    pub const fn const_digest<T>(&self, data: &[u8]) -> T
    where
        T: ~const From<u8> + Copy,
        Wrapping<T>: ~const AddAssign<T> + ~const MulAssign<T>,
        Self: ~const AmpDigest<T>,
        &'static [u8]: ~const IntoIterator,
        Iter<'static, u8>: ~const Iterator,
    {
        let mut sum: Wrapping<T> = Wrapping(self.seed());
        for &b in data {
            sum += b.into();
            sum *= self.prime();
        }
        sum.0
    }

    /// Digests an unordered list using the AMP algorithm.
    pub fn digest_unordered<T>(&self, data: &[impl AsRef<[u8]>]) -> T where
        T: From<u8> + Copy + Ord + ToBytes + fmt::Debug,
        Wrapping<T>: AddAssign<T> + MulAssign<T>,
        Self: AmpDigest<T>,
    {
        let mut list = data.iter().map(|bytes| self.digest(bytes.as_ref())).collect_vec();
        list.sort();
        dbg!(&list);
        let flat = list.iter()
            .flat_map(|hash| hash.to_be_bytes().as_ref().to_vec())
            .collect_vec();
        self.digest(&flat)
    }
}
impl Default for AmpHasher {
    fn default() -> Self { AMP }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AmpHasherError {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
}
impl fmt::Display for AmpHasherError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AmpHasherError::U8(num) => write!(f, "'{}' (u8) is not prime", num),
            AmpHasherError::U16(num) => write!(f, "'{}' (u16) is not prime", num),
            AmpHasherError::U32(num) => write!(f, "'{}' (u32) is not prime", num),
            AmpHasherError::U64(num) => write!(f, "'{}' (u64) is not prime", num),
            AmpHasherError::U128(num) => write!(f, "'{}' (u128) is not prime", num),
        }
    }
}

pub const trait AmpDigest<T> {
    fn prime(&self) -> T;
    fn seed(&self) -> T;
}

macro_rules! impl_amp_digest {
    ( $( $type:ty ),* ) => {
        $(
            paste!(
                impl AmpDigest<$type> for AmpHasher {
                    fn prime(&self) -> $type {
                        self.[<prime_ $type>]
                    }
                    fn seed(&self) -> $type {
                        self.[<seed_ $type>]
                    }
                }
            );
        )*
    }
}
impl_amp_digest!(u8, u16, u32, u64, u128);