# PackedTime-RS

### Utilities for efficiently storing, parsing, formatting and truncating timestamps

 - A bit-packed timestamp representation using the same layout as `i64` (`PackedTimestamp`).
   Each timestamp component uses the minimal number of bits, leaving enough bits for
   arbitrary timezone offsets in minutes and enough range for years from -9999 to 9999.
   This is a useful storage format if the timestamps are only parsed, formatted or compared. 
 - SIMD optimized parsing and formatting functions using [rfc 3339 format](https://datatracker.ietf.org/doc/html/rfc3339).
   In microbenchmarks these functions are ~20x faster than using [chrono][chrono]
 - Optimized functions for truncating timestamps to year, month, quarter, week or day precision.
   When used in compute kernels with arrays as input and output these functions are 2x-3x faster compared to [chrono][chrono].



The package [net.jhorstmann:packedtime](https://github.com/jhorstmann/packedtime) implements the same packed layout for Java.


 [chrono]: https://crates.io/crates/chrono