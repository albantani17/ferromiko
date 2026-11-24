use regex::Regex;

/// Removes the echoed command (first line) and, when `prompt` is given, the trailing prompt line.
pub fn clean_output(output: &str, command: &str, prompt: Option<&Regex>) -> String {
    let mut lines: Vec<&str> = output.lines().collect();
    if lines.first().is_some_and(|first| is_command_echo(first, command)) {
        lines.remove(0);
    }
    if let Some(prompt) = prompt
        && lines.last().is_some_and(|last| prompt.is_match(last))
    {
        lines.pop();
    }
    lines.join("\n")
}

fn is_command_echo(line: &str, command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return line.trim().is_empty();
    }
    line.trim_end().ends_with(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::prompt::prompt_pattern::prompt_pattern;

    #[test]
    fn removes_echo_and_trailing_prompt() {
        let prompt = prompt_pattern("R1");
        let output = "show clock\n12:00:00 WIB\nR1#";
        assert_eq!(clean_output(output, "show clock", Some(&prompt)), "12:00:00 WIB");
    }

    #[test]
    fn keeps_multi_line_body() {
        let prompt = prompt_pattern("R1");
        let output = "show ip int brief\nGi0/0 up\nGi0/1 down\nR1#";
        assert_eq!(clean_output(output, "show ip int brief", Some(&prompt)), "Gi0/0 up\nGi0/1 down");
    }

    #[test]
    fn keeps_last_line_when_it_is_not_a_prompt() {
        let output = "copy run start\nDestination filename [startup-config]?";
        assert_eq!(clean_output(output, "copy run start", None), "Destination filename [startup-config]?");
    }

    #[test]
    fn keeps_first_line_when_it_is_not_the_echo() {
        let prompt = prompt_pattern("R1");
        assert_eq!(clean_output("hello\nR1#", "echo hello", Some(&prompt)), "hello");
    }

    #[test]
    fn returns_empty_when_only_echo_and_prompt() {
        let prompt = prompt_pattern("R1");
        assert_eq!(clean_output("terminal length 0\nR1#", "terminal length 0", Some(&prompt)), "");
    }

    #[test]
    fn treats_blank_first_line_as_echo_of_empty_command() {
        assert_eq!(clean_output("\nR1#", "", None), "R1#");
    }
}
