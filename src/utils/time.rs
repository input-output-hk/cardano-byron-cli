use std::time::{self, SystemTime};
use std::{fmt, ops::Deref};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time(SystemTime);
impl Time {
    pub fn now() -> Self {
        Time(SystemTime::now())
    }
}
impl Deref for Time {
    type Target = SystemTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s: String = format!("{}", ::humantime::format_rfc3339(self.0))
            .chars()
            .take(10)
            .collect();
        write!(f, "{}", s)
    }
}
impl From<SystemTime> for Time {
    fn from(st: SystemTime) -> Time {
        Time(st)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(time::Duration);
impl Duration {
    pub fn since(t: Time) -> Self {
        let now = Time::now();
        Self::between(t, now)
    }
    pub fn between(t1: Time, t2: Time) -> Self {
        match t1.duration_since(*t2) {
            Ok(duration) => Duration::from(duration),
            Err(_) => match t2.duration_since(*t1) {
                Ok(duration) => Duration::from(duration),
                Err(err) => {
                    unreachable!("error when trying to get duration between 2 dates, {}", err)
                }
            },
        }
    }
}
impl Deref for Duration {
    type Target = time::Duration;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<time::Duration> for Duration {
    fn from(d: time::Duration) -> Self {
        Duration(time::Duration::new(d.as_secs(), 0))
    }
}
impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{}", ::humantime::format_duration(self.0));
        write!(f, "{}", s)
    }
}
