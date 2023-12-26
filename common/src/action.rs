use std::{process::Command, fmt::Display};

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
    pub fn find_by_name(name: &String, actions: &Vec<ActionDefinition>) -> Option<ActionDefinition> {
        actions.iter().find(|a| &a.name == name).cloned()
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
