use chrono::NaiveTime;
use std::{
    convert::TryFrom,
    fmt::{self, Display},
    str::FromStr,
};

pub const WEEKDAYS: usize = 7;

#[derive(Debug, Clone, Copy)]
pub enum WeekDay {
    Mon = 0,
    Tue = 1,
    Wed = 2,
    Thu = 3,
    Fri = 4,
    Sat = 5,
    Sun = 6,
}

pub const ALL_DAYS: [WeekDay; WEEKDAYS] = [
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
pub enum ClassType {
    T(u32),
    L(u32),
}

impl ClassType {
    pub fn unique_id(self) -> i64 {
        match self {
            Self::T(n) => n as i64,
            Self::L(n) => (n as i64) * -1,
        }
    }
}

impl Display for ClassType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::T(n) => write!(f, "T{:x}", n),
            Self::L(n) => write!(f, "L{:x}", n),
        }
    }
}

impl FromStr for ClassType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s.starts_with("T") => Ok(Self::T(
                s[1..].parse().map_err(|_| "Invalid lab shift number")?,
            )),
            _ if s.starts_with("L") => Ok(Self::L(
                s[1..].parse().map_err(|_| "Invalid lab shift number")?,
            )),
            _ => Err("ClassType needs to be T or L"),
        }
    }
}

#[derive(Debug)]
pub struct Class {
    pub weekday: WeekDay,
    pub kind: ClassType,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub name: String,
}
