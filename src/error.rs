use std::fmt::{Display, Formatter};

use anyhow::Error;

#[derive(Debug)]
pub enum Status {
    NotFound(Error),
    InvalidArgument(Error),
    Internal(Error),
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::NotFound(error) => write!(f, "NotFound: {}", error),
            Status::InvalidArgument(error) => write!(f, "InvalidArgument: {}", error),
            Status::Internal(error) => write!(f, "Internal: {}", error),
        }
    }
}
