#[derive(Debug, PartialEq)]
pub enum KnownBoardNames {
    NucleoF4,
    MicrobitV2,
    Nrf52840dk,
    NucleoU545ReQ,
}

impl KnownBoardNames {
    pub fn from_str(name: &str) -> Option<Self> {
        match name {
            "nucleo-f4" => Some(Self::NucleoF4),
            "microbit-v2" => Some(Self::MicrobitV2),
            "nrf52840dk" => Some(Self::Nrf52840dk),
            "nucleo-u545re-q" => Some(Self::NucleoU545ReQ),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match &self {
            KnownBoardNames::NucleoF4 => "nucleo-f4",
            KnownBoardNames::MicrobitV2 => "microbit-v2",
            KnownBoardNames::Nrf52840dk => "nrf52840dk",
            KnownBoardNames::NucleoU545ReQ => "nucleo-u545re-q",
        }
    }
}

pub fn list_known_board_names() -> Vec<KnownBoardNames> {
    vec![
        KnownBoardNames::NucleoF4,
        KnownBoardNames::MicrobitV2,
        KnownBoardNames::Nrf52840dk,
        KnownBoardNames::NucleoU545ReQ,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_and_to_str_consistent() {
        for board in list_known_board_names() {
            assert_eq!(KnownBoardNames::from_str(board.to_str()).unwrap(), board);
        }
    }

    #[test]
    fn list_known_boards_updated() {
        let backup_list = vec![
            KnownBoardNames::NucleoF4,
            KnownBoardNames::MicrobitV2,
            KnownBoardNames::Nrf52840dk,
            KnownBoardNames::NucleoU545ReQ,
        ];

        assert_eq!(list_known_board_names(), backup_list, "If this fails it means that you likely forgot to update `list_known_boards`, and subsequently the `list_known_boards_updated` test");
    }
}
