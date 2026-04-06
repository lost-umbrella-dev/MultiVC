//! Строго типизированная версия вида `{prefix}{major}.{minor}.{patch}-{suffix}`.
//!
//! Примеры: `v0.0.1`, `v1.30.1`, `p1.20.3-beta`, `1.0.0`, `1.0.0-rc.1`.
//!
//! - `prefix` и `suffix` опциональны.
//! - При парсинге через [`FromStr`] принимается любая строка, подходящая под формат.
//! - [`Version::with_default_prefix`] добавляет `"v"`, если prefix отсутствует.
//! - Serde реализован через `Display`/`FromStr` (строковое представление).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

/// Структурированная версия.
///
/// Формат: `{prefix}{major}.{minor}.{patch}[-{suffix}]`
///
/// ```text
/// v0.0.1        → prefix="v",  0.0.1,  suffix=None
/// p1.20.3-beta  → prefix="p",  1.20.3, suffix="beta"
/// 1.0.0-rc.1    → prefix=None, 1.0.0,  suffix="rc.1"
/// 1.0.0         → prefix=None, 1.0.0,  suffix=None
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    /// Необязательный префикс перед major (например `"v"`, `"p"`).
    pub prefix: Option<String>,
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    /// Необязательный суффикс после `-` (например `"beta"`, `"rc.1"`).
    pub suffix: Option<String>,
}

impl Version {
    /// Создаёт версию с заданными компонентами.
    pub fn new(
        prefix: Option<String>,
        major: u8,
        minor: u8,
        patch: u8,
        suffix: Option<String>,
    ) -> Self {
        Self {
            prefix,
            major,
            minor,
            patch,
            suffix,
        }
    }

    /// Возвращает копию с `prefix = "v"`, если prefix отсутствует.
    ///
    /// Если prefix уже задан — возвращает клон без изменений.
    pub fn with_default_prefix(mut self) -> Self {
        if self.prefix.is_none() {
            self.prefix = Some("v".to_owned());
        }
        self
    }
}

// ── Display ──────────────────────────────────────────────────────────

impl fmt::Display for Version {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        if let Some(ref prefix) = self.prefix {
            write!(f, "{prefix}")?;
        }
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref suffix) = self.suffix {
            write!(f, "-{suffix}")?;
        }
        Ok(())
    }
}

// ── Ord ─────────────────────────────────────────────────────────────

impl PartialOrd for Version {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(
        &self,
        other: &Self,
    ) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

// ── FromStr ──────────────────────────────────────────────────────────

impl FromStr for Version {
    type Err = VersionParseError;

    /// Парсит строку вида `[prefix]major.minor.patch[-suffix]`.
    ///
    /// Prefix — это любые символы до первой цифры.
    /// Suffix — всё после первого `-` **после** `patch`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(VersionParseError::Empty);
        }

        // 1. Отделяем prefix — всё до первой ASCII-цифры.
        let digit_start =
            s.find(|c: char| c.is_ascii_digit()).ok_or(VersionParseError::NoDigits)?;

        let prefix = if digit_start > 0 {
            Some(s[..digit_start].to_owned())
        } else {
            None
        };

        let rest = &s[digit_start..];

        // 2. Отделяем suffix — первый `-` после начала цифр.
        let (version_part, suffix) = match rest.find('-') {
            Some(idx) => {
                let suf = &rest[idx + 1..];
                if suf.is_empty() {
                    return Err(VersionParseError::EmptySuffix);
                }
                (&rest[..idx], Some(suf.to_owned()))
            },
            None => (rest, None),
        };

        // 3. Парсим major.minor.patch
        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionParseError::InvalidFormat {
                detail: format!(
                    "expected 3 dot-separated components (major.minor.patch), got {}",
                    parts.len()
                ),
            });
        }

        let major = parts[0].parse::<u8>().map_err(|_| VersionParseError::ComponentOverflow {
            component: "major",
            value: parts[0].to_owned(),
        })?;

        let minor = parts[1].parse::<u8>().map_err(|_| VersionParseError::ComponentOverflow {
            component: "minor",
            value: parts[1].to_owned(),
        })?;

        let patch = parts[2].parse::<u8>().map_err(|_| VersionParseError::ComponentOverflow {
            component: "patch",
            value: parts[2].to_owned(),
        })?;

        Ok(Version {
            prefix,
            major,
            minor,
            patch,
            suffix,
        })
    }
}

// ── VersionParseError ────────────────────────────────────────────────

/// Ошибка парсинга [`Version`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionParseError {
    /// Пустая строка.
    Empty,
    /// Строка не содержит ни одной цифры.
    NoDigits,
    /// Суффикс после `-` пуст.
    EmptySuffix,
    /// Неверный формат (не 3 компоненты через `.`).
    InvalidFormat {
        detail: String,
    },
    /// Компонента не помещается в `u8` (0..=255).
    ComponentOverflow {
        component: &'static str,
        value: String,
    },
}

impl fmt::Display for VersionParseError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "version string is empty"),
            Self::NoDigits => write!(f, "version string contains no digits"),
            Self::EmptySuffix => write!(f, "suffix after '-' is empty"),
            Self::InvalidFormat {
                detail,
            } => write!(f, "invalid version format: {detail}"),
            Self::ComponentOverflow {
                component,
                value,
            } => {
                write!(f, "{component} component '{value}' is not a valid u8 (0..=255)")
            },
        }
    }
}

impl std::error::Error for VersionParseError {}

// ── Serde ────────────────────────────────────────────────────────────

impl Serialize for Version {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Version::from_str(&s).map_err(D::Error::custom)
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── FromStr ──────────────────────────────────────────────────

    #[test]
    fn parse_full_version_with_prefix() {
        let v: Version = "v0.0.1".parse().unwrap();
        assert_eq!(v.prefix.as_deref(), Some("v"));
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 1);
        assert_eq!(v.suffix, None);
    }

    #[test]
    fn parse_version_with_large_components() {
        let v: Version = "v1.30.1".parse().unwrap();
        assert_eq!(v.prefix.as_deref(), Some("v"));
        assert_eq!((v.major, v.minor, v.patch), (1, 30, 1));
    }

    #[test]
    fn parse_version_with_prefix_and_suffix() {
        let v: Version = "p1.20.3-beta".parse().unwrap();
        assert_eq!(v.prefix.as_deref(), Some("p"));
        assert_eq!((v.major, v.minor, v.patch), (1, 20, 3));
        assert_eq!(v.suffix.as_deref(), Some("beta"));
    }

    #[test]
    fn parse_version_no_prefix() {
        let v: Version = "1.0.0".parse().unwrap();
        assert_eq!(v.prefix, None);
        assert_eq!((v.major, v.minor, v.patch), (1, 0, 0));
        assert_eq!(v.suffix, None);
    }

    #[test]
    fn parse_version_no_prefix_with_suffix() {
        let v: Version = "1.0.0-rc.1".parse().unwrap();
        assert_eq!(v.prefix, None);
        assert_eq!((v.major, v.minor, v.patch), (1, 0, 0));
        assert_eq!(v.suffix.as_deref(), Some("rc.1"));
    }

    #[test]
    fn parse_version_suffix_with_multiple_dashes() {
        let v: Version = "v2.0.0-alpha-2".parse().unwrap();
        assert_eq!(v.prefix.as_deref(), Some("v"));
        assert_eq!((v.major, v.minor, v.patch), (2, 0, 0));
        // Только первый `-` после patch отделяет suffix, остаток — часть suffix.
        assert_eq!(v.suffix.as_deref(), Some("alpha-2"));
    }

    #[test]
    fn parse_max_u8_components() {
        let v: Version = "255.255.255".parse().unwrap();
        assert_eq!((v.major, v.minor, v.patch), (255, 255, 255));
    }

    // ── Display (roundtrip) ─────────────────────────────────────

    #[test]
    fn display_roundtrip_full() {
        let input = "v1.30.1";
        let v: Version = input.parse().unwrap();
        assert_eq!(v.to_string(), input);
    }

    #[test]
    fn display_roundtrip_with_suffix() {
        let input = "p1.20.3-beta";
        let v: Version = input.parse().unwrap();
        assert_eq!(v.to_string(), input);
    }

    #[test]
    fn display_roundtrip_no_prefix() {
        let input = "1.0.0";
        let v: Version = input.parse().unwrap();
        assert_eq!(v.to_string(), input);
    }

    #[test]
    fn display_roundtrip_no_prefix_with_suffix() {
        let input = "1.0.0-rc.1";
        let v: Version = input.parse().unwrap();
        assert_eq!(v.to_string(), input);
    }

    // ── with_default_prefix ─────────────────────────────────────

    #[test]
    fn with_default_prefix_adds_v() {
        let v: Version = "1.0.0".parse().unwrap();
        let v = v.with_default_prefix();
        assert_eq!(v.prefix.as_deref(), Some("v"));
        assert_eq!(v.to_string(), "v1.0.0");
    }

    #[test]
    fn with_default_prefix_preserves_existing() {
        let v: Version = "p1.0.0".parse().unwrap();
        let v = v.with_default_prefix();
        assert_eq!(v.prefix.as_deref(), Some("p"));
        assert_eq!(v.to_string(), "p1.0.0");
    }

    // ── Serde ───────────────────────────────────────────────────

    #[test]
    fn serde_json_roundtrip() {
        let v: Version = "v1.30.1-beta".parse().unwrap();
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#""v1.30.1-beta""#);

        let deserialized: Version = serde_json::from_str(&json).unwrap();
        assert_eq!(v, deserialized);
    }

    #[test]
    fn serde_deserialize_no_prefix() {
        let deserialized: Version = serde_json::from_str(r#""1.0.0""#).unwrap();
        assert_eq!(deserialized.prefix, None);
        assert_eq!((deserialized.major, deserialized.minor, deserialized.patch), (1, 0, 0));
    }

    // ── Errors ──────────────────────────────────────────────────

    #[test]
    fn error_empty() {
        assert_eq!("".parse::<Version>(), Err(VersionParseError::Empty));
    }

    #[test]
    fn error_no_digits() {
        assert_eq!("vvv".parse::<Version>(), Err(VersionParseError::NoDigits));
    }

    #[test]
    fn error_too_few_components() {
        let err = "v1.0".parse::<Version>().unwrap_err();
        assert!(matches!(err, VersionParseError::InvalidFormat { .. }));
    }

    #[test]
    fn error_too_many_components() {
        let err = "v1.0.0.0".parse::<Version>().unwrap_err();
        assert!(matches!(err, VersionParseError::InvalidFormat { .. }));
    }

    #[test]
    fn error_component_overflow() {
        let err = "v256.0.0".parse::<Version>().unwrap_err();
        assert!(matches!(
            err,
            VersionParseError::ComponentOverflow {
                component: "major",
                ..
            }
        ));
    }

    #[test]
    fn error_empty_suffix() {
        let err = "v1.0.0-".parse::<Version>().unwrap_err();
        assert_eq!(err, VersionParseError::EmptySuffix);
    }

    #[test]
    fn error_serde_invalid() {
        let result: Result<Version, _> = serde_json::from_str(r#""not-a-version""#);
        assert!(result.is_err());
    }
}
