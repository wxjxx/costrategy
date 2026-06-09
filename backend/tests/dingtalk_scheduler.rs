use chrono::NaiveDate;
use costrategy_backend::dingtalk::next_contact_sync_run_after;

#[test]
fn contact_sync_scheduler_runs_daily_at_three_am_local_time() {
    let before_target = NaiveDate::from_ymd_opt(2026, 6, 9)
        .unwrap()
        .and_hms_opt(2, 59, 0)
        .unwrap();
    let at_target = NaiveDate::from_ymd_opt(2026, 6, 9)
        .unwrap()
        .and_hms_opt(3, 0, 0)
        .unwrap();

    assert_eq!(
        next_contact_sync_run_after(before_target),
        NaiveDate::from_ymd_opt(2026, 6, 9)
            .unwrap()
            .and_hms_opt(3, 0, 0)
            .unwrap()
    );
    assert_eq!(
        next_contact_sync_run_after(at_target),
        NaiveDate::from_ymd_opt(2026, 6, 10)
            .unwrap()
            .and_hms_opt(3, 0, 0)
            .unwrap()
    );
}
