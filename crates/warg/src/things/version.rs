use std::str::FromStr;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ToString for Version {
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub enum VersionParseError {
    WrongNumberOfParts,
    PartsNotIntegers,
}

impl FromStr for Version {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(|c| c == '.').collect();
        if parts.len() != 3 {
            return Err(VersionParseError::WrongNumberOfParts);
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| VersionParseError::PartsNotIntegers)?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| VersionParseError::PartsNotIntegers)?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| VersionParseError::PartsNotIntegers)?;

        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}
