
#[cfg(feature = "std")]
#[rustfmt::skip]
mod std;
#[cfg(feature = "std")]
pub use std::*;

#[cfg(not(feature = "std"))]
#[rustfmt::skip]
mod nostd;
#[cfg(not(feature = "std"))]
pub use nostd::*;
