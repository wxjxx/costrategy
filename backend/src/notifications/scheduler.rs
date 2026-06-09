use crate::dingtalk::DingTalkClient;
use crate::notifications::{NotificationRepository, ReminderNotificationService};
use crate::tasks::TaskRepository;
use crate::users::UserRepository;
use chrono::{DateTime, Duration as ChronoDuration, Local, NaiveDateTime, NaiveTime, TimeZone};
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
            let now = Local::now();
            let next_run = next_local_daily_run(now, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
            let sleep_for = (next_run - now)
                .to_std()
                .unwrap_or_else(|_| std::time::Duration::from_secs(60));
            tokio::time::sleep(sleep_for).await;
            if let Err(error) = service.notify_due_tomorrow(Local::now().date_naive()).await {
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
            let now = Local::now();
            let next_run = next_local_daily_run(now, NaiveTime::from_hms_opt(9, 10, 0).unwrap());
            let sleep_for = (next_run - now)
                .to_std()
                .unwrap_or_else(|_| std::time::Duration::from_secs(60));
            tokio::time::sleep(sleep_for).await;
            if let Err(error) = service.notify_overdue(Local::now().date_naive()).await {
                log::error!("task_overdue scheduler failed: {error}");
            }
        }
    })
}

fn next_local_daily_run(now: DateTime<Local>, target_time: NaiveTime) -> DateTime<Local> {
    let naive = next_daily_run_naive(now.naive_local(), target_time);
    Local
        .from_local_datetime(&naive)
        .single()
        .unwrap_or_else(|| now + ChronoDuration::minutes(1))
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
