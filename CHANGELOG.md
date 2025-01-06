# Changelog

## [0.3.2](https://github.com/jhorstmann/packedtime-rs/tree/0.3.1) (2025-01-06)

 - Add `days_in_month` kernel

## [0.3.1](https://github.com/jhorstmann/packedtime-rs/tree/0.3.1) (2023-11-03)

 - The `date_add_month` kernels should leave the time part unchanged

## [0.3.0](https://github.com/jhorstmann/packedtime-rs/tree/0.3.0) (2023-11-01)

 - Support calculating the difference between dates in units of years or months
 - More reliable auto-vectorization due to less branches

## [0.2.6](https://github.com/jhorstmann/packedtime-rs/tree/0.2.6) (2023-07-03)

 - Fixed bug with negative timezone offsets in `PackedTimestamp`
 - Refactoring

## [0.2.5](https://github.com/jhorstmann/packedtime-rs/tree/0.2.5) (2023-04-17)

 - Support running with miri

## [0.2.4](https://github.com/jhorstmann/packedtime-rs/tree/0.2.4) (2023-03-06)

 - Improved handling of fractional seconds with simd instructions

## [0.2.3](https://github.com/jhorstmann/packedtime-rs/tree/0.2.3) (2022-09-15)

 - Add checks for target-arch and target-features to support compilation for non-x86 targets