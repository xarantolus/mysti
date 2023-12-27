use std::{fmt::Display, process::Command};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Action {
    pub action: String,
    #[serde(default = "Vec::new")]
    pub args: Vec<String>,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.action)?;
        for arg in &self.args {
            write!(f, " {}", arg)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ActionDefinition {
    pub name: String,

    pub linux: Option<String>,
    pub macos: Option<String>,
    pub windows: Option<String>,
}

impl ActionDefinition {
    pub fn find_by_name(
        name: &String,
        actions: &Vec<ActionDefinition>,
    ) -> Option<ActionDefinition> {
        actions.iter().find(|a| &a.name == name).cloned()
    }

    pub fn required_args(&self) -> usize {
        // Find the number of arguments in the command_string, as in $1, %2, etc
        let mut max_arg = 0;
        if let Some(command_string) = self.command_string().ok() {
            let mut arg = String::new();
            let mut expect_digit = false;

            for c in command_string.chars() {
                if c == '%' || c == '$' {
                    expect_digit = true;
                } else if expect_digit && c.is_digit(10) {
                    arg.push(c);
                } else if expect_digit {
                    expect_digit = false;
                    let arg_num = arg.parse::<usize>().unwrap_or(0);
                    if arg_num > max_arg {
                        max_arg = arg_num;
                    }
                    arg.clear();
                }
            }

            let arg_num = arg.parse::<usize>().unwrap_or(0);
            if arg_num > max_arg {
                max_arg = arg_num;
            }

            arg.clear();
        }

        max_arg
    }

    pub fn is_available(&self) -> bool {
        match std::env::consts::OS {
            "linux" => self.linux.is_some(),
            "macos" => self.macos.is_some(),
            "windows" => self.windows.is_some(),
            _ => false,
        }
    }

    fn command_string(&self) -> Result<&String> {
        match std::env::consts::OS {
            "linux" => self.linux.as_ref(),
            "macos" => self.macos.as_ref(),
            "windows" => self.windows.as_ref(),
            _ => None,
        }
        .context(format!(
            "command {} is not defined for operating system {}",
            self.name,
            std::env::consts::OS
        ))
    }

    fn to_command(&self, args: &Vec<String>) -> Result<Command> {
        let command_string = self.command_string()?;

        let split_results: Vec<String> = shell_words::split(command_string)?;
        if split_results.len() == 0 {
            return Err(anyhow::anyhow!("command {} is empty", self.name));
        }

        if args.len() < self.required_args() {
            return Err(anyhow::anyhow!(
                "command {} requires {} arguments, but only {} were provided",
                self.name,
                self.required_args(),
                args.len()
            ));
        }

        // Replace %1, etc or $1, $2 in the command string with the arguments
        // Note that we might have escaped % or $ characters in the command string
        // and that we might have multiple digits in the argument number
        let mut command = Command::new(split_results[0].clone());

        for i in 1..split_results.len() {
            let mut arg = split_results[i].clone();
            for j in (0..args.len()).rev() {
                arg = arg.replace(&format!("%{}", j + 1), &args[j]);
                arg = arg.replace(&format!("${}", j + 1), &args[j]);
            }
            command.arg(arg);
        }

        Ok(command)
    }

    pub fn run(&self, args: &Vec<String>) -> Result<()> {
        let mut command = self.to_command(args)?;

        let _ = command
            .spawn()
            .context(format!("failed to run command {}", self.name))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_required_args() {
        let command_str = Some("echo %1 %3 $22m %8ß".to_string());
        let action = ActionDefinition {
            name: "test".to_string(),
            linux: command_str.clone(),
            macos: command_str.clone(),
            windows: command_str.clone(),
        };

        assert_eq!(action.required_args(), 22);

        let cmd_list = action.to_command(&vec!["a".to_string(); 22]).unwrap();

        let cmd_list: Vec<&str> = cmd_list
            .get_args()
            .into_iter()
            .map(|s| s.to_str().unwrap())
            .collect();

        assert_eq!(cmd_list, vec!["a", "a", "am", "aß"]);
    }
}
