use crate::dingtalk::{DingTalkClient, DingtalkSyncService};
use crate::time::{shanghai_datetime, shanghai_now};
use crate::users::UserRepository;
use chrono::{Duration as ChronoDuration, NaiveDateTime, NaiveTime};
use tokio::task::JoinHandle;

const CONTACT_SYNC_HOUR: u32 = 3;

pub fn start_contact_sync_scheduler<C, R>(service: DingtalkSyncService<C, R>) -> JoinHandle<()>
where
    C: DingTalkClient,
    R: UserRepository,
{
    tokio::spawn(async move {
        loop {
            let now = shanghai_now();
            let next_run = next_contact_sync_run(now.naive_local());
            let sleep_for = (next_run - now)
                .to_std()
                .unwrap_or_else(|_| std::time::Duration::from_secs(60));
            tokio::time::sleep(sleep_for).await;
            if let Err(error) = service.sync_contacts().await {
                log::error!("dingtalk contact sync scheduler failed: {error}");
            }
        }
    })
}

pub fn next_contact_sync_run_after(now: NaiveDateTime) -> NaiveDateTime {
    let target_time = NaiveTime::from_hms_opt(CONTACT_SYNC_HOUR, 0, 0).unwrap();
    let today_target = now.date().and_time(target_time);
    if now < today_target {
        today_target
    } else {
        today_target + ChronoDuration::days(1)
    }
}

fn next_contact_sync_run(now: NaiveDateTime) -> chrono::DateTime<chrono_tz::Tz> {
    let naive = next_contact_sync_run_after(now);
    shanghai_datetime(naive.date(), naive.time())
}
