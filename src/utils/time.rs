/*
amFOSS Daemon: A discord bot for the amFOSS Discord server.
Copyright (C) 2024 amFOSS

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use chrono::{DateTime, Datelike, Local, TimeZone};
use chrono_tz::Asia::Kolkata;
use chrono_tz::Tz;
use tracing::debug;

use std::time::Duration;

pub fn time_until(hour: u32, minute: u32) -> Duration {
    debug!(
        "time_until called with args hour: {}, minute: {}",
        hour, minute
    );

    let now = Local::now().with_timezone(&Kolkata);
    let today_run = Kolkata
        .with_ymd_and_hms(now.year(), now.month(), now.day(), hour, minute, 0)
        .single()
        .expect("Valid datetime must be created");

    let next_run = if now < today_run {
        today_run
    } else {
        today_run + chrono::Duration::days(1)
    };

    debug!("now: {}, today_run: {}", now, today_run);

    let duration = next_run.signed_duration_since(now);
    debug!("duration: {}", duration);
    Duration::from_secs(duration.num_seconds().max(0) as u64)
}

pub fn get_five_forty_five_pm_timestamp(now: DateTime<Tz>) -> DateTime<Local> {
    let date =
        chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), now.day()).expect("Invalid date");
    let time = chrono::NaiveTime::from_hms_opt(17, 45, 0).expect("Invalid time");
    let naive_dt = date.and_time(time);

    chrono::Local
        .from_local_datetime(&naive_dt)
        .single()
        .expect("Chrono must work.")
}
