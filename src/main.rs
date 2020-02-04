mod error;
mod shifts;
mod timetable;
mod util;

use chrono::{format::ParseResult, NaiveTime};
use dialoguer::{Checkboxes, Input};
use shifts::Shifts;
use std::{
    collections::HashSet,
    env::args,
    fs::File,
    io::{self, stdin, BufRead, BufReader},
};
use timetable::TimeTable;
use util::{ClassType, WeekDay, ALL_DAYS};

fn main() -> io::Result<()> {
    let schedule = match Shifts::parse_schedule(
        BufReader::new(File::open(
            args().nth(1).expect("No schedule file provided"),
        )?)
        .lines()
        .filter_map(Result::ok)
        .filter(|x| !x.is_empty()),
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            return Err(io::Error::from(io::ErrorKind::Other));
        }
    };
    let mut tts: Vec<TimeTable> = (&schedule).into();
    tts.sort_by_cached_key(|t| -t.sum_work_hours());
    let mut filters = TimetableFilters::default();
    loop {
        let mut amount = 0;
        for (i, t) in tts.iter().filter(|t| filters.filter(t)).enumerate() {
            println!("{}", t);
            amount = i + 1;
        }
        println!("Number of possible timetables: {}", amount);
        filters.prompt();
    }
}

#[derive(Debug, Default)]
pub struct TimetableFilters {
    starts_after: Option<NaiveTime>,
    ends_before: Option<NaiveTime>,
    free_days: HashSet<WeekDay>,
    has_the_shift: Vec<(ClassType, String)>,
    hasnt_the_shift: Vec<(ClassType, String)>,
}

impl TimetableFilters {
    pub fn filter(&self, timetable: &TimeTable) -> bool {
        self.starts_after
            .map_or(true, |t| timetable.starts_after(t))
            && self.ends_before.map_or(true, |t| timetable.ends_before(t))
            && self.free_days.iter().all(|d| timetable.free_day(*d))
            && self
                .has_the_shift
                .iter()
                .all(|(s, t)| timetable.has_the_shift(*s, &t))
            && self
                .hasnt_the_shift
                .iter()
                .all(|(s, t)| timetable.hasnt_the_shift(*s, &t))
    }

    pub fn prompt(&mut self) -> &mut Self {
        let pick = Input::new()
            .with_prompt(
                "Filters:
1) Starts after
2) Ends before
3) Has free day
4) Has a shift
5) Hasn't a shift
0) Cancel
Pick one",
            )
            .interact();
        match pick {
            Ok(1) => {
                let input = Input::<String>::new()
                    .with_prompt("Time")
                    .allow_empty(true)
                    .interact()
                    .unwrap();
                if let Ok(t) = parse_time(&input) {
                    self.starts_after = Some(t)
                } else {
                    self.starts_after = None;
                    println!("Cleared")
                }
            }
            Ok(2) => {
                let input = Input::<String>::new()
                    .with_prompt("Time")
                    .allow_empty(true)
                    .interact()
                    .unwrap();
                if let Ok(t) = parse_time(&input) {
                    self.ends_before = Some(t)
                } else {
                    self.ends_before = None;
                    println!("Cleared")
                }
            }
            Ok(3) => {
                let checked = ALL_DAYS
                    .iter()
                    .map(|d| (d, self.free_days.contains(d)))
                    .collect::<Vec<_>>();
                let selection = Checkboxes::new()
                    .items_checked(&checked)
                    .interact()
                    .unwrap();
                self.free_days.clear();
                for d in selection {
                    self.free_days.insert(*checked[d].0);
                }
            }
            Ok(4) => {
                let k = Input::new()
                    .with_prompt("Shift [T{{number}}/L{{number}}]")
                    .interact();
                let name = Input::new().with_prompt("Course").interact().unwrap();
                if let Ok(k) = k {
                    self.has_the_shift.push((k, name));
                } else {
                    self.has_the_shift.clear();
                    println!("Cleared, press enter");
                    read_line(&mut String::new());
                }
            }
            Ok(5) => {
                let k = Input::new()
                    .with_prompt("Shift [T{{number}}/L{{number}}]")
                    .interact();
                let name = Input::new().with_prompt("Course").interact().unwrap();
                if let Ok(k) = k {
                    self.hasnt_the_shift.push((k, name));
                } else {
                    self.hasnt_the_shift.clear();
                    println!("Cleared, press enter");
                    read_line(&mut String::new());
                }
            }
            Ok(0) => (),
            _ => println!("Invalid choice"),
        }
        self
    }
}

fn read_line(i: &mut String) -> usize {
    i.clear();
    stdin().read_line(i).unwrap()
}

fn parse_time(s: &str) -> ParseResult<NaiveTime> {
    match s {
        _ if s.len() < 3 => Ok(NaiveTime::from_hms(
            s.parse()
                .map_err(|_| NaiveTime::parse_from_str(s, "%H:%M").unwrap_err())?,
            0,
            0,
        )),
        _ if s.contains("h") => NaiveTime::parse_from_str(s, "%Hh%M"),
        _ if s.contains(":") => NaiveTime::parse_from_str(s, "%H:%M")
            .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S")),
        _ => NaiveTime::parse_from_str(s, "%H:%M"),
    }
}
