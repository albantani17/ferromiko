use regex::Regex;

use super::profile::LoginQuestion;
use crate::Result;

/// The answer to the login question the output ends with, if any.
pub fn answer_login_question(questions: &[LoginQuestion], output: &str) -> Result<Option<&'static str>> {
    for (pattern, answer) in questions {
        if Regex::new(pattern)?.is_match(output) {
            return Ok(Some(answer));
        }
    }
    Ok(None)
}

/// Matches either a device prompt or one of the login questions.
pub fn prompt_or_question_pattern(any_prompt: &Regex, questions: &[LoginQuestion]) -> Result<Regex> {
    let alternatives: Vec<String> = std::iter::once(any_prompt.as_str())
        .chain(questions.iter().map(|(pattern, _)| *pattern))
        .map(|pattern| format!("(?:{pattern})"))
        .collect();
    Ok(Regex::new(&alternatives.join("|"))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::prompt::prompt_pattern::any_prompt_pattern;

    const QUESTIONS: &[LoginQuestion] = &[(r"Change now[^\n]*\z", "N"), (r"YES.NO[^\n]*\z", "yes")];

    #[test]
    fn answers_matching_question() {
        let output = "The password has expired.\nChange now? [Y/N]:";
        assert_eq!(answer_login_question(QUESTIONS, output).unwrap(), Some("N"));
    }

    #[test]
    fn returns_none_when_output_ends_with_prompt() {
        assert_eq!(answer_login_question(QUESTIONS, "Change now? [Y/N]: N\n<HUAWEI>").unwrap(), None);
    }

    #[test]
    fn returns_none_without_questions() {
        assert_eq!(answer_login_question(&[], "Change now? [Y/N]:").unwrap(), None);
    }

    #[test]
    fn combined_pattern_matches_prompt_and_each_question() {
        let pattern = prompt_or_question_pattern(&any_prompt_pattern(">]"), QUESTIONS).unwrap();
        assert!(pattern.is_match("\n<HUAWEI>"));
        assert!(pattern.is_match("Change now? [Y/N]:"));
        assert!(pattern.is_match("(YES/NO)[YES]:"));
        assert!(!pattern.is_match("Info: The max number of VTY users is 10."));
    }
}
