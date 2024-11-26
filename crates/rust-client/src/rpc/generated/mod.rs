#[cfg(feature = "std")]
mod std;
#[cfg(feature = "std")]
pub use std::*;

#[cfg(not(feature = "std"))]
mod nostd;
#[cfg(not(feature = "std"))]
pub use nostd::*;
