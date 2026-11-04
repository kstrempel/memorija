//! Build script: captures the UTC build date as `BUILD_DATE=YYYY-MM-DD`.
//!
//! Deliberately dependency-free (design D1): the days-since-epoch to civil
//! date conversion is Howard Hinnant's `civil_from_days` algorithm in plain
//! `std`. `SOURCE_DATE_EPOCH` is honored when set so reproducible builds
//! stay reproducible.

use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let days: u64 = match std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    {
        Some(epoch) => epoch / 86_400,
        None => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before the Unix epoch")
            .as_secs()
            / 86_400,
    };
    let (year, month, day) = civil_from_days(days as i64);
    println!("cargo:rustc-env=BUILD_DATE={year:04}-{month:02}-{day:02}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
}

/// Days since 1970-01-01 to a `(year, month, day)` civil date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64; // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365; // [0, 399]
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let mp = (5 * day_of_year + 2) / 153; // [0, 11]
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if month <= 2 { year + 1 } else { year }, month, day)
}
