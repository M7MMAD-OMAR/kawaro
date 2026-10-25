//! Interface language. Arabic when the user's locale is Arabic, English otherwise.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lang {
    Ar,
    #[default]
    En,
}

impl Lang {
    /// From `KAWARO_LANG`, then `LC_ALL`, `LC_MESSAGES`, `LANGUAGE`, `LANG`.
    pub fn detect() -> Self {
        for var in ["KAWARO_LANG", "LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"] {
            if let Ok(v) = std::env::var(var)
                && !v.is_empty()
            {
                return Self::parse(&v);
            }
        }
        Lang::En
    }

    pub fn parse(s: &str) -> Self {
        if s.trim().to_ascii_lowercase().starts_with("ar") {
            Lang::Ar
        } else {
            Lang::En
        }
    }

    pub fn rtl(self) -> bool {
        self == Lang::Ar
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::Ar => "ar",
            Lang::En => "en",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_locale_strings() {
        assert_eq!(Lang::parse("ar_SY.UTF-8"), Lang::Ar);
        assert_eq!(Lang::parse("AR"), Lang::Ar);
        assert_eq!(Lang::parse("en_US.UTF-8"), Lang::En);
        assert_eq!(Lang::parse("de"), Lang::En);
    }
}
