mod error;
mod ical;
mod shifts;
mod timetable;
mod util;

use chrono::{format::ParseResult, NaiveDate, NaiveTime};
use dialoguer::{Checkboxes, Input};
use enum_iterator::IntoEnumIterator;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer};
use shifts::Shifts;
use std::{
    collections::HashSet,
    env::args,
    error::Error,
    fmt::{self, Display},
    fs::File,
    io::{self, BufRead, BufReader},
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
    let mut feedback = String::new();
    loop {
        tts.retain(|t| filters.filter(t));
        let amount = tts.len();
        for t in &tts {
            println!("{}", t);
        }
        println!("Number of possible timetables: {}", amount);
        if !feedback.is_empty() {
            println!("{}", feedback);
        }
        if let Some(f) = filters.prompt(&mut feedback, tts.iter()) {
            filters = f;
        } else {
            break Ok(());
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, IntoEnumIterator)]
pub enum SubMenus {
    StartsAfter,
    EndsBefore,
    HasFreeDay,
    HasAShift,
    HasntAShift,
    SaveFilters,
    LoadFilters,
    ExportToIcal,
    Close,
}

impl Display for SubMenus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use SubMenus::*;
        let s = match self {
            StartsAfter => "Starts after",
            EndsBefore => "Ends before",
            HasFreeDay => "Has free day",
            HasAShift => "Has a shift",
            HasntAShift => "Hasn't a shift",
            SaveFilters => "Save filters",
            LoadFilters => "Load filters",
            ExportToIcal => "Export as iCal",
            Close => "Close",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
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

    pub fn prompt<'a>(
        mut self,
        feedback: &mut String,
        timetables: impl Iterator<Item = &'a TimeTable>,
    ) -> Option<Self> {
        feedback.clear();
        let submenus = SubMenus::into_enum_iter().collect::<Vec<_>>();
        let pick = Input::new()
            .with_prompt(&format!(
                "Filters:\n{}\nPick one",
                submenus
                    .iter()
                    .enumerate()
                    .format_with("\n", |(i, p), f| f(&format_args!("{}) {}", i, p)))
            ))
            .interact();
        match pick.map(|i: usize| submenus[i]) {
            Ok(SubMenus::StartsAfter) => {
                let input = Input::<String>::new()
                    .with_prompt("Time")
                    .allow_empty(true)
                    .interact()
                    .unwrap();
                if let Ok(t) = parse_time(&input) {
                    self.starts_after = Some(t)
                } else {
                    self.starts_after = None;
                    feedback.push_str("Cleared")
                }
            }
            Ok(SubMenus::EndsBefore) => {
                let input = Input::<String>::new()
                    .with_prompt("Time")
                    .allow_empty(true)
                    .interact()
                    .unwrap();
                if let Ok(t) = parse_time(&input) {
                    self.ends_before = Some(t)
                } else {
                    self.ends_before = None;
                    feedback.push_str("Cleared")
                }
            }
            Ok(SubMenus::HasFreeDay) => {
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
            Ok(SubMenus::HasAShift) => {
                let k = Input::new()
                    .with_prompt("Shift [T{{number}}/L{{number}}]")
                    .interact();
                let name = Input::new().with_prompt("Course").interact().unwrap();
                if let Ok(k) = k {
                    self.has_the_shift.push((k, name));
                } else {
                    self.has_the_shift.clear();
                    feedback.push_str("Cleared, press enter");
                }
            }
            Ok(SubMenus::HasntAShift) => {
                let k = Input::new()
                    .with_prompt("Shift [T{{number}}/L{{number}}]")
                    .interact();
                let name = Input::new().with_prompt("Course").interact().unwrap();
                if let Ok(k) = k {
                    self.hasnt_the_shift.push((k, name));
                } else {
                    self.hasnt_the_shift.clear();
                    feedback.push_str("Cleared, press enter");
                }
            }
            Ok(SubMenus::SaveFilters) => {
                let k = Input::<String>::new()
                    .with_prompt("Filename")
                    .interact()
                    .map_err(|e| Box::new(e) as Box<dyn Error>)
                    .and_then(|f| File::create(f).map_err(|e| Box::new(e) as Box<dyn Error>))
                    .and_then(|f| to_writer(f, &self).map_err(|e| Box::new(e) as Box<dyn Error>));
                match k {
                    Ok(_) => feedback.push_str("Saved!"),
                    Err(e) => feedback.push_str(&format!("Error saving filters: {}", e)),
                }
            }
            Ok(SubMenus::LoadFilters) => {
                let k = Input::<String>::new()
                    .with_prompt("Filename")
                    .interact()
                    .map_err(|e| Box::new(e) as Box<dyn Error>)
                    .and_then(|f| File::open(f).map_err(|e| Box::new(e) as Box<dyn Error>))
                    .and_then(|f| from_reader(f).map_err(|e| Box::new(e) as Box<dyn Error>));
                match k {
                    Ok(f) => {
                        feedback.push_str("Loaded!");
                        self = f
                    }
                    Err(e) => feedback.push_str(&format!("Error saving filters: {}", e)),
                }
            }
            Ok(SubMenus::Close) => return None,
            Ok(SubMenus::ExportToIcal) => {
                let k = || -> Result<(), Box<dyn Error>> {
                    let mut file = Input::<String>::new()
                        .with_prompt("Filename")
                        .interact()
                        .map_err(|e| Box::new(e) as Box<dyn Error>)
                        .and_then(|f| File::create(f).map_err(|e| Box::new(e) as Box<dyn Error>))?;
                    let time_table = match timetables.exactly_one() {
                        Ok(t) => t,
                        Err(_) => return Err("Either too many timetables or too few".into()),
                    };
                    let start_date = Input::<NaiveDate>::new()
                        .with_prompt("Start date: YY-MM-DD")
                        .interact()
                        .map_err(|e| Box::new(e) as Box<dyn Error>)?;
                    let end_date = Input::<NaiveDate>::new()
                        .with_prompt("End date: YY-MM-DD")
                        .interact()
                        .map_err(|e| Box::new(e) as Box<dyn Error>)?;
                    ical::write_cal(&mut file, time_table, start_date, end_date)
                        .map_err(|e| Box::new(e) as Box<dyn Error>)
                }();
                match k {
                    Ok(_) => feedback.push_str("Saved!"),
                    Err(e) => feedback.push_str(&format!("Error exporting to iCal: {}", e)),
                }
            }
            Err(_) => feedback.push_str("Invalid choice"),
        }
        Some(self)
    }
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
