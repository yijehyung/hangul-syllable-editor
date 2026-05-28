use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum HangulComponent {
    Cho,
    Jung,
    Jong,
}

impl HangulComponent {
    pub fn kind(self) -> u8 {
        match self {
            HangulComponent::Cho => 0,
            HangulComponent::Jung => 1,
            HangulComponent::Jong => 2,
        }
    }

    pub fn from_kind(k: u8) -> Option<Self> {
        match k {
            0 => Some(Self::Cho),
            1 => Some(Self::Jung),
            2 => Some(Self::Jong),
            _ => None,
        }
    }
}

pub fn kind_to_name(kind: HangulComponent) -> &'static str {
    match kind {
        HangulComponent::Cho => "초성",
        HangulComponent::Jung => "중성",
        HangulComponent::Jong => "종성",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_values() {
        assert_eq!(HangulComponent::Cho.kind(), 0);
        assert_eq!(HangulComponent::Jung.kind(), 1);
        assert_eq!(HangulComponent::Jong.kind(), 2);
    }

    #[test]
    fn kind_to_name_all() {
        assert_eq!(kind_to_name(HangulComponent::Cho), "초성");
        assert_eq!(kind_to_name(HangulComponent::Jung), "중성");
        assert_eq!(kind_to_name(HangulComponent::Jong), "종성");
    }

    #[test]
    fn from_kind_0_is_cho() {
        assert_eq!(HangulComponent::from_kind(0), Some(HangulComponent::Cho));
    }

    #[test]
    fn from_kind_1_is_jung() {
        assert_eq!(HangulComponent::from_kind(1), Some(HangulComponent::Jung));
    }

    #[test]
    fn from_kind_2_is_jong() {
        assert_eq!(HangulComponent::from_kind(2), Some(HangulComponent::Jong));
    }

    #[test]
    fn from_kind_invalid_returns_none() {
        assert_eq!(HangulComponent::from_kind(255), None);
    }
}
