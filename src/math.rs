//! A generic, saturating clamped numeric type.

use num_traits::{Bounded, Float, Num, Signed, ToPrimitive};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{
    Add, AddAssign, Deref, Div, DivAssign, Mul, MulAssign, Neg, RangeFrom, RangeInclusive, RangeTo,
    Rem, RemAssign, Sub, SubAssign,
};

/// Numeric types [`Clamp`] can wrap.
///
/// This exists (rather than bounding directly on [`num_traits::Num`]) because
/// integer arithmetic in Rust panics/wraps on overflow *before* a plain
/// post-hoc `min(max(...))` clamp ever gets a chance to run -- e.g.
/// `100u32 - 1000` blows up at the hardware level regardless of what you do
/// with the result. `ClampNum` requires arithmetic that saturates to the
/// type's own representable range first, so the subsequent clamp to your
/// chosen `[min, max]` is always operating on a valid value.
///
/// Implemented out of the box for every standard numeric primitive:
/// `i8`..`i128`, `isize`, `u8`..`u128`, `usize`, `f32`, `f64`. Implement it
/// yourself for other numeric types (e.g. from `num-bigint`) if needed.
pub trait ClampNum: Num + PartialOrd + Copy {
    fn raw_add(self, rhs: Self) -> Self;
    fn raw_sub(self, rhs: Self) -> Self;
    fn raw_mul(self, rhs: Self) -> Self;
    fn raw_div(self, rhs: Self) -> Self;
    fn raw_rem(self, rhs: Self) -> Self;
}
macro_rules! impl_clamp_num_int {
    ($($t:ty),*) => {
        $(
            impl ClampNum for $t {
                #[inline]
                fn raw_add(self, rhs: Self) -> Self { self.saturating_add(rhs) }
                #[inline]
                fn raw_sub(self, rhs: Self) -> Self { self.saturating_sub(rhs) }
                #[inline]
                fn raw_mul(self, rhs: Self) -> Self { self.saturating_mul(rhs) }
                #[inline]
                fn raw_div(self, rhs: Self) -> Self {
                    // Preserve Rust's normal divide-by-zero panic; the only
                    // other failure mode (e.g. i32::MIN / -1) is a genuine
                    // overflow, which we saturate to MAX.
                    self.checked_div(rhs).unwrap_or_else(|| {
                        if rhs == 0 { self / rhs } else { <$t>::MAX }
                    })
                }
                #[inline]
                fn raw_rem(self, rhs: Self) -> Self {
                    self.checked_rem(rhs).unwrap_or_else(|| {
                        if rhs == 0 { self % rhs } else { 0 }
                    })
                }
            }
        )*
    };
}
impl_clamp_num_int!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);
macro_rules! impl_clamp_num_float {
    ($($t:ty),*) => {
        $(
            impl ClampNum for $t {
                // IEEE 754 floats already "saturate" to +/-infinity instead
                // of panicking, so plain arithmetic is safe here.
                #[inline]
                fn raw_add(self, rhs: Self) -> Self { self + rhs }
                #[inline]
                fn raw_sub(self, rhs: Self) -> Self { self - rhs }
                #[inline]
                fn raw_mul(self, rhs: Self) -> Self { self * rhs }
                #[inline]
                fn raw_div(self, rhs: Self) -> Self { self / rhs }
                #[inline]
                fn raw_rem(self, rhs: Self) -> Self { self % rhs }
            }
        )*
    };
}
impl_clamp_num_float!(f32, f64);

/// A numeric value clamped to a `[min, max]` range.
///
/// Any operation that would push the value outside the configured bounds
/// saturates to the nearest bound rather than overflowing, wrapping, or
/// panicking.
///
/// ```
/// use doodads::math::Clamp;
///
/// let mut health = Clamp::new(80u32, 0, 100);
/// health += 50;
/// assert_eq!(health.value(), 100); // saturated at max
///
/// health -= 1000;
/// assert_eq!(health.value(), 0); // saturated at min, no panic
/// ```
#[inline]
fn clamp_value<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Clamp<T> {
    value: T,
    min: T,
    max: T,
}
impl<T: ClampNum> Clamp<T> {
    /// Creates a new `Clamp`, immediately saturating `value` into `[min, max]`.
    ///
    /// # Panics
    /// Panics if `min > max`.
    pub fn new(value: T, min: T, max: T) -> Self {
        assert!(min <= max, "Clamp::new: min must be <= max");
        Self {
            value: clamp_value(value, min, max),
            min,
            max,
        }
    }

    pub fn try_new(value: T, min: T, max: T) -> Option<Self> {
        (min <= max).then_some(Self {
            value: clamp_value(value, min, max),
            min,
            max,
        })
    }

    pub unsafe fn new_unchecked(value: T, min: T, max: T) -> Self {
        Self { value, min, max }
    }

    /// The current (always in-bounds) value.
    #[inline]
    pub fn value(&self) -> T {
        self.value
    }

    /// The lower bound.
    #[inline]
    pub fn min(&self) -> T {
        self.min
    }

    /// The upper bound.
    #[inline]
    pub fn max(&self) -> T {
        self.max
    }

    /// Replaces the inner value, saturating it into the existing range.
    pub fn set(&mut self, value: T) {
        self.value = clamp_value(value, self.min, self.max);
    }

    pub fn set_min(&mut self, min: T) {
        self.min = min;
    }

    pub fn set_max(&mut self, max: T) {
        self.max = max;
    }

    /// Changes the bounds, re-saturating the current value into the new range.
    ///
    /// # Panics
    /// Panics if `min > max`.
    pub fn set_bounds(&mut self, min: T, max: T) {
        assert!(min <= max, "Clamp::set_bounds: min must be <= max");
        self.min = min;
        self.max = max;
        self.value = clamp_value(self.value, min, max);
    }

    /// Attempts to change the bounds, leaving them unchanged if `min > max`.
    /// Returns `true` if the bounds were updated.
    pub fn try_set_bounds(&mut self, min: T, max: T) -> bool {
        if min > max {
            return false;
        }
        self.min = min;
        self.max = max;
        self.value = clamp_value(self.value, min, max);
        true
    }

    /// `true` if the value is currently pinned to the lower bound.
    pub fn is_at_min(&self) -> bool {
        self.value == self.min
    }

    /// `true` if the value is currently pinned to the upper bound.
    pub fn is_at_max(&self) -> bool {
        self.value == self.max
    }

    /// Distance from the current value up to `max`.
    pub fn headroom(&self) -> T {
        self.max.raw_sub(self.value)
    }

    /// Maps the value linearly into `[0.0, 1.0]` across `[min, max]`.
    /// Returns `0.0` for a zero-width range.
    pub fn fraction(&self) -> f64
    where
        T: ToPrimitive,
    {
        let span = (self.max.raw_sub(self.min)).to_f64().unwrap_or(0.0);
        if span == 0.0 {
            return 0.0;
        }
        let pos = (self.value.raw_sub(self.min)).to_f64().unwrap_or(0.0);
        pos / span
    }

    #[inline]
    fn apply(&mut self, rhs: T, op: impl FnOnce(T, T) -> T) {
        self.value = clamp_value(op(self.value, rhs), self.min, self.max);
    }

    #[inline]
    fn combined(self, rhs: T, op: impl FnOnce(T, T) -> T) -> Self {
        Self {
            value: clamp_value(op(self.value, rhs), self.min, self.max),
            min: self.min,
            max: self.max,
        }
    }
}
impl<T: ClampNum> ClampRangeApi<T, RangeInclusive<T>> for Clamp<T> {
    fn from_range(value: T, range: RangeInclusive<T>) -> Self {
        let (min, max) = range.into_inner();
        Clamp { value, min, max }
    }

    fn set_bounds_from_range(&mut self, range: RangeInclusive<T>) {
        let (min, max) = range.into_inner();
        self.min = min;
        self.max = max;
    }

    fn try_set_bounds_from_range(&mut self, range: RangeInclusive<T>) -> bool {
        let (min, max) = range.into_inner();

        if min <= max {
            self.try_set_bounds(min, max)
        } else {
            false
        }
    }
}
impl<T: ClampNum + Bounded> ClampRangeApi<T, RangeFrom<T>> for Clamp<T> {
    fn from_range(value: T, range: RangeFrom<T>) -> Self {
        Self {
            value,
            min: range.start,
            max: T::max_value(),
        }
    }

    fn set_bounds_from_range(&mut self, range: RangeFrom<T>) {
        (self.min, self.max) = (range.start, T::max_value());
    }

    /// Infallible.
    fn try_set_bounds_from_range(&mut self, range: RangeFrom<T>) -> bool {
        self.set_bounds_from_range(range);
        true
    }
}
impl<T: ClampNum + Bounded> ClampRangeApi<T, RangeTo<T>> for Clamp<T> {
    fn from_range(value: T, range: RangeTo<T>) -> Self {
        Self {
            value,
            min: T::min_value(),
            max: range.end,
        }
    }

    fn set_bounds_from_range(&mut self, range: RangeTo<T>) {
        (self.min, self.max) = (T::min_value(), range.end);
    }

    /// Infallible.
    fn try_set_bounds_from_range(&mut self, range: RangeTo<T>) -> bool {
        self.set_bounds_from_range(range);
        true
    }
}
impl<T: ClampNum + Bounded> Default for Clamp<T> {
    /// Defaults to the full representable range of `T`, with `value` set to zero.
    fn default() -> Self {
        Self::new(T::zero(), T::min_value(), T::max_value())
    }
}
impl<T: ClampNum + Signed> Neg for Clamp<T> {
    type Output = Clamp<T>;
    fn neg(self) -> Self::Output {
        let negated = T::zero().raw_sub(self.value);
        Self {
            value: clamp_value(negated, self.min, self.max),
            min: self.min,
            max: self.max,
        }
    }
}
impl<T: PartialEq> PartialEq for Clamp<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<T: PartialEq> PartialEq<T> for Clamp<T> {
    fn eq(&self, other: &T) -> bool {
        self.value == *other
    }
}
impl<T: PartialOrd> PartialOrd for Clamp<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}
impl<T: PartialOrd> PartialOrd<T> for Clamp<T> {
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.value.partial_cmp(other)
    }
}
impl<T> Deref for Clamp<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T: fmt::Display> fmt::Display for Clamp<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
macro_rules! impl_op {
    ($trait:ident, $method:ident, $assign_trait:ident, $assign_method:ident, $raw:ident) => {
        impl<T: ClampNum> $trait<T> for Clamp<T> {
            type Output = Clamp<T>;
            fn $method(self, rhs: T) -> Self::Output {
                self.combined(rhs, T::$raw)
            }
        }
        impl<T: ClampNum> $assign_trait<T> for Clamp<T> {
            fn $assign_method(&mut self, rhs: T) {
                self.apply(rhs, T::$raw);
            }
        }
        impl<T: ClampNum> $trait<Clamp<T>> for Clamp<T> {
            type Output = Clamp<T>;
            fn $method(self, rhs: Clamp<T>) -> Self::Output {
                self.combined(rhs.value, T::$raw)
            }
        }
    };
}
impl_op!(Add, add, AddAssign, add_assign, raw_add);
impl_op!(Sub, sub, SubAssign, sub_assign, raw_sub);
impl_op!(Mul, mul, MulAssign, mul_assign, raw_mul);
impl_op!(Div, div, DivAssign, div_assign, raw_div);
impl_op!(Rem, rem, RemAssign, rem_assign, raw_rem);

pub trait ClampRangeApi<T, R> {
    fn from_range(value: T, range: R) -> Self;
    fn set_bounds_from_range(&mut self, range: R);
    fn try_set_bounds_from_range(&mut self, range: R) -> bool;
}

trait UnFuck
where
    Self: Sized,
{
    fn unfuck(self) -> Option<Self>;
}
impl<T: Float> UnFuck for T {
    fn unfuck(self) -> Option<Self> {
        if self.is_nan() || self.is_infinite() {
            None
        } else {
            Some(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturates_on_add_and_sub_unsigned() {
        let mut hp = Clamp::new(80u32, 0, 100);
        hp += 50;
        assert_eq!(hp.value(), 100);
        hp -= 1000; // would underflow u32 entirely; must not panic
        assert_eq!(hp.value(), 0);
    }

    #[test]
    fn saturates_at_type_boundary_not_just_clamp_bounds() {
        // min/max wider than the type's own range pressure point.
        let mut c = Clamp::new(u8::MAX - 1, 0, 255);
        c += 10; // 254 + 10 would overflow u8 itself
        assert_eq!(c.value(), 255);
    }

    #[test]
    fn works_with_signed_ints_including_min_edge_case() {
        let mut c = Clamp::new(0i32, -10, 10);
        c -= 100;
        assert_eq!(c.value(), -10);
        c += 1000;
        assert_eq!(c.value(), 10);
        c = -c;
        assert_eq!(c.value(), -10);

        // i32::MIN negation overflows i32 itself; must saturate, not panic.
        let extreme = Clamp::new(i32::MIN, i32::MIN, i32::MAX);
        let negated = -extreme;
        assert_eq!(negated.value(), i32::MAX);
    }

    #[test]
    fn works_with_floats() {
        let mut c = Clamp::new(0.5f64, 0.0, 1.0);
        c *= 10.0;
        assert_eq!(c.value(), 1.0);
        c -= 5.0;
        assert_eq!(c.value(), 0.0);
        assert_eq!(c.fraction(), 0.0);
    }

    #[test]
    fn integer_division_overflow_edge_case() {
        let c = Clamp::new(i32::MIN, i32::MIN, i32::MAX);
        let divided = c / -1; // mathematically overflows i32::MAX by 1
        assert_eq!(divided.value(), i32::MAX);
    }

    #[test]
    fn binary_op_between_two_clamps() {
        let a = Clamp::new(5i32, 0, 10);
        let b = Clamp::new(8i32, 0, 100);
        let sum = a + b; // 13, saturates to a's max of 10
        assert_eq!(sum.value(), 10);
    }

    #[test]
    fn comparisons_and_deref() {
        let c = Clamp::new(42u8, 0, 255);
        assert_eq!(c, 42);
        assert!(c < 100);
        assert_eq!(*c, 42);
    }

    #[test]
    fn default_uses_full_type_range() {
        let c: Clamp<i8> = Clamp::default();
        assert_eq!(c.min(), i8::MIN);
        assert_eq!(c.max(), i8::MAX);
        assert_eq!(c.value(), 0);
    }

    #[test]
    #[should_panic]
    fn panics_on_invalid_bounds() {
        let _ = Clamp::new(0i32, 10, -10);
    }
}
