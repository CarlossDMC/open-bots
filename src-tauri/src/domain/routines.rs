use chrono::{DateTime, Days, LocalResult, NaiveTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DomainError, DomainResult};

pub const MAX_ROUTINES_PER_AGENT: usize = 50;
pub const MIN_INTERVAL_MINUTES: u32 = 5;
pub const MAX_INTERVAL_MINUTES: u32 = 7 * 24 * 60;
pub const MAX_ROUTINE_NAME_LENGTH: usize = 80;
pub const MAX_ROUTINE_INSTRUCTIONS_LENGTH: usize = 4_000;

/// When a routine fires. Daily times are wall-clock times in the user's local time zone.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RoutineSchedule {
    Interval { minutes: u32 },
    Daily { hour: u32, minute: u32 },
}

impl RoutineSchedule {
    pub fn validate(&self) -> DomainResult<()> {
        match *self {
            Self::Interval { minutes } => {
                if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&minutes) {
                    return Err(DomainError::Validation(format!(
                        "routine interval must be between {MIN_INTERVAL_MINUTES} and {MAX_INTERVAL_MINUTES} minutes"
                    )));
                }
            }
            Self::Daily { hour, minute } => {
                if NaiveTime::from_hms_opt(hour, minute, 0).is_none() {
                    return Err(DomainError::Validation(
                        "routine daily time must be a valid hour and minute".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// The first run strictly after `after`. Assumes the schedule has been validated.
    pub fn next_after<Tz: TimeZone>(&self, after: DateTime<Utc>, zone: &Tz) -> DateTime<Utc> {
        match *self {
            Self::Interval { minutes } => after + chrono::Duration::minutes(i64::from(minutes)),
            Self::Daily { hour, minute } => next_daily(after, zone, hour, minute),
        }
    }
}

fn next_daily<Tz: TimeZone>(
    after: DateTime<Utc>,
    zone: &Tz,
    hour: u32,
    minute: u32,
) -> DateTime<Utc> {
    let time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or(NaiveTime::MIN);
    let start = after.with_timezone(zone).date_naive();
    // A daily time can be skipped by a daylight-saving gap, so look a few days ahead.
    for offset in 0..=3 {
        let Some(date) = start.checked_add_days(Days::new(offset)) else {
            break;
        };
        let candidate = match zone.from_local_datetime(&date.and_time(time)) {
            LocalResult::Single(value) => value,
            LocalResult::Ambiguous(earliest, _) => earliest,
            LocalResult::None => continue,
        };
        let candidate = candidate.with_timezone(&Utc);
        if candidate > after {
            return candidate;
        }
    }
    after + chrono::Duration::days(1)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewRoutine {
    pub agent_id: Uuid,
    pub name: String,
    pub instructions: String,
    pub schedule: RoutineSchedule,
}

/// Recurring work an agent should be woken for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Routine {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub name: String,
    pub instructions: String,
    pub schedule: RoutineSchedule,
    pub enabled: bool,
    pub next_run_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Routine {
    /// Creates an enabled routine. `existing_routines` is how many the agent already has.
    pub fn create<Tz: TimeZone>(
        input: NewRoutine,
        existing_routines: usize,
        now: DateTime<Utc>,
        zone: &Tz,
    ) -> DomainResult<Self> {
        if existing_routines >= MAX_ROUTINES_PER_AGENT {
            return Err(DomainError::Validation(format!(
                "an agent can have at most {MAX_ROUTINES_PER_AGENT} routines"
            )));
        }
        let name = bounded_text(&input.name, "routine name", MAX_ROUTINE_NAME_LENGTH)?;
        let instructions = bounded_text(
            &input.instructions,
            "routine instructions",
            MAX_ROUTINE_INSTRUCTIONS_LENGTH,
        )?;
        input.schedule.validate()?;
        Ok(Self {
            id: Uuid::new_v4(),
            agent_id: input.agent_id,
            name,
            instructions,
            schedule: input.schedule,
            enabled: true,
            next_run_at: input.schedule.next_after(now, zone),
            last_run_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        self.enabled && self.next_run_at <= now
    }

    /// Re-enabling starts the schedule from `now`, so paused time never produces a burst of runs.
    pub fn set_enabled<Tz: TimeZone>(&mut self, enabled: bool, now: DateTime<Utc>, zone: &Tz) {
        if enabled && !self.enabled {
            self.next_run_at = self.schedule.next_after(now, zone);
        }
        self.enabled = enabled;
        self.updated_at = now;
    }

    /// Records a run and schedules the next one after `now`. Runs missed while the
    /// application was closed collapse into this single run.
    pub fn record_run<Tz: TimeZone>(&mut self, now: DateTime<Utc>, zone: &Tz) {
        self.last_run_at = Some(now);
        self.next_run_at = self.schedule.next_after(now, zone);
        self.updated_at = now;
    }
}

fn bounded_text(value: &str, label: &str, max: usize) -> DomainResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(DomainError::Validation(format!("{label} cannot be empty")));
    }
    if value.chars().count() > max {
        return Err(DomainError::Validation(format!(
            "{label} cannot exceed {max} characters"
        )));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn at(value: &str) -> DateTime<Utc> {
        value.parse().expect("timestamp")
    }

    fn input(schedule: RoutineSchedule) -> NewRoutine {
        NewRoutine {
            agent_id: Uuid::new_v4(),
            name: " Morning digest ".into(),
            instructions: " Summarize open pull requests ".into(),
            schedule,
        }
    }

    #[test]
    fn creates_an_enabled_routine_scheduled_after_now() {
        let now = at("2026-01-10T10:00:00Z");
        let routine = Routine::create(
            input(RoutineSchedule::Interval { minutes: 30 }),
            0,
            now,
            &Utc,
        )
        .expect("routine");
        assert!(routine.enabled);
        assert_eq!(routine.name, "Morning digest");
        assert_eq!(routine.instructions, "Summarize open pull requests");
        assert_eq!(routine.next_run_at, at("2026-01-10T10:30:00Z"));
        assert!(!routine.is_due(now));
        assert!(routine.is_due(at("2026-01-10T10:30:00Z")));
    }

    #[test]
    fn rejects_invalid_routines() {
        let now = Utc::now();
        let mut empty = input(RoutineSchedule::Interval { minutes: 30 });
        empty.name = " ".into();
        assert!(Routine::create(empty, 0, now, &Utc).is_err());
        for schedule in [
            RoutineSchedule::Interval { minutes: 4 },
            RoutineSchedule::Interval {
                minutes: MAX_INTERVAL_MINUTES + 1,
            },
            RoutineSchedule::Daily {
                hour: 24,
                minute: 0,
            },
            RoutineSchedule::Daily {
                hour: 9,
                minute: 60,
            },
        ] {
            assert!(Routine::create(input(schedule), 0, now, &Utc).is_err());
        }
    }

    #[test]
    fn limits_routines_per_agent() {
        let schedule = RoutineSchedule::Interval { minutes: 30 };
        let now = Utc::now();
        assert!(Routine::create(input(schedule), MAX_ROUTINES_PER_AGENT - 1, now, &Utc).is_ok());
        assert!(Routine::create(input(schedule), MAX_ROUTINES_PER_AGENT, now, &Utc).is_err());
    }

    #[test]
    fn schedules_daily_runs_in_the_local_time_zone() {
        let zone = FixedOffset::west_opt(3 * 3600).expect("offset");
        let schedule = RoutineSchedule::Daily { hour: 9, minute: 0 };
        // 08:00 local is before today's run.
        assert_eq!(
            schedule.next_after(at("2026-01-10T11:00:00Z"), &zone),
            at("2026-01-10T12:00:00Z")
        );
        // Exactly 09:00 local moves to tomorrow.
        assert_eq!(
            schedule.next_after(at("2026-01-10T12:00:00Z"), &zone),
            at("2026-01-11T12:00:00Z")
        );
    }

    #[test]
    fn collapses_missed_runs_into_one() {
        let created = at("2026-01-10T10:00:00Z");
        let mut routine = Routine::create(
            input(RoutineSchedule::Interval { minutes: 30 }),
            0,
            created,
            &Utc,
        )
        .expect("routine");
        let much_later = at("2026-01-10T15:10:00Z");
        assert!(routine.is_due(much_later));
        routine.record_run(much_later, &Utc);
        assert_eq!(routine.last_run_at, Some(much_later));
        assert_eq!(routine.next_run_at, at("2026-01-10T15:40:00Z"));
        assert!(!routine.is_due(much_later));
    }

    #[test]
    fn re_enabling_restarts_the_schedule_from_now() {
        let created = at("2026-01-10T10:00:00Z");
        let mut routine = Routine::create(
            input(RoutineSchedule::Interval { minutes: 30 }),
            0,
            created,
            &Utc,
        )
        .expect("routine");
        routine.set_enabled(false, created, &Utc);
        assert!(!routine.is_due(at("2026-01-10T12:00:00Z")));
        routine.set_enabled(true, at("2026-01-10T12:00:00Z"), &Utc);
        assert_eq!(routine.next_run_at, at("2026-01-10T12:30:00Z"));
    }
}
