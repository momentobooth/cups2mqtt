use std::{fmt::Formatter, str::FromStr, time::Duration};

use chrono::Local;
use croner::Cron;
use serde::{de::{self, Visitor}, Deserialize};
use snafu::{ResultExt, Snafu};

#[derive(Debug)]
pub enum TimeSchedule {
    Interval(Duration),
    Cron(Cron),
}

impl TimeSchedule {
    pub fn get_duration_till_next_occurrence(&self) -> Result<Duration, ScheduleError> {
        match self {
            TimeSchedule::Cron(cron) => {
                let now = Local::now();
                let next_occurrence = cron.find_next_occurrence(&now, true).with_whatever_context(|_| format!("Could not resolve next occurrence from cron {cron}"))?;
                (next_occurrence - now).to_std().with_whatever_context(|_| "Could not convert TimeDelta to Duration")
            },
            TimeSchedule::Interval(duration) => Ok(*duration),
        }
    }
}

impl<'de> Deserialize<'de> for TimeSchedule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        struct TimeScheduleVisitor;

        impl<'de> Visitor<'de> for TimeScheduleVisitor {
            type Value = TimeSchedule;

            fn expecting(&self, formatter: &mut Formatter) -> Result<(), std::fmt::Error> {
                formatter.write_str("a cron expression or a duration string")
            }

            fn visit_str<E>(self, value: &str) -> Result<TimeSchedule, E>
            where
                E: de::Error,
            {
                // First try parsing as cron syntax.
                if let Ok(cron) = Cron::from_str(value) {
                    return Ok(TimeSchedule::Cron(cron));
                }

                // Now try humantime (e.g. '30m' for 30 minutes).
                if let Ok(duration) = humantime::parse_duration(value) {
                    return Ok(TimeSchedule::Interval(duration));
                }

                Err(E::custom(format!("Invalid time schedule string: '{}'", value)))
            }
        }

        deserializer.deserialize_str(TimeScheduleVisitor)
    }
}

// ////// //
// Errors //
// ////// //

#[derive(Debug, Snafu)]
pub enum ScheduleError {
    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error + Send + Sync>, Some)))]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}
