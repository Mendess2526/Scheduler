use crate::{
    error::ErrMessage,
    util::{Class, ClassType, WeekDay, WEEKDAYS},
};
use chrono::NaiveTime;
use std::{
    collections::{HashMap, HashSet},
};

const TIME_FORMAT: &str = "%Hh%M";

type Day = HashMap<(NaiveTime, NaiveTime), Vec<Class>>;

pub type ClassGroup<'a> = Vec<&'a Class>;

#[derive(Default, Debug)]
pub struct Shifts {
    table: [Day; WEEKDAYS],
    classes: HashMap<String, HashSet<ClassType>>,
}

impl Shifts {
    pub fn parse_schedule<'a, L: Iterator<Item = String>>(l: L) -> Result<Self, ErrMessage> {
        l.enumerate()
            .map(|(i, l)| -> Result<(WeekDay, Class), ErrMessage> {
                // CPD:L13:08h00:09h30:Mon
                // 0  :1  :2    :3    :4
                let fields = l.split(':').map(|x| x.trim()).collect::<Vec<_>>();
                let kind = match fields[1].parse() {
                    Ok(k) => k,
                    Err(e) => return Err(ErrMessage::new(e, i, l)),
                };
                let start = match NaiveTime::parse_from_str(fields[2], TIME_FORMAT) {
                    Ok(s) => s,
                    Err(_) => return Err(ErrMessage::new("Invalid start time", i, l)),
                };
                let end = match NaiveTime::parse_from_str(fields[3], TIME_FORMAT) {
                    Ok(e) => e,
                    Err(_) => return Err(ErrMessage::new("Invalid end time", i, l)),
                };
                let weekday = match fields[4].parse() {
                    Ok(w) => w,
                    Err(_) => return Err(ErrMessage::new("Invalid time", i, l)),
                };
                Ok((
                    weekday,
                    Class {
                        weekday,
                        kind,
                        start,
                        end,
                        name: fields[0].to_string(),
                    },
                ))
            })
            .try_fold(Self::default(), |mut acc, s| -> Result<Self, ErrMessage> {
                let (day, c) = s?;
                acc.classes
                    .entry(c.name.clone())
                    .or_default()
                    .insert(c.kind);
                acc.table[day as usize]
                    .entry((c.start, c.end))
                    .or_default()
                    .push(c);
                Ok(acc)
            })
    }

    pub fn class_set(&self) -> HashMap<(&str, bool), HashMap<i64, ClassGroup>> {
        self.table
            .iter()
            .flat_map(|x| x.values())
            .flatten()
            .fold(HashMap::new(), |mut acc, c| {
                let n = c.kind.unique_id();
                acc.entry((&c.name, n > 0))
                    .or_default()
                    .entry(n)
                    .or_default()
                    .push(c);
                acc
            })
    }
}
