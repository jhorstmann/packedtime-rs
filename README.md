# PackedTime-RS

## Utilities for efficiently storing, parsing, formatting and truncating timestamps

 - A bit-packed timestamp representation using the same layout as `i64` (`PackedTimestamp`).
   Each timestamp component uses the minimal number of bits, leaving enough bits for
   arbitrary timezone offsets in minutes and enough range for years from -9999 to 9999.
   This is a useful storage format if the timestamps are only parsed, formatted or compared. 
 - SIMD optimized parsing and formatting functions using [rfc 3339 format](https://datatracker.ietf.org/doc/html/rfc3339).
   In microbenchmarks these functions are ~20x faster than using [chrono][chrono]
 - Optimized functions for truncating timestamps to year, month, quarter, week or day precision.
   When used in compute kernels with arrays as input and output these functions are 2x-3x faster compared to [chrono][chrono].

## Usage

### Parsing Timestamps

Parsing uses SSE instructions when available and has a special fast-path when the millisecond uses 3 digits and the timezone is UTC.
Without SSE a hand-written recursive descent parser is used.

```rust
assert_eq!(
    "2022-08-21T17:30:15.250Z".parse(),
    Ok(PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 250))
);
assert_eq!(
    "2022-08-21T17:30:15.25Z".parse(),
    Ok(PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 250))
);
assert_eq!(
    "2022-08-21 17:30:15.1Z".parse(),
    Ok(PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 100))
);
assert_eq!(
    "2022-08-21 17:30:15Z".parse(),
    Ok(PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 0))
);
```

### Formatting Timestamps

Note that formatting currently ignores the timezone offset and always writes a `Z` as the offset.

Milliseconds are always included and printed using 3 digits.

```rust
assert_eq!(
    PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 100).to_string(),
    "2022-08-21T17:30:15.100Z".to_owned()
);
assert_eq!(
    PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 123).to_string(),
    "2022-08-21T17:30:15.123Z".to_owned()
);
assert_eq!(
    PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 250).to_string(),
    "2022-08-21T17:30:15.250Z".to_owned()
);
```




The package [net.jhorstmann:packedtime](https://github.com/jhorstmann/packedtime) implements the same packed layout for Java.


 [chrono]: https://crates.io/crates/chrono