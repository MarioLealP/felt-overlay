#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Show,
    Hide,
    Toggle,
    Status,
    Quit,
}

impl Command {
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("show") {
            Some(Command::Show)
        } else if trimmed.eq_ignore_ascii_case("hide") {
            Some(Command::Hide)
        } else if trimmed.eq_ignore_ascii_case("toggle") {
            Some(Command::Toggle)
        } else if trimmed.eq_ignore_ascii_case("status") {
            Some(Command::Status)
        } else if trimmed.eq_ignore_ascii_case("quit") || trimmed.eq_ignore_ascii_case("exit") {
            Some(Command::Quit)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_commands() {
        assert_eq!(Command::parse("show"), Some(Command::Show));
        assert_eq!(Command::parse("SHOW\n"), Some(Command::Show));
        assert_eq!(Command::parse("  show  \r\n"), Some(Command::Show));

        assert_eq!(Command::parse("hide"), Some(Command::Hide));
        assert_eq!(Command::parse("HIDE\n"), Some(Command::Hide));
        assert_eq!(Command::parse("\thide \n"), Some(Command::Hide));

        assert_eq!(Command::parse("toggle"), Some(Command::Toggle));
        assert_eq!(Command::parse("TOGGLE\n"), Some(Command::Toggle));

        assert_eq!(Command::parse("status"), Some(Command::Status));
        assert_eq!(Command::parse("STATUS\n"), Some(Command::Status));

        assert_eq!(Command::parse("quit"), Some(Command::Quit));
        assert_eq!(Command::parse("exit\n"), Some(Command::Quit));

        assert_eq!(Command::parse("unknown"), None);
        assert_eq!(Command::parse(""), None);
        assert_eq!(Command::parse("   \n"), None);
    }
}
