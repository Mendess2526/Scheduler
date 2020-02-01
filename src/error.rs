use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display},
};

#[derive(Debug)]
pub struct ErrMessage {
    msg: Cow<'static, str>,
    line_no: usize,
    line: String,
}

impl ErrMessage {
    pub fn new<S: Into<Cow<'static, str>>>(s: S, line_no: usize, line: String) -> Self {
        Self {
            msg: s.into(),
            line_no,
            line,
        }
    }
}

impl Display for ErrMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Parse error in line {}", self.line_no)?;
        writeln!(f, "Line {}", self.line)?;
        writeln!(f, "Note: {}", self.msg)
    }
}

impl Error for ErrMessage {}
