use chrono::{NaiveTime, Timelike};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    convert::TryFrom,
    env::args,
    fmt::{self, Display},
    fs::File,
    io::{self, BufRead, BufReader},
    ops::Index,
    ops::IndexMut,
    str::FromStr,
};
use unicode_width::UnicodeWidthStr;

const TIME_FORMAT: &str = "%Hh%M";
const WEEKDAYS: usize = 7;

#[derive(Debug, Clone, Copy)]
enum WeekDay {
    Mon = 0,
    Tue = 1,
    Wed = 2,
    Thu = 3,
    Fri = 4,
    Sat = 5,
    Sun = 6,
}

const ALL_DAYS: [WeekDay; WEEKDAYS] = [
    WeekDay::Mon,
    WeekDay::Tue,
    WeekDay::Wed,
    WeekDay::Thu,
    WeekDay::Fri,
    WeekDay::Sat,
    WeekDay::Sun,
];

impl TryFrom<u8> for WeekDay {
    type Error = u8;
    fn try_from(u: u8) -> Result<Self, Self::Error> {
        match u {
            0 => Ok(Self::Mon),
            1 => Ok(Self::Tue),
            2 => Ok(Self::Wed),
            3 => Ok(Self::Thu),
            4 => Ok(Self::Fri),
            5 => Ok(Self::Sat),
            6 => Ok(Self::Sun),
            _ => Err(u),
        }
    }
}

impl FromStr for WeekDay {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if "Mon".eq_ignore_ascii_case(s) => Ok(Self::Mon),
            _ if "Tue".eq_ignore_ascii_case(s) => Ok(Self::Tue),
            _ if "Wed".eq_ignore_ascii_case(s) => Ok(Self::Wed),
            _ if "Thu".eq_ignore_ascii_case(s) => Ok(Self::Thu),
            _ if "Fri".eq_ignore_ascii_case(s) => Ok(Self::Fri),
            _ => Err("Invalid week day"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ClassType {
    T,
    L(usize),
}

impl Display for ClassType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::T => write!(f, "T"),
            Self::L(_) => write!(f, "L"),
        }
    }
}

impl FromStr for ClassType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s.starts_with("T") => Ok(Self::T),
            _ if s.starts_with("L") => Ok(Self::L(
                s[1..].parse().map_err(|_| "Invalid lab shift number")?,
            )),
            _ => Err("ClassType needs to be T or L"),
        }
    }
}

#[derive(Debug)]
struct ErrMessage {
    msg: Cow<'static, str>,
    line_no: usize,
    line: String,
}

impl ErrMessage {
    fn new<S: Into<Cow<'static, str>>>(s: S, line_no: usize, line: String) -> Self {
        Self {
            msg: s.into(),
            line_no,
            line,
        }
    }
}

#[derive(Debug)]
struct Class {
    weekday: WeekDay,
    kind: ClassType,
    start: NaiveTime,
    end: NaiveTime,
    name: String,
}

type Day = HashMap<(NaiveTime, NaiveTime), Vec<Class>>;

#[derive(Default, Debug)]
struct Schedule {
    table: [Day; WEEKDAYS],
    classes: HashMap<String, HashSet<ClassType>>,
}

impl Schedule {
    fn parse_schedule<'a, L: Iterator<Item = String>>(l: L) -> Result<Self, ErrMessage> {
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
            .try_fold(
                Schedule::default(),
                |mut acc, s| -> Result<Schedule, ErrMessage> {
                    let (day, c) = s?;
                    acc.classes
                        .entry(c.name.clone())
                        .or_default()
                        .insert(c.kind);
                    acc[day].entry((c.start, c.end)).or_default().push(c);
                    Ok(acc)
                },
            )
    }

    fn all_the_combos(&self) -> Result<Vec<TimeTable>, Cow<'static, str>> {
        let (lab_classes, timetable) = self
            .table
            .iter()
            .flat_map(|x| x.values())
            .flatten()
            .try_fold(
                (HashMap::<&str, Vec<&Class>>::new(), TimeTable::default()),
                |mut acc, c| match c.kind {
                    ClassType::T => {
                        let (hm, tt) = acc;
                        tt.add(c)
                            .map_err(|[first, second]| {
                                format!("Incompatilbe T classes.\n  {:?}\n  {:?}", first, second)
                            })
                            .map(|tt| (hm, tt))
                    }
                    ClassType::L(_) => {
                        acc.0.entry(&c.name).or_default().push(c);
                        Ok(acc)
                    }
                },
            )?;
        let courses = lab_classes.keys().map(|s| *s).collect::<Vec<_>>();
        fn gather(
            lab_classes: &HashMap<&str, Vec<&Class>>,
            timetable: TimeTable,
            course: &str,
            other_courses: &[&str],
        ) -> Vec<TimeTable> {
            lab_classes[course]
                .iter()
                .map(|class| {
                    timetable.clone().add(class)
                })
                .filter_map(Result::ok)
                .map(|timetable| match other_courses.get(0) {
                    Some(next) => gather(lab_classes, timetable.clone(), next, &other_courses[1..]),
                    None => vec![timetable],
                })
                .flatten()
                .collect()
        }
        Ok(courses
            .get(0)
            .map(|first| gather(&lab_classes, timetable, first, &courses[1..]))
            .unwrap_or_else(Vec::new))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum TimeBlock {
    Empty,
    Filled(ClassType, String),
}

impl TimeBlock {
    fn to_time(u: usize) -> NaiveTime {
        NaiveTime::from_hms((u / 2) as u32, (u % 2) as u32 * 30, 0)
    }

    fn width(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Filled(_, s) => 2 + s.as_str().width(),
        }
    }

    fn display(&self, width: usize, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Empty => (0..(width + 2)).try_for_each(|_| write!(f, " ")),
            Self::Filled(t, s) => {
                write!(f, "{} {}", t, s)?;
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

#[derive(Clone)]
struct TimeTable([[TimeBlock; 24 * 2]; WEEKDAYS]);

impl Default for TimeTable {
    fn default() -> Self {
        Self(array_init::array_init(|_| {
            array_init::array_init(|_| TimeBlock::default())
        }))
    }
}

impl TimeTable {
    fn add(mut self, class: &Class) -> Result<TimeTable, [(WeekDay, [NaiveTime; 2], String); 2]> {
        assert!(class.start.minute() == 0 || class.start.minute() == 30);
        assert!(class.end.minute() == 0 || class.end.minute() == 30);
        let start = (class.start.hour() * 2 + class.start.minute() / 30) as usize;
        let end = (class.end.hour() * 2 + class.end.minute() / 30) as usize;
        if let Some(TimeBlock::Filled(_, s)) = self.0[class.weekday as usize][start..end]
            .iter()
            .find(|b| **b != TimeBlock::Empty)
        {
            let e = [
                (
                    class.weekday,
                    [TimeBlock::to_time(start), TimeBlock::to_time(end)],
                    s.to_string(),
                ),
                (
                    class.weekday,
                    [TimeBlock::to_time(start), TimeBlock::to_time(end)],
                    class.name.to_string(),
                ),
            ];
            Err(e)
        } else {
            // dbg!([start, end], class);
            self.0[class.weekday as usize][start..end]
                .iter_mut()
                .for_each(|b| *b = TimeBlock::Filled(class.kind, class.name.to_string()));
            Ok(self)
        }
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
        for i in 0..(24 * 2) {
            if self.0.iter().any(|x| x[i] != TimeBlock::Empty) {
                write!(f, "{} ", TimeBlock::to_time(i))?;
                for day in &ALL_DAYS {
                    self.0[*day as usize][i].display(max_width, f)?
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let schedule = match Schedule::parse_schedule(
        BufReader::new(File::open(
            args().nth(1).expect("No schedule file provided"),
        )?)
        .lines()
        .filter_map(Result::ok),
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Parse error in line {}", e.line_no);
            eprintln!("Line {}", e.line);
            eprintln!("Note: {}", e.msg);
            return Err(io::Error::from(io::ErrorKind::Other));
        }
    };
    match schedule.all_the_combos() {
        Ok(tts) => {
            println!("Possible timetables: {}", tts.len());
            tts.iter().for_each(|t| println!("{}", t));
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(io::Error::from(io::ErrorKind::Other));
        }
    }
    // for day in 0..schedule.table.len() {
    //     let day: WeekDay = (day as u8).try_into().expect("To many days in a week");
    //     println!("Day: {:?}", day);
    //     for time_slice in schedule[day].keys() {
    //         let (start, end) = time_slice;
    //         println!("  {},{}", start, end);
    //         for class in schedule[day][time_slice].iter() {
    //             println!("    {:?}", class);
    //         }
    //     }
    // }
    Ok(())
}

impl Index<WeekDay> for Schedule {
    type Output = Day;
    fn index(&self, i: WeekDay) -> &Self::Output {
        &self.table[i as usize]
    }
}

impl IndexMut<WeekDay> for Schedule {
    fn index_mut(&mut self, i: WeekDay) -> &mut Self::Output {
        &mut self.table[i as usize]
    }
}
