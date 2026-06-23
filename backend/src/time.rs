use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::{Asia::Shanghai, Tz};

pub const APP_TIMEZONE_NAME: &str = "Asia/Shanghai";

pub fn shanghai_now() -> DateTime<Tz> {
    Utc::now().with_timezone(&Shanghai)
}

pub fn shanghai_today() -> NaiveDate {
    shanghai_now().date_naive()
}

pub fn utc_to_shanghai(value: DateTime<Utc>) -> DateTime<Tz> {
    value.with_timezone(&Shanghai)
}

pub fn shanghai_datetime(date: NaiveDate, time: NaiveTime) -> DateTime<Tz> {
    let naive = NaiveDateTime::new(date, time);
    match Shanghai.from_local_datetime(&naive) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(first, _) => first,
        LocalResult::None => Utc::now().with_timezone(&Shanghai),
    }
}
