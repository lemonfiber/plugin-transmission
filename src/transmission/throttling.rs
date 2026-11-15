use async_trait::async_trait;
use lemonfiber_ports::service::{
    Failure, Hours, Metering, Moved, Rates, Throttled, Throttling, Wanted, Window,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{Transmission, SESSION_STATS};

const UNLIMITED: u64 = 9_007_199_254_740_992;

const SLOWEST: u64 = 1;

const EVERY_DAY: u8 = 127;

const MINUTES_AN_HOUR: u16 = 60;

const LAST_HOUR: u8 = 23;

const LAST_MINUTE: u8 = 59;

const DOWN_LIMIT: &str = "speed_limit_down";

const DOWN_LIMITED: &str = "speed_limit_down_enabled";

const UP_LIMIT: &str = "speed_limit_up";

const UP_LIMITED: &str = "speed_limit_up_enabled";

const SCHEDULED_DOWN: &str = "alt_speed_down";

const SCHEDULED_UP: &str = "alt_speed_up";

const SCHEDULED_NOW: &str = "alt_speed_enabled";

const SCHEDULED: &str = "alt_speed_time_enabled";

const SCHEDULE_BEGINS: &str = "alt_speed_time_begin";

const SCHEDULE_ENDS: &str = "alt_speed_time_end";

const SCHEDULE_DAYS: &str = "alt_speed_time_day";

const UNITS: &str = "units";

const LIMITS: [&str; 9] = [
    DOWN_LIMIT,
    DOWN_LIMITED,
    UP_LIMIT,
    UP_LIMITED,
    SCHEDULED_DOWN,
    SCHEDULED_UP,
    SCHEDULED_NOW,
    SCHEDULED,
    UNITS,
];

#[derive(Deserialize)]
struct Units {
    speed_bytes: u64,
}

#[derive(Deserialize)]
struct Measured {
    units: Units,
}

#[derive(Deserialize)]
struct Ordinary {
    #[serde(rename = "speed_limit_down")]
    down: u64,
    #[serde(rename = "speed_limit_down_enabled")]
    down_limited: bool,
    #[serde(rename = "speed_limit_up")]
    up: u64,
    #[serde(rename = "speed_limit_up_enabled")]
    up_limited: bool,
}

#[derive(Deserialize)]
struct Alternative {
    #[serde(rename = "alt_speed_down")]
    down: u64,
    #[serde(rename = "alt_speed_up")]
    up: u64,
    #[serde(rename = "alt_speed_enabled")]
    now: bool,
    #[serde(rename = "alt_speed_time_enabled")]
    scheduled: bool,
}

#[derive(Deserialize)]
struct Limits {
    #[serde(flatten)]
    ordinary: Ordinary,
    #[serde(flatten)]
    alternative: Alternative,
    units: Units,
}

#[derive(Deserialize)]
struct Counted {
    downloaded_bytes: u64,
    uploaded_bytes: u64,
}

#[derive(Deserialize)]
struct Figures {
    download_speed: u64,
    upload_speed: u64,
    current_stats: Counted,
}

impl Ordinary {
    fn rates(&self, unit: u64) -> Rates {
        Rates {
            down: self.down_limited.then(|| self.down.saturating_mul(unit)),
            up: self.up_limited.then(|| self.up.saturating_mul(unit)),
        }
    }
}

impl Alternative {
    fn rates(&self, unit: u64) -> Rates {
        Rates {
            down: scheduled(self.down, unit),
            up: scheduled(self.up, unit),
        }
    }

    fn hours(&self) -> Option<Hours> {
        self.scheduled.then_some(if self.now {
            Hours::Active
        } else {
            Hours::Quiet
        })
    }
}

impl Transmission {
    async fn speed_unit(&self, whenever: &str) -> Result<u64, Failure> {
        let measured: Measured = self.session(&[UNITS], whenever).await?;
        self.unit_of(&measured.units)
    }

    fn unit_of(&self, units: &Units) -> Result<u64, Failure> {
        Some(units.speed_bytes)
            .filter(|bytes| *bytes > 0)
            .ok_or_else(|| self.endpoint.refused("Transmission names no speed unit"))
    }

    async fn stats(&self) -> Result<Figures, Failure> {
        self.call(
            SESSION_STATS,
            json!({}),
            "the transfer figures could not be read",
        )
        .await
    }
}

#[async_trait]
impl Throttling for Transmission {
    async fn throttled(&self) -> Result<Throttled, Failure> {
        let held: Limits = self
            .session(&LIMITS, "the rate limits could not be read")
            .await?;
        let unit = self.unit_of(&held.units)?;
        let alternative = &held.alternative;
        Ok(Throttled {
            rates: if alternative.now {
                alternative.rates(unit)
            } else {
                held.ordinary.rates(unit)
            },
            uploads: true,
            hours: alternative.hours(),
        })
    }

    async fn restrain(&self, wanted: &Wanted) -> Result<Throttled, Failure> {
        if !wanted.window.is_none_or(on_the_clock) {
            return Err(self
                .endpoint
                .refused("that window is not hours and minutes of a day"));
        }
        let unit = self.speed_unit("the speed unit could not be read").await?;
        self.set(
            settings_of(wanted, unit),
            "the rate limits could not be set",
        )
        .await?;
        self.throttled().await
    }

    async fn moving(&self) -> Result<Rates, Failure> {
        let moving = self.stats().await?;
        Ok(Rates {
            down: Some(moving.download_speed),
            up: Some(moving.upload_speed),
        })
    }
}

#[async_trait]
impl Metering for Transmission {
    async fn moved(&self, _month: &str) -> Result<Moved, Failure> {
        let moved = self.stats().await?.current_stats;
        Ok(Moved {
            down: moved.downloaded_bytes,
            up: moved.uploaded_bytes,
            since_start: true,
        })
    }
}

fn settings_of(wanted: &Wanted, unit: u64) -> Map<String, Value> {
    let ordinary = if wanted.window.is_some() {
        wanted.quiet
    } else {
        wanted.active
    };
    let mut asked = Map::from_iter([
        (DOWN_LIMITED.to_owned(), ordinary.down.is_some().into()),
        (UP_LIMITED.to_owned(), ordinary.up.is_some().into()),
        (SCHEDULED.to_owned(), wanted.window.is_some().into()),
    ]);
    for (key, rate) in [(DOWN_LIMIT, ordinary.down), (UP_LIMIT, ordinary.up)] {
        if let Some(bytes) = rate {
            asked.insert(key.to_owned(), figure(bytes, unit).into());
        }
    }
    match wanted.window {
        Some(window) => asked.extend([
            (
                SCHEDULED_DOWN.to_owned(),
                limited(wanted.active.down, unit).into(),
            ),
            (
                SCHEDULED_UP.to_owned(),
                limited(wanted.active.up, unit).into(),
            ),
            (
                SCHEDULE_BEGINS.to_owned(),
                minutes(window.from_hour, window.from_minute).into(),
            ),
            (
                SCHEDULE_ENDS.to_owned(),
                minutes(window.to_hour, window.to_minute).into(),
            ),
            (SCHEDULE_DAYS.to_owned(), EVERY_DAY.into()),
        ]),
        None => {
            asked.insert(SCHEDULED_NOW.to_owned(), false.into());
        }
    }
    asked
}

fn limited(rate: Option<u64>, unit: u64) -> u64 {
    rate.map_or(UNLIMITED, |bytes| figure(bytes, unit))
}

fn figure(bytes: u64, unit: u64) -> u64 {
    (bytes / unit).clamp(SLOWEST, UNLIMITED - 1)
}

fn scheduled(held: u64, unit: u64) -> Option<u64> {
    (held != UNLIMITED).then(|| held.saturating_mul(unit))
}

fn minutes(hour: u8, minute: u8) -> u16 {
    u16::from(hour) * MINUTES_AN_HOUR + u16::from(minute)
}

fn on_the_clock(window: Window) -> bool {
    window.from_hour <= LAST_HOUR
        && window.to_hour <= LAST_HOUR
        && window.from_minute <= LAST_MINUTE
        && window.to_minute <= LAST_MINUTE
}

#[cfg(test)]
mod tests;
