use std::fmt;

use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    Criminal = 0,
    Innocent = 1,
}

impl Answer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Criminal => "criminal",
            Self::Innocent => "innocent",
        }
    }

    pub const fn encoded(self) -> u8 {
        self as u8
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::Criminal => Self::Innocent,
            Self::Innocent => Self::Criminal,
        }
    }

    pub const fn from_encoded(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Criminal),
            1 => Some(Self::Innocent),
            _ => None,
        }
    }
}

impl fmt::Display for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub type Name = String;

pub const NAMES: [&str; 87] = [
    "Ajay", "Akhil", "Alex", "Albert", "Anand", "BK", "Blake", "Brian", "Clara", "Cole",
    "Coriander", "Cyrus", "Daphne", "Dasith", "Dean", "Diego", "Eddy", "Eduardo", "Elias",
    "Eva", "Felix", "Finn", "Fiona", "Gopal", "Gia", "Grace", "Hana", "Hazel", "Hugo", "Iris",
    "Isaac", "Ivy", "Jake", "Jasper", "June", "Kai", "Kevin", "Knox", "Laika", "Lakshmi",
    "Leo", "Lydia", "Maya", "Milo", "Mira", "Neal", "Noah", "Nora", "Omar", "Opal", "Owen",
    "Parker",
    "Pavan", "Piper", "Priya", "Quentin", "Quinn", "Quincy", "Rafael", "Rowan", "Ruby", "Scott",
    "Sid", "Silas", "Sloane", "Tessa", "Theo", "Thrisha", "Tristan", "Uma", "Uri", "Ursula",
    "Venkat",
    "Victor", "Violet", "Willow", "Wren", "Wyatt", "Xander", "Xia", "Ximena", "Yara", "Yasmin",
    "Yuri", "Zane", "Zara", "Zoe",
];

pub type Role = String;

#[rustfmt::skip]
pub const ROLES: [&str; 18] = [
    "Artist", "Baker", "Builder", "Cook", "Detective", "Doctor", "Farmer", "Firefighter",
    "Guard", "Judge", "Mechanic", "Nurse", "Pilot", "Police Officer", "Scientist", "Singer",
    "Teacher", "Technologist",
];
