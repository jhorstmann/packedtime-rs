use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use packedtime_rs::{
    date_part_month_timestamp_millis, date_part_year_timestamp_millis,
    date_trunc_month_timestamp_millis, date_trunc_month_timestamp_millis_float,
    date_trunc_year_timestamp_millis, date_trunc_year_timestamp_millis_float,
    days_in_month_timestamp_millis,
};
use std::hint::unreachable_unchecked;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[inline(never)]
fn bench_date_trunc_year(input: &[i64], output: &mut [i64]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_trunc_year_timestamp_millis(input);
        });
}

#[inline(never)]
fn bench_date_trunc_month(input: &[i64], output: &mut [i64]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_trunc_month_timestamp_millis(input);
        });
}

#[inline(never)]
fn bench_date_trunc_year_float(input: &[f64], output: &mut [f64]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_trunc_year_timestamp_millis_float(input);
        });
}

#[inline(never)]
fn bench_date_trunc_month_float(input: &[f64], output: &mut [f64]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_trunc_month_timestamp_millis_float(input);
        });
}

#[inline(never)]
fn bench_date_part_year(input: &[i64], output: &mut [i32]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_part_year_timestamp_millis(input);
        });
}

#[inline(never)]
fn bench_date_part_month(input: &[i64], output: &mut [i32]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_part_month_timestamp_millis(input);
        });
}

#[inline(never)]
fn bench_date_trunc_year_chrono(input: &[i64], output: &mut [i64]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            let ndt = NaiveDateTime::from_timestamp(input / 1000, 0);
            let truncated = NaiveDateTime::new(
                NaiveDate::from_ymd(ndt.year(), 1, 1),
                NaiveTime::from_hms(0, 0, 0),
            );
            *output = truncated.timestamp_millis();
        });
}

#[inline(never)]
fn bench_date_trunc_month_chrono(input: &[i64], output: &mut [i64]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            let ndt = NaiveDateTime::from_timestamp(input / 1000, 0);
            let truncated = NaiveDateTime::new(
                NaiveDate::from_ymd(ndt.year(), ndt.month(), 1),
                NaiveTime::from_hms(0, 0, 0),
            );
            *output = truncated.timestamp_millis();
        });
}

#[inline(never)]
fn bench_date_part_year_chrono(input: &[i64], output: &mut [i32]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            let ndt = NaiveDateTime::from_timestamp(input / 1000, 0);
            *output = ndt.year();
        });
}

#[inline(never)]
fn bench_date_part_month_chrono(input: &[i64], output: &mut [i32]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            let ndt = NaiveDateTime::from_timestamp(input / 1000, 0);
            *output = ndt.month() as i32;
        });
}

#[inline(never)]
fn bench_days_in_month(input: &[i64], output: &mut [i32]) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = days_in_month_timestamp_millis(input);
        });
}

#[inline(never)]
fn bench_days_in_month_chrono(input: &[i64], output: &mut [i32]) {
    #[inline]
    fn is_leap_year(year: i32) -> bool {
        return year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    }
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            let ndt = NaiveDateTime::from_timestamp(input / 1000, 0);
            *output = match ndt.month() {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => {
                    if is_leap_year(ndt.year()) {
                        29
                    } else {
                        28
                    }
                }
                _ => unsafe { unreachable_unchecked() },
            }
        });
}

pub fn bench_date_trunc(c: &mut Criterion) {
    const BATCH_SIZE: usize = 1024;

    let mut rng = StdRng::seed_from_u64(42);

    let input = (0..BATCH_SIZE)
        .map(|_| rng.gen_range(0..4102444800_000_i64))
        .collect::<Vec<_>>();

    let input_float = input.iter().map(|i| *i as f64).collect::<Vec<_>>();

    let mut output = vec![0_i64; BATCH_SIZE];
    let mut output_float = vec![0_f64; BATCH_SIZE];
    let mut output_int = vec![0_i32; BATCH_SIZE];

    c.benchmark_group("date_trunc")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * 2 * std::mem::size_of::<i64>()) as u64,
        ))
        .bench_function("date_trunc_year", |b| {
            b.iter(|| bench_date_trunc_year(&input, &mut output))
        })
        .bench_function("date_trunc_month", |b| {
            b.iter(|| bench_date_trunc_month(&input, &mut output))
        })
        .bench_function("date_trunc_year_float", |b| {
            b.iter(|| bench_date_trunc_year_float(&input_float, &mut output_float))
        })
        .bench_function("date_trunc_month_float", |b| {
            b.iter(|| bench_date_trunc_month_float(&input_float, &mut output_float))
        })
        .bench_function("date_trunc_year_chrono", |b| {
            b.iter(|| bench_date_trunc_year_chrono(&input, &mut output))
        })
        .bench_function("date_trunc_month_chrono", |b| {
            b.iter(|| bench_date_trunc_month_chrono(&input, &mut output))
        });

    c.benchmark_group("date_part")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * (std::mem::size_of::<i64>() + std::mem::size_of::<i32>())) as u64,
        ))
        .bench_function("date_part_year", |b| {
            b.iter(|| bench_date_part_year(&input, &mut output_int))
        })
        .bench_function("date_part_month", |b| {
            b.iter(|| bench_date_part_month(&input, &mut output_int))
        })
        .bench_function("date_part_year_chrono", |b| {
            b.iter(|| bench_date_part_year_chrono(&input, &mut output_int))
        })
        .bench_function("date_part_month_chrono", |b| {
            b.iter(|| bench_date_part_month_chrono(&input, &mut output_int))
        });

    c.benchmark_group("days_in_month")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * (std::mem::size_of::<i64>() + std::mem::size_of::<i32>())) as u64,
        ))
        .bench_function("days_in_month", |b| {
            b.iter(|| bench_days_in_month(&input, &mut output_int))
        })
        .bench_function("days_in_month_chrono", |b| {
            b.iter(|| bench_days_in_month_chrono(&input, &mut output_int))
        });
}

criterion_group!(benches, bench_date_trunc);
criterion_main!(benches);
