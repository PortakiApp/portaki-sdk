//! What a failing check reports.

use std::fmt;

/// The problems one or more checks found — displayed as the panic message of a failing test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Findings {
    sections: Vec<(String, Vec<String>)>,
}

impl Findings {
    /// The problems of one check.
    pub(crate) fn of(check: &str, problems: Vec<String>) -> Result<(), Self> {
        if problems.is_empty() {
            Ok(())
        } else {
            Err(Self {
                sections: vec![(check.to_string(), problems)],
            })
        }
    }

    pub(crate) fn merge(all: Vec<Findings>) -> Self {
        Self {
            sections: all.into_iter().flat_map(|f| f.sections).collect(),
        }
    }

    /// Every problem, without its check name — for assertions.
    pub fn problems(&self) -> Vec<&str> {
        self.sections
            .iter()
            .flat_map(|(_, problems)| problems.iter().map(String::as_str))
            .collect()
    }

    /// The names of the checks that failed.
    pub fn checks(&self) -> Vec<&str> {
        self.sections
            .iter()
            .map(|(check, _)| check.as_str())
            .collect()
    }
}

impl fmt::Display for Findings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, (check, problems)) in self.sections.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            let noun = if problems.len() == 1 {
                "problem"
            } else {
                "problems"
            };
            writeln!(
                f,
                "portaki conformance — {check}: {} {noun}",
                problems.len()
            )?;
            for problem in problems {
                writeln!(f, "  - {problem}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for Findings {}
