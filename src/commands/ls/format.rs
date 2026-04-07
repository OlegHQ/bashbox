//! Long listing formatting (`ls -l`).

use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn format_mode(mode: u32, is_dir: bool, is_link: bool) -> String {
    let file_type = if is_link {
        'l'
    } else if is_dir {
        'd'
    } else {
        '-'
    };

    let perms = [
        if mode & 0o400 != 0 { 'r' } else { '-' },
        if mode & 0o200 != 0 { 'w' } else { '-' },
        if mode & 0o100 != 0 { 'x' } else { '-' },
        if mode & 0o040 != 0 { 'r' } else { '-' },
        if mode & 0o020 != 0 { 'w' } else { '-' },
        if mode & 0o010 != 0 { 'x' } else { '-' },
        if mode & 0o004 != 0 { 'r' } else { '-' },
        if mode & 0o002 != 0 { 'w' } else { '-' },
        if mode & 0o001 != 0 { 'x' } else { '-' },
    ];

    format!("{}{}", file_type, perms.iter().collect::<String>())
}

pub(crate) fn format_size(size: u64, human_readable: bool) -> String {
    if !human_readable {
        return size.to_string();
    }

    if size < 1024 {
        return size.to_string();
    }
    if size < 1024 * 1024 {
        let k = size as f64 / 1024.0;
        return if k < 10.0 {
            format!("{:.1}K", k)
        } else {
            format!("{}K", k as u64)
        };
    }
    if size < 1024 * 1024 * 1024 {
        let m = size as f64 / (1024.0 * 1024.0);
        return if m < 10.0 {
            format!("{:.1}M", m)
        } else {
            format!("{}M", m as u64)
        };
    }
    let g = size as f64 / (1024.0 * 1024.0 * 1024.0);
    if g < 10.0 {
        format!("{:.1}G", g)
    } else {
        format!("{}G", g as u64)
    }
}

pub(crate) fn format_time(mtime: SystemTime) -> String {
    let duration = mtime.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let days_since_epoch = secs / 86400;
    let year = 1970 + (days_since_epoch / 365) as i32;
    let day_of_year = days_since_epoch % 365;
    let month = (day_of_year / 30).min(11) as usize;
    let day = (day_of_year % 30) + 1;

    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let six_months_ago = now_secs.saturating_sub(180 * 86400);

    if secs > six_months_ago {
        format!("{} {:>2} {:02}:{:02}", months[month], day, hour, minute)
    } else {
        format!("{} {:>2}  {}", months[month], day, year)
    }
}
