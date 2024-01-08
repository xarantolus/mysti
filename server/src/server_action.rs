use anyhow::Context;
use log::info;

use crate::config::ClipboardAction;

impl ClipboardAction {
    pub fn matches(&self, clipboard: &str) -> (bool, Vec<String>) {
        let mut matches = false;
        let mut args = Vec::new();

        if let Some(regex) = &self.compiled_regex {
            if let Some(captures) = regex.captures(clipboard) {
                matches = true;

                for i in 1..captures.len() {
                    if let Some(capture) = captures.get(i) {
                        args.push(capture.as_str().to_string());
                    }
                }
            }
        }

        (matches, args)
    }

    pub fn run(&self, args: Vec<String>) -> anyhow::Result<()> {
        let mut command = self.command.clone();

        for (i, arg) in args.iter().enumerate() {
            command = command.replace(&format!("${}", i + 1), arg);
            command = command.replace(&format!("%{}", i + 1), arg);
        }

        // get default $PATH
        let mut command_path = std::env::var("PATH").unwrap_or_default();

        // If we have an environment variable HOSTPATH and HOSTMOUNT, we modify the default path passed to the command
        if let (Some(hostpath), Some(hostmount)) = (
            std::env::var("HOSTPATH").ok(),
            std::env::var("HOSTMOUNT").ok(),
        ) {
            let mut new_paths = Vec::new();

            for dir in hostpath.split(":") {
                new_paths.push(format!("{}{}", hostmount, dir));
            }

            if command_path.is_empty() {
                command_path = new_paths.join(":");
            } else {
                command_path = new_paths.join(":") + ":" + &command_path;
            }
        }

        info!("Running command: {}", command);

        // Run in bash
        let mut cmd = std::process::Command::new("bash");
        cmd.env("PATH", command_path);
        cmd.arg("-c");
        cmd.arg(command);

        let output = cmd.output().context("Failed to spawn process")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            return Err(anyhow::anyhow!(
                "Command failed with status code {}: {}{}",
                output.status,
                stderr,
                stdout
            ));
        }

        Ok(())
    }
}
