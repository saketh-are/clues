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

pub const NAMES: [&str; 78] = [
    "Alex", "Albert", "Anand", "Ben", "Bianca", "Blake", "Clara", "Cole", "Cyrus", "Daphne",
    "Dean", "Diego", "Eden", "Elias", "Eva", "Felix", "Finn", "Fiona", "Gavin", "Gia", "Grace",
    "Hana", "Hazel", "Hugo", "Iris", "Isaac", "Ivy", "Jade", "Jasper", "June", "Kai", "Kira",
    "Knox", "Leo", "Luna", "Lydia", "Maya", "Milo", "Mira", "Nadia", "Noah", "Nora", "Omar",
    "Opal", "Owen", "Parker", "Piper", "Priya", "Quentin", "Quinn", "Quincy", "Rafael", "Rowan",
    "Ruby", "Sadie", "Silas", "Sloane", "Tessa", "Theo", "Tristan", "Uma", "Uri", "Ursula",
    "Vera", "Victor", "Violet", "Willow", "Wren", "Wyatt", "Xander", "Xia", "Ximena", "Yara",
    "Yasmin", "Yuri", "Zane", "Zara", "Zoe",
];

pub type Role = String;

#[rustfmt::skip]
pub const ROLES: [&str; 24] = [
    "Artist", "Baker", "Botanist", "Chef", "Coach", "Coder", "Curator", "Dancer", "Designer",
    "Doctor", "Driver", "Florist", "Gardener", "Journalist", "Librarian", "Musician", "Nurse",
    "Painter", "Pilot", "Poet", "Ranger", "Sailor", "Sculptor", "Teacher",
];
