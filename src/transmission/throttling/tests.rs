use lemonfiber_fixtures::http::Answer;
use lemonfiber_ports::service::{
    Failure, Hours, Metering, Moved, Rates, Throttled, Throttling, Wanted, Window,
};
use serde_json::{json, Map, Value};

use super::super::tests::{answered, call, called, client, DONE};
use super::UNLIMITED;

const UNITS: &str = r#"{"units":{"speed_bytes":1000}}"#;

const STATS: &str = r#"{"download_speed":1200,"upload_speed":300,"active_torrent_count":1,
    "current_stats":{"downloaded_bytes":5000,"uploaded_bytes":700,"seconds_active":60},
    "cumulative_stats":{"downloaded_bytes":90000,"uploaded_bytes":8000,"seconds_active":600}}"#;

fn fresh() -> Map<String, Value> {
    let held = json!({
        "speed_limit_down": 100,
        "speed_limit_down_enabled": false,
        "speed_limit_up": 100,
        "speed_limit_up_enabled": false,
        "alt_speed_down": 50,
        "alt_speed_up": 50,
        "alt_speed_enabled": false,
        "alt_speed_time_enabled": false,
        "units": {"speed_bytes": 1000}
    });
    held.as_object().cloned().unwrap_or_default()
}

fn holding(changed: &Map<String, Value>, scheduled_now: bool) -> Answer {
    let mut held = fresh();
    held.extend(changed.clone());
    if changed.get("alt_speed_time_enabled") == Some(&Value::Bool(true)) {
        held.insert("alt_speed_enabled".to_owned(), scheduled_now.into());
    }
    answered(&Value::Object(held).to_string())
}

fn settings(written: &Value) -> Map<String, Value> {
    written.as_object().cloned().unwrap_or_default()
}

const DAY: Window = Window {
    from_hour: 8,
    from_minute: 30,
    to_hour: 23,
    to_minute: 0,
};

async fn restrained(wanted: Wanted, written: &Value, scheduled_now: bool) -> Option<Throttled> {
    let (transmission, http) = client(vec![
        answered(UNITS),
        answered(DONE),
        holding(&settings(written), scheduled_now),
    ]);
    let read = transmission.restrain(&wanted).await.ok();
    assert_eq!(
        called(&http).get(1),
        Some(&call("session_set", written.clone()))
    );
    read
}

#[tokio::test]
async fn with_no_window_the_session_limits_hold_the_active_rates_and_the_schedule_is_off() {
    let active = Rates {
        down: Some(2_000_000),
        up: Some(500_000),
    };
    let read = restrained(
        Wanted {
            active,
            quiet: Rates::default(),
            window: None,
        },
        &json!({
            "speed_limit_down_enabled": true,
            "speed_limit_up_enabled": true,
            "alt_speed_time_enabled": false,
            "speed_limit_down": 2000,
            "speed_limit_up": 500,
            "alt_speed_enabled": false
        }),
        false,
    )
    .await;
    assert_eq!(
        read,
        Some(Throttled {
            rates: active,
            uploads: true,
            hours: None,
        })
    );
}

#[tokio::test]
async fn a_window_with_both_directions_limited_puts_them_on_the_schedule_and_the_quiet_hours_free()
{
    let active = Rates {
        down: Some(4_000_000),
        up: Some(1_000_000),
    };
    let wanted = Wanted {
        active,
        quiet: Rates::default(),
        window: Some(DAY),
    };
    let written = json!({
        "speed_limit_down_enabled": false,
        "speed_limit_up_enabled": false,
        "alt_speed_time_enabled": true,
        "alt_speed_down": 4000,
        "alt_speed_up": 1000,
        "alt_speed_time_begin": 510,
        "alt_speed_time_end": 1380,
        "alt_speed_time_day": 127
    });
    assert_eq!(
        restrained(wanted, &written, true).await,
        Some(Throttled {
            rates: active,
            uploads: true,
            hours: Some(Hours::Active),
        })
    );
    assert_eq!(
        restrained(wanted, &written, false).await,
        Some(Throttled {
            rates: Rates::default(),
            uploads: true,
            hours: Some(Hours::Quiet),
        })
    );
}

#[tokio::test]
async fn a_direction_left_unlimited_in_active_hours_is_held_at_the_unlimited_figure_and_read_back_as_none(
) {
    let active = Rates {
        down: Some(3_000_000),
        up: None,
    };
    let read = restrained(
        Wanted {
            active,
            quiet: Rates::default(),
            window: Some(DAY),
        },
        &json!({
            "speed_limit_down_enabled": false,
            "speed_limit_up_enabled": false,
            "alt_speed_time_enabled": true,
            "alt_speed_down": 3000,
            "alt_speed_up": UNLIMITED,
            "alt_speed_time_begin": 510,
            "alt_speed_time_end": 1380,
            "alt_speed_time_day": 127
        }),
        true,
    )
    .await;
    assert_eq!(
        read,
        Some(Throttled {
            rates: active,
            uploads: true,
            hours: Some(Hours::Active),
        })
    );
}

#[tokio::test]
async fn quiet_hours_with_limits_of_their_own_hold_them_on_the_session_limits() {
    let quiet = Rates {
        down: Some(9_000_000),
        up: None,
    };
    let read = restrained(
        Wanted {
            active: Rates::default(),
            quiet,
            window: Some(DAY),
        },
        &json!({
            "speed_limit_down_enabled": true,
            "speed_limit_up_enabled": false,
            "alt_speed_time_enabled": true,
            "speed_limit_down": 9000,
            "alt_speed_down": UNLIMITED,
            "alt_speed_up": UNLIMITED,
            "alt_speed_time_begin": 510,
            "alt_speed_time_end": 1380,
            "alt_speed_time_day": 127
        }),
        false,
    )
    .await;
    assert_eq!(
        read,
        Some(Throttled {
            rates: quiet,
            uploads: true,
            hours: Some(Hours::Quiet),
        })
    );
}

#[test]
fn a_rate_is_whole_units_rounded_down_and_never_below_one_or_at_the_unlimited_figure() {
    assert_eq!(super::figure(1_048_576, 1000), 1048);
    assert_eq!(super::figure(999, 1000), 1);
    assert_eq!(super::figure(0, 1000), 1);
    assert_eq!(super::figure(u64::MAX, 1), UNLIMITED - 1);
    assert_eq!(super::scheduled(UNLIMITED, 1000), None);
    assert_eq!(super::scheduled(0, 1000), Some(0));
    assert_eq!(
        super::scheduled(UNLIMITED - 1, 1000),
        Some((UNLIMITED - 1) * 1000)
    );
}

#[tokio::test]
async fn a_window_off_the_clock_is_refused_before_anything_is_asked() {
    for window in [
        Window {
            from_hour: 24,
            ..DAY
        },
        Window { to_hour: 24, ..DAY },
        Window {
            from_minute: 60,
            ..DAY
        },
        Window {
            to_minute: 60,
            ..DAY
        },
    ] {
        let (transmission, http) = client(vec![]);
        let refused = transmission
            .restrain(&Wanted {
                active: Rates::default(),
                quiet: Rates::default(),
                window: Some(window),
            })
            .await;
        assert!(matches!(
            refused,
            Err(Failure::Refused { detail, .. }) if detail == "that window is not hours and minutes of a day"
        ));
        assert!(http.requests().is_empty());
    }
}

#[tokio::test]
async fn a_client_that_names_no_speed_unit_is_refused() {
    let (transmission, http) = client(vec![answered(r#"{"units":{"speed_bytes":0}}"#)]);
    let refused = transmission
        .restrain(&Wanted {
            active: Rates::default(),
            quiet: Rates::default(),
            window: None,
        })
        .await;
    assert!(matches!(
        refused,
        Err(Failure::Refused { detail, .. }) if detail == "Transmission names no speed unit"
    ));
    assert_eq!(http.requests().len(), 1);
    let (refused, _) = client(vec![answered(UNITS), Answer::reply(500, "")]);
    assert!(refused
        .restrain(&Wanted {
            active: Rates::default(),
            quiet: Rates::default(),
            window: None,
        })
        .await
        .is_err());
}

#[tokio::test]
async fn the_session_limits_in_force_are_read_through_its_speed_unit() {
    let mut held = fresh();
    held.extend(settings(&json!({
        "speed_limit_down_enabled": true,
        "speed_limit_down": 250,
        "units": {"speed_bytes": 1024}
    })));
    let (transmission, http) = client(vec![answered(&Value::Object(held).to_string())]);
    assert_eq!(
        transmission.throttled().await.ok(),
        Some(Throttled {
            rates: Rates {
                down: Some(256_000),
                up: None,
            },
            uploads: true,
            hours: None,
        })
    );
    assert_eq!(
        called(&http)
            .first()
            .and_then(|body| body.pointer("/params/fields").cloned()),
        Some(json!(super::LIMITS))
    );
    let mut unitless = fresh();
    unitless.insert("units".to_owned(), json!({"speed_bytes": 0}));
    let (transmission, _) = client(vec![answered(&Value::Object(unitless).to_string())]);
    assert!(transmission.throttled().await.is_err());
}

#[tokio::test]
async fn how_fast_it_moves_and_what_it_moved_since_it_started_are_its_session_figures() {
    let (transmission, http) = client(vec![answered(STATS), answered(STATS)]);
    assert_eq!(
        transmission.moving().await.ok(),
        Some(Rates {
            down: Some(1200),
            up: Some(300),
        })
    );
    assert_eq!(
        transmission.moved("2026-01").await.ok(),
        Some(Moved {
            down: 5000,
            up: 700,
            since_start: true,
        })
    );
    assert_eq!(
        called(&http).first(),
        Some(&call("session_stats", json!({})))
    );
    let (refused, _) = client(vec![Answer::reply(500, "")]);
    assert!(refused.moved("2026-01").await.is_err());
}
