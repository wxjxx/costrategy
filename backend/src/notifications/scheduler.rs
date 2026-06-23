use crate::dingtalk::DingTalkClient;
use crate::notifications::{NotificationRepository, ReminderNotificationService};
use crate::tasks::TaskRepository;
use crate::time::{shanghai_datetime, shanghai_now, shanghai_today};
use crate::users::UserRepository;
use chrono::{Duration as ChronoDuration, NaiveDateTime, NaiveTime};
use tokio::task::JoinHandle;

pub fn start_due_tomorrow_scheduler<C, U, N, T>(
    service: ReminderNotificationService<C, U, N, T>,
) -> JoinHandle<()>
where
    C: DingTalkClient,
    U: UserRepository,
    N: NotificationRepository,
    T: TaskRepository,
{
    tokio::spawn(async move {
        loop {
            let now = shanghai_now();
            let next_run = next_local_daily_run(now.naive_local(), NaiveTime::from_hms_opt(9, 0, 0).unwrap());
            let sleep_for = (next_run - now)
                .to_std()
                .unwrap_or_else(|_| std::time::Duration::from_secs(60));
            tokio::time::sleep(sleep_for).await;
            if let Err(error) = service.notify_due_tomorrow(shanghai_today()).await {
                log::error!("due_tomorrow scheduler failed: {error}");
            }
        }
    })
}

pub fn start_overdue_scheduler<C, U, N, T>(
    service: ReminderNotificationService<C, U, N, T>,
) -> JoinHandle<()>
where
    C: DingTalkClient,
    U: UserRepository,
    N: NotificationRepository,
    T: TaskRepository,
{
    tokio::spawn(async move {
        loop {
            let now = shanghai_now();
            let next_run = next_local_daily_run(now.naive_local(), NaiveTime::from_hms_opt(9, 10, 0).unwrap());
            let sleep_for = (next_run - now)
                .to_std()
                .unwrap_or_else(|_| std::time::Duration::from_secs(60));
            tokio::time::sleep(sleep_for).await;
            if let Err(error) = service.notify_overdue(shanghai_today()).await {
                log::error!("task_overdue scheduler failed: {error}");
            }
        }
    })
}

fn next_local_daily_run(now: NaiveDateTime, target_time: NaiveTime) -> chrono::DateTime<chrono_tz::Tz> {
    let naive = next_daily_run_naive(now, target_time);
    shanghai_datetime(naive.date(), naive.time())
}

fn next_daily_run_naive(now: NaiveDateTime, target_time: NaiveTime) -> NaiveDateTime {
    let today_target = now.date().and_time(target_time);
    if now < today_target {
        today_target
    } else {
        today_target + ChronoDuration::days(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn next_daily_run_moves_to_tomorrow_after_target_time() {
        let now = NaiveDate::from_ymd_opt(2026, 6, 9)
            .unwrap()
            .and_hms_opt(9, 10, 0)
            .unwrap();

        let next_run = next_daily_run_naive(now, NaiveTime::from_hms_opt(9, 0, 0).unwrap());

        assert_eq!(
            next_run,
            NaiveDate::from_ymd_opt(2026, 6, 10)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap()
        );
    }
}
