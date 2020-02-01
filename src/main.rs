mod error;
mod shifts;
mod timetable;
mod util;

use chrono::NaiveTime;
use shifts::Shifts;
use std::{
    env::args,
    fs::File,
    io::{self, stdin, BufRead, BufReader},
};
use timetable::TimeTable;
use util::{ClassType, WeekDay};

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
    let tts: Vec<TimeTable> = (&schedule).into();
    let mut input = String::new();
    let mut filters = TimetableFilters::default();
    loop {
        if let Ok(n) = stdin().read_line(&mut input) {
            if n < 1 {
                break;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct TimetableFilters {
    starts_after: Option<NaiveTime>,
    ends_before: Option<NaiveTime>,
    free_days: Vec<WeekDay>,
    has_the_shift: Vec<(ClassType, String)>,
}

impl TimetableFilters {
    pub fn starts_before(&mut self, time: NaiveTime) -> &mut Self {
        self.starts_after = Some(time);
        self
    }

    pub fn ends_after(&mut self, time: NaiveTime) -> &mut Self {
        self.ends_before = Some(time);
        self
    }

    pub fn free_day(&mut self, d: WeekDay) -> &mut Self {
        self.free_days.push(d);
        self
    }

    pub fn has_the_shift(&mut self, kind: ClassType, name: String) -> &mut Self {
        self.has_the_shift.push((kind, name));
        self
    }

    pub fn filter(&self, timetable: &TimeTable) -> bool {
        self.starts_after
            .map_or(false, |t| timetable.starts_after(t))
            && self.ends_before.map_or(false, |t| timetable.ends_before(t))
            && self.free_days.iter().all(|d| timetable.free_day(*d))
            && self
                .has_the_shift
                .iter()
                .all(|(s, t)| timetable.has_the_shift(*s, &t))
    }

    pub fn prompt(&mut self) -> &mut Self {
        print!(
            "Filters:
             1) Starts after
             2) Ends before
             3) Has free day
             4) Has a shift
             0) Cancel
             Pick one: "
        );
        let mut input = String::new();
        read_line(&mut input).unwrap();
        match input.trim().parse() {
            Ok(1) => {
                read_line(&mut input)
            },
            Ok(2) => (),
            Ok(3) => (),
            Ok(4) => (),
            Ok(0) => (),
            _ => println!("Invalid choice"),
        }
        self
    }
}

fn read_line(i: &mut String) -> io::Result<usize> {
    stdin().read_line(i)
}
