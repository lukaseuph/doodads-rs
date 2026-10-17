#![feature(const_trait_impl, const_convert, const_iter, const_ops)]

pub mod bench;
pub mod hash;
pub mod invariants;
pub mod math;
pub mod sivec;

use std::{
    collections::{HashMap, HashSet},
    fmt::{LowerHex, UpperHex},
    hash::{BuildHasher, Hash},
};

/// Provides a [`pipe`] method for chaining functions that map a value to another value.
///
/// # Examples
///
/// ```
/// use doodads::Pipe;
///
/// let result = 42.pipe(|x| x * 2);
/// assert_eq!(result, 84);
/// ```
///
/// ```
/// use doodads::Pipe;
///
/// let option: Option<_> = "some input".pipe(Some);
/// assert_eq!(option, Some("some input"));
/// ```
pub trait Pipe
where
    Self: Sized,
{
    /// Pipe the value through a function that only takes that value as an argument.
    fn pipe<T, F: Fn(Self) -> T>(self, f: F) -> T {
        f(self)
    }
}
impl<T: Sized> Pipe for T {}

pub trait OptionExtensions {
    fn overlay(&mut self, other: Self);
    fn overlaid(self, other: Self) -> Self;
}
impl<T> OptionExtensions for Option<T> {
    /// Overwrites the value of `self` in place if `other` is [`Some`].
    ///
    /// # Examples
    /// ```
    /// use doodads::OptionExtensions;
    ///
    /// let mut opt = Some(5);
    /// opt.overlay(Some(10));
    /// opt.overlay(None);
    /// assert_eq!(opt, Some(10));
    /// ```
    fn overlay(&mut self, other: Self) {
        if other.is_some() {
            *self = other;
        }
    }

    /// Returns `self` if `other` is [`None`], otherwise returns `other`.
    ///
    /// # Examples
    /// ```
    /// use doodads::OptionExtensions;
    /// let opt = Some(5).overlaid(Some(10)).overlaid(None);
    /// assert_eq!(opt, Some(10));
    fn overlaid(self, other: Self) -> Self {
        if other.is_some() { other } else { self }
    }
}

/// Extra functionality expanding on iterators.
/// Bridges some gaps in `itertools` such as collecting
/// [`HashMap`](std::collections::HashMap)s
pub trait IteratorExtra: Iterator {
    /// More convenient version of
    /// [`collect`](`std::iter::Iterator::collect`)`::<`[`HashMap`](std::collections::HashMap)`<_, _>>`.
    fn collect_hash_map<K, V>(self) -> HashMap<K, V>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Eq + Hash,
    {
        self.collect()
    }

    /// [`IteratorExtra::collect_hash_map`] but allows you to pass in a
    /// [`BuildHasher`] to use.
    fn collect_hash_map_with_hasher<K, V, S>(self) -> HashMap<K, V, S>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Eq + Hash,
        S: BuildHasher + Default,
    {
        self.collect()
    }

    /// More convenient version of
    /// [`collect`](`std::iter::Iterator::collect`)`::<`[`HashSet`](std::collections::HashSet)`<_>>`.
    fn collect_hash_set<T>(self) -> HashSet<T>
    where
        Self: Iterator<Item = T> + Sized,
        T: Eq + Hash,
    {
        self.collect()
    }

    /// [`IteratorExtra::collect_hash_set`] but allows you to pass in a
    /// [`BuildHasher`] to use.
    fn collect_hash_set_with_hasher<T, S>(self) -> HashSet<T, S>
    where
        Self: Iterator<Item = T> + Sized,
        T: Eq + Hash,
        S: BuildHasher + Default,
    {
        self.collect()
    }
}
impl<T> IteratorExtra for T where T: Iterator + ?Sized {}

/// Formats hex strings of fixed length based on the size of the type.
/// # Examples
/// ```rust
/// use doodads::FixedHex;
/// let x: u32 = 1;
/// assert_eq!(x.fixed_hex_lower(), "00000001");
/// assert_eq!(x.fixed_hex_upper(), "00000001");
/// ```
pub trait FixedHex {
    /// Returns an uppercase hex string of fixed length based on the size of
    /// the type.
    fn fixed_hex_lower(&self) -> String;

    /// Returns an uppercase hex string of fixed length based on the size of
    /// the type.
    fn fixed_hex_upper(&self) -> String;
}
impl<T: LowerHex + UpperHex + Sized> FixedHex for T {
    /// Returns an uppercase hex string of fixed length based on the size of
    /// the type.
    fn fixed_hex_lower(&self) -> String {
        format!("{:0size$x}", self, size = size_of::<Self>() * 2)
    }

    /// Returns an uppercase hex string of fixed length based on the size of
    /// the type.
    fn fixed_hex_upper(&self) -> String {
        format!("{:0size$X}", self, size = size_of::<Self>() * 2)
    }
}

pub type Array2d<T, const W: usize, const H: usize = W> = [[T; W]; H];
pub type Array3d<T, const W: usize, const H: usize = W, const D: usize = W> = [Array2d<T, W, H>; D];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe() {
        assert_eq!(15.pipe(Some), Some(15))
    }

    #[test]
    fn fixed_hex() {
        let x: u8 = 1;
        assert_eq!(x.fixed_hex_lower(), "01");
        assert_eq!(x.fixed_hex_upper(), "01");

        let x: u16 = 1;
        assert_eq!(x.fixed_hex_lower(), "0001");
        assert_eq!(x.fixed_hex_upper(), "0001");

        let x: u32 = 1;
        assert_eq!(x.fixed_hex_lower(), "00000001");
        assert_eq!(x.fixed_hex_upper(), "00000001");

        let x: u64 = 1;
        assert_eq!(x.fixed_hex_lower(), "0000000000000001");
        assert_eq!(x.fixed_hex_upper(), "0000000000000001");

        let x: u128 = 1;
        assert_eq!(x.fixed_hex_lower(), "00000000000000000000000000000001");
        assert_eq!(x.fixed_hex_upper(), "00000000000000000000000000000001");
    }

    #[test]
    fn option_overlay() {
        let mut opt = Some(5);
        opt.overlay(Some(10));
        opt.overlay(None);
        assert_eq!(opt, Some(10));
    }

    #[test]
    fn option_overlaid() {
        let opt = Some(5).overlaid(Some(10)).overlaid(None);
        assert_eq!(opt, Some(10));
    }

    #[test]
    fn array2d() {
        // rectangular
        let arr = Array2d::<i32, 3, 4>::default();
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0].len(), 3);
        // square
        let arr = Array2d::<i32, 5>::default();
        assert_eq!(arr.len(), 5);
        assert_eq!(arr[0].len(), 5);
    }

    #[test]
    fn array3d() {
        // rectangular
        let arr = Array3d::<i32, 3, 4, 5>::default();
        assert_eq!(arr.len(), 5);
        assert_eq!(arr[0].len(), 4);
        assert_eq!(arr[0][0].len(), 3);
        // cubic
        let arr = Array3d::<i32, 6>::default();
        assert_eq!(arr.len(), 6);
        assert_eq!(arr[0].len(), 6);
        assert_eq!(arr[0][0].len(), 6);
    }
}
