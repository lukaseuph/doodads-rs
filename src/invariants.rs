use std::{borrow::Borrow, fmt, marker::PhantomData, ops::Deref};

pub trait StringRule {
    /// Checks if the string satisfies this rule.
    fn check(&self, string: &str) -> bool;
    /// Applies this rule to the string, modifying the string if needed.
    /// If the string is unfixable, returns `None`.
    fn apply(&self, string: String) -> Option<String>;
}

/// A [`String`] that is guaranteed to satisfy a given [`StringRule`].
///
/// # Examples
///
/// ```
/// use doodads::invariants::{InvariantString, LowerCase};
///
/// let value = InvariantString::new("Hello World", LowerCase);
/// assert!(value.is_none());
///
/// let value = InvariantString::new("hello world", LowerCase);
/// assert!(value.is_some());
/// ```
///
/// It can be used to enforce rules in function arguments.
/// ```
/// use doodads::invariants::{InvariantString, LowerCase};
///
/// // No need to verify that the s is lowercase.
/// // The type guarantees that.
/// fn foo(s: InvariantString<LowerCase>) {
///     println!("{}", s);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvariantString<I: StringRule> {
    _phantom: PhantomData<I>,
    string: String,
}
impl<I: StringRule> InvariantString<I> {
    /// Creates a new [`InvariantString`] without checking if the string satisfies the rule.
    ///
    /// # Safety
    /// The string must satisfy the rule, otherwise the invariant will not be upheld.
    pub unsafe fn new_unchecked(string: impl Into<String>) -> Self {
        Self {
            _phantom: PhantomData,
            string: string.into(),
        }
    }

    /// Creates a new [`InvariantString`] by checking if the string satisfies the rule.
    /// If the string does not satisfy the rule, this method returns `None`.
    pub fn new(string: impl Into<String>, rule: I) -> Option<Self> {
        let string = string.into();
        rule.check(&string)
            .then(|| unsafe { Self::new_unchecked(string) })
    }

    /// Creates a new [`InvariantString`] by applying the rule to the string.
    /// The rule is able to modfiy the string to make it satisfy the rule.
    /// If the string is unfixable, this method returns `None`.
    pub fn new_applied(string: impl Into<String>, rule: I) -> Option<Self> {
        rule.apply(string.into())
            .map(|s| unsafe { Self::new_unchecked(s) })
    }
}
impl<I: StringRule> From<&InvariantStr<I>> for InvariantString<I> {
    fn from(value: &InvariantStr<I>) -> Self {
        unsafe { Self::new_unchecked(value.string.to_string()) }
    }
}
impl<I: StringRule> From<InvariantString<I>> for String {
    fn from(value: InvariantString<I>) -> Self {
        value.string
    }
}
impl<I: StringRule> Deref for InvariantString<I> {
    type Target = InvariantStr<I>;
    fn deref(&self) -> &Self::Target {
        unsafe { InvariantStr::new_unchecked(&self.string) }
    }
}
impl<I: StringRule> AsRef<str> for InvariantString<I> {
    fn as_ref(&self) -> &str {
        &self.string
    }
}
impl<I: StringRule> AsRef<InvariantStr<I>> for InvariantString<I> {
    fn as_ref(&self) -> &InvariantStr<I> {
        unsafe { InvariantStr::new_unchecked(&self.string) }
    }
}
impl<I: StringRule> Borrow<str> for InvariantString<I> {
    fn borrow(&self) -> &str {
        &self.string
    }
}
impl<I: StringRule> Borrow<InvariantStr<I>> for InvariantString<I> {
    fn borrow(&self) -> &InvariantStr<I> {
        unsafe { InvariantStr::new_unchecked(&self.string) }
    }
}
impl<I: StringRule> fmt::Display for InvariantString<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.string.fmt(f)
    }
}

/// A [`str`] that is guaranteed to satisfy a given rule.
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvariantStr<I: StringRule> {
    _phantom: PhantomData<I>,
    string: str,
}
impl<I: StringRule> InvariantStr<I> {
    pub unsafe fn new_unchecked(s: &str) -> &InvariantStr<I> {
        unsafe { &*(s as *const str as *const InvariantStr<I>) }
    }

    pub fn new<'a>(string: &'a str, rule: I) -> Option<&'a InvariantStr<I>> {
        rule.check(string)
            .then(|| unsafe { Self::new_unchecked(string) })
    }
}
impl<'a, I: StringRule> From<&'a InvariantStr<I>> for &'a str {
    fn from(value: &'a InvariantStr<I>) -> Self {
        &value.string
    }
}
impl<T: StringRule> ToOwned for InvariantStr<T> {
    type Owned = InvariantString<T>;
    fn to_owned(&self) -> Self::Owned {
        unsafe { InvariantString::new_unchecked(&self.string) }
    }
}
impl<I: StringRule> Deref for InvariantStr<I> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.string
    }
}
impl<I: StringRule> fmt::Display for InvariantStr<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.string.fmt(f)
    }
}

/// Guarantees that there are no uppercase characters in the string.
/// Characters that have no case are treated as non-uppercase.
pub struct LowerCase;
impl StringRule for LowerCase {
    fn check(&self, string: &str) -> bool {
        string.chars().all(|c| !c.is_uppercase())
    }

    fn apply(&self, string: String) -> Option<String> {
        Some(string.to_lowercase())
    }
}

/// Guarantees that there are only lowercase characters in the string.
/// Characters that have no case are treated as non-lowercase.
pub struct StrictLowerCase;
impl StringRule for StrictLowerCase {
    fn check(&self, string: &str) -> bool {
        string.chars().all(char::is_lowercase)
    }

    fn apply(&self, string: String) -> Option<String> {
        let lower = string.to_lowercase();
        lower.chars().all(char::is_lowercase).then_some(lower)
    }
}

/// Guarantees that there are no lowercase characters in the string.
/// Characters that have no case are treated as non-lowercase.
pub struct UpperCase;
impl StringRule for UpperCase {
    fn check(&self, string: &str) -> bool {
        string.chars().all(|c| !c.is_lowercase())
    }

    fn apply(&self, string: String) -> Option<String> {
        Some(string.to_uppercase())
    }
}

/// Guarantees that there are only uppercase characters in the string.
/// Characters that have no case are treated as non-uppercase.
pub struct StrictUpperCase;
impl StringRule for StrictUpperCase {
    fn check(&self, string: &str) -> bool {
        string.chars().all(char::is_uppercase)
    }

    fn apply(&self, string: String) -> Option<String> {
        let upper = string.to_uppercase();
        upper.chars().all(char::is_uppercase).then_some(upper)
    }
}

pub struct AsciiLowerCase;
impl StringRule for AsciiLowerCase {
    fn check(&self, string: &str) -> bool {
        string.bytes().all(|b| b.is_ascii_lowercase())
    }

    fn apply(&self, string: String) -> Option<String> {
        Some(string.to_lowercase())
    }
}

pub struct AsciiUpperCase;
impl StringRule for AsciiUpperCase {
    fn check(&self, string: &str) -> bool {
        string.bytes().all(|b| b.is_ascii_uppercase())
    }

    fn apply(&self, string: String) -> Option<String> {
        Some(string.to_uppercase())
    }
}

pub struct Alphanumeric;
impl StringRule for Alphanumeric {
    fn check(&self, string: &str) -> bool {
        string.chars().all(char::is_alphanumeric)
    }

    fn apply(&self, string: String) -> Option<String> {
        self.check(&string).then_some(string)
    }
}

/// Guarantees that a string's character set uses only lowercase ASCII characters and underscores.
/// Does not enforce snake case rules such as leading/trailing underscores or consecutive underscores.
pub struct SnakeCase;
impl StringRule for SnakeCase {
    fn check(&self, string: &str) -> bool {
        string
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    }

    fn apply(&self, string: String) -> Option<String> {
        string
            .bytes()
            .map(|b| {
                let b = b.to_ascii_lowercase();
                (b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_').then_some(b)
            })
            .collect::<Option<Vec<u8>>>()
            // SAFETY: we know the bytes are ASCII lowercase or underscore
            .map(|bytes| unsafe { String::from_utf8_unchecked(bytes) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_case() {
        // valid
        assert!(LowerCase.check("foo"));
        assert!(LowerCase.check("foo bar"));
        // invalid
        assert!(!LowerCase.check("Foo"));

        // valid
        assert_eq!(LowerCase.apply("foo".to_string()), Some("foo".to_string()));
        // fixable
        assert_eq!(LowerCase.apply("Foo".to_string()), Some("foo".to_string()));
        assert_eq!(
            LowerCase.apply("Foo BAR".to_string()),
            Some("foo bar".to_string())
        );
        // there is no unfixable case
    }

    #[test]
    fn upper_case() {
        // valid
        assert!(UpperCase.check("FOO"));
        assert!(UpperCase.check("FOO BAR"));
        // invalid
        assert!(!UpperCase.check("foo"));

        // valid
        assert_eq!(UpperCase.apply("FOO".to_string()), Some("FOO".to_string()));
        // fixable
        assert_eq!(UpperCase.apply("foo".to_string()), Some("FOO".to_string()));
        assert_eq!(
            UpperCase.apply("foo bar".to_string()),
            Some("FOO BAR".to_string())
        );
        // there is no unfixable case
    }

    #[test]
    fn snake_case() {
        // valid
        assert!(SnakeCase.check("foo_bar"));
        // invalid
        assert!(!SnakeCase.check("FooBar"));

        // valid
        assert_eq!(
            SnakeCase.apply("foo_bar".to_string()),
            Some("foo_bar".to_string())
        );
        // fixable
        assert_eq!(
            SnakeCase.apply("FooBar".to_string()),
            Some("foobar".to_string())
        );
        // unfixable
        assert_eq!(SnakeCase.apply("foo_bar.".to_string()), None);
    }
}
