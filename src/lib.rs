#![allow(unused_parens)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::manual_range_contains)]

mod epoch_days;
mod error;
mod format;
mod kernels;
mod packed;
mod parse;
mod util;

pub use epoch_days::*;
pub use error::*;
pub use format::*;
pub use kernels::*;
pub use packed::*;
pub use parse::*;

pub(crate) const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1000;
