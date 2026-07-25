#![feature(const_trait_impl)]
#![feature(const_convert)]
#![feature(const_iter)]
#![feature(const_ops)]

pub mod bench;
pub mod hash;
pub mod math;
pub mod sivec;

use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fmt::{LowerHex, UpperHex};
use std::hash::{BuildHasher, Hash};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};

/// Provides methods to wrap `self` in [`Option`], [`Result`],
/// and other wrapper types like smart pointers.
/// Intended for use in method chains to improve readability.
/// Instead of your eyes jumping back to the top of the chain
/// to read `Ok(...)`, you can keep the vertical ordering and
/// end the chain with `.wrap_ok()`
///
/// Globally implemented on all [`Sized`] types.
pub trait WrapSelf
where
    Self: Sized,
{
    fn wrap_some(self) -> Option<Self> {
        Some(self)
    }
    fn wrap_ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }
    fn wrap_err<T>(self) -> Result<T, Self> {
        Err(self)
    }
    fn wrap_box(self) -> Box<Self> {
        Box::new(self)
    }
    fn wrap_rc(self) -> Rc<Self> {
        Rc::new(self)
    }
    fn wrap_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
    fn wrap_cell(self) -> Cell<Self> {
        Cell::new(self)
    }
    fn wrap_ref_cell(self) -> RefCell<Self> {
        RefCell::new(self)
    }
    fn wrap_mutex(self) -> Mutex<Self> {
        Mutex::new(self)
    }
    fn wrap_rw_lock(self) -> RwLock<Self> {
        RwLock::new(self)
    }
    fn wrap_reverse(self) -> Reverse<Self> {
        Reverse(self)
    }
    fn wrap_vec(self) -> Vec<Self> {
        vec![self]
    }
    fn wrap_pin(self) -> Pin<Box<Self>> {
        Box::pin(self)
    }
    fn wrap_hash_set(self) -> HashSet<Self>
    where
        Self: Eq + Hash,
    {
        HashSet::from([self])
    }
}
impl<T> WrapSelf for T {}

pub trait IteratorExtra: Iterator {
    /// More convenient version of `collect::<HashMap<_, _>>()`.
    fn collect_hash_map<K, V>(self) -> HashMap<K, V>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Eq + Hash,
    {
        self.collect()
    }

    /// [`IteratorExtra::collect_hash_map`] but allows you to pass in a [`BuildHasher`] to use.
    fn collect_hash_map_with_hasher<K, V, S>(self) -> HashMap<K, V, S>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Eq + Hash,
        S: BuildHasher + Default,
    {
        self.collect()
    }

    /// More convenient version of `collect::<HashSet<_>>()`.
    fn collect_hash_set<T>(self) -> HashSet<T>
    where
        Self: Iterator<Item = T> + Sized,
        T: Eq + Hash,
    {
        self.collect()
    }

    /// [`IteratorExtra::collect_hash_set`] but allows you to pass in a [`BuildHasher`] to use.
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

pub trait FixedHex
where
    Self: LowerHex + UpperHex + Sized,
{
    /// Returns an uppercase hex string of fixed length based on the size of the type.
    fn fixed_hex_lower(&self) -> String {
        format!("{:0size$x}", self, size = size_of::<Self>() * 2)
    }

    /// Returns an uppercase hex string of fixed length based on the size of the type.
    fn fixed_hex_upper(&self) -> String {
        format!("{:0size$X}", self, size = size_of::<Self>() * 2)
    }
}
impl<T: LowerHex + UpperHex + Sized> FixedHex for T {}
