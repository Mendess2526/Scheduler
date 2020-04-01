use crate::timetable::{index_to_time, TimeTable};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use icalendar::{Calendar, Component, Event};
use std::{
    collections::HashMap,
    io::{self, Write},
    iter::successors,
};

#[derive(Debug)]
struct EventBuilder<'a> {
    summary: &'a str,
    start: Option<NaiveTime>,
    end: Option<NaiveTime>,
}

impl<'a> EventBuilder<'a> {
    fn new(summary: &'a str) -> Self {
        Self {
            summary,
            start: None,
            end: None,
        }
    }

    fn starts(mut self, t: NaiveTime) -> Self {
        if self.start != None {
            panic!("Can't start twice");
        }
        self.start = Some(t);
        self
    }

    fn ends(&mut self, t: NaiveTime) {
        if self.end < Some(t) {
            self.end = Some(t)
        }
    }
}

pub fn write_cal<W: Write>(
    out: &mut W,
    time: &TimeTable,
    start_day: NaiveDate,
    end_day: NaiveDate,
) -> io::Result<()> {
    write!(
        out,
        "{}",
        time.iter()
            .map(|table| {
                table.fold(
                    HashMap::default(),
                    |mut acc: HashMap<_, EventBuilder>, (i, tp, c)| {
                        let t = index_to_time(i);
                        acc.entry((tp, c))
                            .and_modify(|e| e.ends(t + Duration::minutes(30)))
                            .or_insert_with(|| EventBuilder::new(c).starts(t));
                        acc
                    },
                )
            })
            .zip(successors(Some(Weekday::Mon), |w| Some(w.succ())))
            .fold(Calendar::new(), |cal, (day, weekday)| {
                successors(
                    Some(match weekday
                        .number_from_monday()
                        .checked_sub(start_day.weekday().number_from_monday())
                    {
                        Some(x) => start_day + Duration::days(x.into()),
                        _ => {
                            start_day + Duration::weeks(1)
                                - Duration::days(
                                    start_day.weekday().num_days_from_monday() as i64
                                        - weekday.num_days_from_monday() as i64,
                                )
                        }
                    }),
                    |d| Some(*d + Duration::weeks(1)).filter(|nd| nd <= &end_day),
                )
                .flat_map(|date| {
                    day.values().map(move |event| {
                        Event::new()
                            .summary(event.summary)
                            .starts(NaiveDateTime::new(date, event.start.unwrap()))
                            .ends(NaiveDateTime::new(date, event.end.unwrap()))
                            .done()
                    })
                })
                .fold(cal, |mut cal, e| {
                    cal.push(e);
                    cal
                })
            })
    )
}
