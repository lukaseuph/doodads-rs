use std::time::{Duration, Instant};

#[macro_export]
macro_rules! st {
    ($label:expr, $callback: expr) => {
        let _st = $crate::bench::ScopeTimer::new($label, $callback);
    };
    (print $label:expr) => {
        let _st = $crate::bench::ScopeTimer::print($label);
    };
}

/// Tracks how long the scope it is constructed in takes to execute using a [Drop] implementation.
/// Always construct `ScopeTimer` at the top of the scope.
/// Upon being dropped, `ScopeTimer` calls the function passed in during initialization.
/// Use [ScopeTimer::print] for just printing the duration and the timer's label.
/// For situations where you need to pass in a function or closure, use [ScopeTimer::new].
/// # Examples
/// ```rust
/// use doodads::bench::ScopeTimer;
///
/// fn foo() {
///     // will print how long the function took
///     let _st = ScopeTimer::print("example::foo");
///     {
///         // will print how long the block took
///         let _st = ScopeTimer::print("example::foo block");
///     }
/// }
/// ```
pub struct ScopeTimer<F: FnMut(&str, Duration)> {
    label: String,
    start_time: Instant,
    callback: F,
}
impl<F: FnMut(&str, Duration)> ScopeTimer<F> {
    pub fn new(label: impl ToString, callback: F) -> Self {
        Self {
            label: label.to_string(),
            start_time: Instant::now(),
            callback,
        }
    }
}
impl ScopeTimer<fn(&str, Duration)> {
    pub fn print(label: impl ToString) -> Self {
        Self {
            label: label.to_string(),
            start_time: Instant::now(),
            callback: |label, duration| println!("{label}: {duration:?}"),
        }
    }
}
impl<F: FnMut(&str, Duration)> Drop for ScopeTimer<F> {
    fn drop(&mut self) {
        (self.callback)(&self.label, self.start_time.elapsed());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn two_scope_timers() {
        st!(print "#1");
        println!("middle");
        st!(print "#2");
    }
}
