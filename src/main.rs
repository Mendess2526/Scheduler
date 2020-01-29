use chrono::NaiveTime;
use std::{
    borrow::Cow,
    collections::HashMap,
    env::args,
    fs::File,
    io::{self, BufRead, BufReader},
    ops::Index,
    ops::IndexMut,
    str::FromStr,
};

const TIME_FORMAT: &str = "%Hh%M";

enum WeekDay {
    Mon = 0,
    Tue = 1,
    Wed = 2,
    Thu = 3,
    Fri = 4,
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

#[derive(Debug, Clone, Copy)]
enum ClassType {
    T,
    L(usize),
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
    kind: ClassType,
    start: NaiveTime,
    end: NaiveTime,
    name: String,
}

type Day = HashMap<(NaiveTime, NaiveTime), Vec<Class>>;

#[derive(Default, Debug)]
struct Schedule {
    table: [Day; 5],
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

fn main() -> io::Result<()> {
    BufReader::new(File::open(
        args().nth(1).expect("No schedule file provided"),
    )?)
    .lines()
    .filter_map(Result::ok)
    .enumerate()
    .map(|(i, l)| -> Result<(WeekDay, Class), ErrMessage> {
        // CPD:L13:08h00:09h30:Mon
        // 0  :1  :2    :3    :4
        let fields: Vec<&str> = l.split(':').collect::<Vec<&str>>();
        let kind = match fields[1].parse() {
            Ok(k) => k,
            Err(e) => return Err(ErrMessage::new(e, i, l)),
        };
        let start = match NaiveTime::parse_from_str(fields[2], TIME_FORMAT) {
            Ok(s) => s,
            Err(e) => return Err(ErrMessage::new("Invalid start time", i, l)),
        };
        let end = match NaiveTime::parse_from_str(fields[3], TIME_FORMAT) {
            Ok(e) => e,
            Err(e) => return Err(ErrMessage::new("Invalid end time", i, l)),
        };
        Ok((
            fields[4]
                .parse()
                .map_err(|_| ErrMessage::new("Invalid time", i, l))?,
            Class {
                kind,
                start,
                end,
                name: fields[0].to_string(),
            },
        ))
    })
    .try_fold(Schedule::default(), |acc, s| {
        s.map(|(day, c)| acc[day].entry((c.start, c.end)).or_default().push(c));
        acc
    });
    Ok(())
}
