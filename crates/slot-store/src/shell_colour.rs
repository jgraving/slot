/// The four painted RG SP shells, with an automatic dark/light default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShellColour {
    #[default]
    Theme,
    Graphite,
    Platinum,
    PearlBlue,
    PearlPink,
}

impl ShellColour {
    pub const ALL: [Self; 5] = [Self::Theme, Self::Graphite, Self::Platinum, Self::PearlBlue, Self::PearlPink];
    pub const FINISHES: [Self; 4] = [Self::Graphite, Self::Platinum, Self::PearlBlue, Self::PearlPink];

    pub fn key(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Graphite => "graphite",
            Self::Platinum => "platinum",
            Self::PearlBlue => "pearl_blue",
            Self::PearlPink => "pearl_pink",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == value)
    }

    pub fn step(self, right: bool) -> Self {
        let at = Self::ALL.iter().position(|c| *c == self).unwrap_or(0);
        Self::ALL[(at + if right { 1 } else { Self::ALL.len() - 1 }) % Self::ALL.len()]
    }

    pub fn resolved(self, light: bool) -> Self {
        if self == Self::Theme {
            if light { Self::Platinum } else { Self::Graphite }
        } else {
            self
        }
    }

    pub fn finish_index(self) -> usize {
        Self::FINISHES.iter().position(|c| *c == self).unwrap_or(0)
    }
}
