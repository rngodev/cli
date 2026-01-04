use serde::Deserialize;
use std::fmt;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PathPart {
    Index(i64),
    Field(String),
}

#[derive(Debug, Deserialize)]
pub struct ProblemIssue {
    message: String,
    path: Option<Vec<PathPart>>,
}

impl fmt::Display for ProblemIssue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = match &self.path {
            Some(path) => {
                let mut path_str = "".to_string();

                for path_part in path {
                    match path_part {
                        PathPart::Index(i) => path_str += &format!("[{}]", i),
                        PathPart::Field(s) if !path_str.is_empty() => {
                            path_str += &format!(".{}", s)
                        }
                        PathPart::Field(s) => path_str = s.into(),
                    }
                }

                &format!("{path}: {message}", path = path_str, message = self.message)
            }
            None => &self.message,
        };

        write!(f, "{}", str)
    }
}

#[derive(Debug, Deserialize)]
pub struct Problem {
    title: String,
    issues: Vec<ProblemIssue>,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.issues.is_empty() {
            let issues = self
                .issues
                .iter()
                .map(|item| format!("  {}", item))
                .collect::<Vec<_>>()
                .join("\n");

            write!(f, "{title}\n{issues}", title = self.title, issues = issues)
        } else {
            write!(f, "{}", self.title)
        }
    }
}

impl std::error::Error for Problem {}
