use crate::{
    shifts::{ClassGroup, Shifts},
    util::{Class, ClassType, WeekDay, ALL_DAYS, WEEKDAYS},
};
use ansi_term::{
    Color::{self, *},
    Style,
};
use chrono::{NaiveTime, Timelike};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    sync::atomic::{AtomicU64, Ordering},
};
use unicode_width::UnicodeWidthStr;

const COLORS: [Color; 6] = [Red, Green, Yellow, Blue, Purple, Cyan];

#[derive(Clone)]
pub struct TimeTable([[TimeBlock; 24 * 2]; WEEKDAYS], HashMap<String, Color>);

impl Default for TimeTable {
    fn default() -> Self {
        Self(
            array_init::array_init(|_| array_init::array_init(|_| TimeBlock::default())),
            HashMap::default(),
        )
    }
}

impl TimeTable {
    pub fn add(
        mut self,
        class: &Class,
    ) -> Result<TimeTable, (WeekDay, [NaiveTime; 2], (String, String))> {
        static ID: AtomicU64 = AtomicU64::new(0);
        assert!(class.start.minute() == 0 || class.start.minute() == 30);
        assert!(class.end.minute() == 0 || class.end.minute() == 30);
        let start = time_to_index(class.start);
        let end = time_to_index(class.end);
        if let Some(TimeBlock::Filled(_, s, _)) = self.0[class.weekday as usize][start..end]
            .iter()
            .find(|b| **b != TimeBlock::Empty)
        {
            let e = (
                class.weekday,
                [TimeBlock::to_time(start), TimeBlock::to_time(end)],
                (s.to_string(), class.name.to_string()),
            );
            Err(e)
        } else {
            let id = ID.fetch_add(1, Ordering::Relaxed);
            self.0[class.weekday as usize][start..end]
                .iter_mut()
                .for_each(|b| *b = TimeBlock::Filled(class.kind, class.name.to_string(), id));
            if !self.1.contains_key(&class.name) {
                let n = self.1.len();
                self.1.insert(class.name.to_string(), COLORS[n]);
            }
            Ok(self)
        }
    }

    pub fn all_the_combos(schedule: &Shifts) -> Vec<Self> {
        let classes = schedule.class_set();
        let courses = classes.keys().map(|x| *x).collect::<Vec<(&str, bool)>>();
        fn gather<'a>(
            classes: &HashMap<(&'a str, bool), HashMap<i64, ClassGroup>>,
            timetable: TimeTable,
            course: &(&'a str, bool),
            other_courses: &[(&str, bool)],
        ) -> Vec<TimeTable> {
            classes[course]
                .values()
                .map(|class_block| {
                    class_block
                        .iter()
                        .try_fold(timetable.clone(), |acc, x| acc.add(x))
                })
                .filter_map(Result::ok)
                .map(|timetable| match other_courses.get(0) {
                    Some(next) => gather(classes, timetable.clone(), next, &other_courses[1..]),
                    None => vec![timetable],
                })
                .flatten()
                .collect()
        }
        courses
            .get(0)
            .map(|first| gather(&classes, TimeTable::default(), first, &courses[1..]))
            .unwrap_or_else(Vec::new)
    }

    pub fn starts_after(&self, time: NaiveTime) -> bool {
        let idx = time_to_index(time);
        dbg!(idx, time);
        self.0
            .iter()
            .all(|day| day[..idx].iter().all(|b| *b == TimeBlock::Empty))
    }

    pub fn ends_before(&self, time: NaiveTime) -> bool {
        let idx = time_to_index(time);
        self.0
            .iter()
            .all(|day| day[(idx + 1)..].iter().all(|b| *b == TimeBlock::Empty))
    }

    pub fn free_day(&self, d: WeekDay) -> bool {
        self.0[d as usize].iter().all(|b| *b == TimeBlock::Empty)
    }

    pub fn has_the_shift(&self, kind: ClassType, name: &str) -> bool {
        self.0
            .iter()
            .flat_map(|x| x.iter())
            .any(|x| *x == (kind, name))
    }

    pub fn hasnt_the_shift(&self, kind: ClassType, name: &str) -> bool {
        !self.has_the_shift(kind, name)
    }

    pub fn sum_work_hours(&self) -> isize {
        self.0
            .iter()
            .map(|day| {
                let first = day
                    .iter()
                    .enumerate()
                    .find(|(_, b)| **b != TimeBlock::Empty)
                    .map(|(i, _)| i);
                let last = day
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, b)| **b != TimeBlock::Empty)
                    .map(|(i, _)| i);
                if let (Some(f), Some(l)) = (first, last) {
                    l - f
                } else {
                    0
                }
            })
            .sum::<usize>() as isize
    }

    pub fn iter(&self) -> impl Iterator<Item = impl Iterator<Item = (usize, ClassType, &str)>> {
        self.0.iter().map(|day| {
            day.iter().enumerate().filter_map(|(i, x)| {
                if let TimeBlock::Filled(t, c, ..) = x {
                    Some((i, *t, c.as_str()))
                } else {
                    None
                }
            })
        })
    }
}

impl From<&Shifts> for Vec<TimeTable> {
    fn from(s: &Shifts) -> Self {
        TimeTable::all_the_combos(s)
    }
}

impl Display for TimeTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let max_width = self
            .0
            .iter()
            .flat_map(|x| x.iter().map(|s| s.width()))
            .max()
            .ok_or(fmt::Error)?;
        let mut first = 48;
        let mut last = 48;
        for i in 0..(24 * 2) {
            if self.0.iter().any(|x| x[i] != TimeBlock::Empty) {
                first = i;
                break;
            }
        }
        for i in (0..(24 * 2)).rev() {
            if self.0.iter().any(|x| x[i] != TimeBlock::Empty) {
                last = i + 1;
                break;
            }
        }
        for i in first..last {
            write!(f, "{} ", TimeBlock::to_time(i).format("%H:%M"))?;
            for day in &ALL_DAYS {
                self.0[*day as usize][i].display(&self.1, max_width, f)?
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum TimeBlock {
    Empty,
    Filled(ClassType, String, u64),
}

impl TimeBlock {
    fn to_time(u: usize) -> NaiveTime {
        index_to_time(u)
    }

    fn width(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Filled(_, s, _) => 2 + s.as_str().width(),
        }
    }

    fn display(
        &self,
        color_map: &HashMap<String, Color>,
        width: usize,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        match self {
            Self::Empty => (0..(width + 3)).try_for_each(|_| write!(f, " ")),
            Self::Filled(t, s, _) => {
                let style = match t {
                    ClassType::T(_) => Style::new().on(color_map[s]),
                    ClassType::L(_) => Style::new().fg(color_map[s]),
                };
                write!(f, "{}", style.paint(format!("{} {}", t, s)))?;
                (0..(width - s.width())).try_for_each(|_| write!(f, " "))
            }
        }
    }
}

impl Default for TimeBlock {
    fn default() -> Self {
        Self::Empty
    }
}

impl PartialEq<(ClassType, &'_ str)> for TimeBlock {
    fn eq(&self, other: &(ClassType, &'_ str)) -> bool {
        if let TimeBlock::Filled(t, n, _) = self {
            *t == other.0 && n == other.1
        } else {
            false
        }
    }
}

pub fn time_to_index(t: NaiveTime) -> usize {
    (t.hour() * 2 + t.minute() / 30) as usize
}

pub fn index_to_time(u: usize) -> NaiveTime {
    NaiveTime::from_hms((u / 2) as u32, (u % 2) as u32 * 30, 0)
}
