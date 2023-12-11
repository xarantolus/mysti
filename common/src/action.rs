use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Shutdown,
    Logoff,
    Reboot,
    Media,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub action: ActionType,
    #[serde(default = "Vec::new")]
    pub args: Vec<String>,
}

impl Action {
    pub async fn run(&self) -> Result<()> {
        match self.action {
            ActionType::Shutdown => shutdown(),
            ActionType::Logoff => logoff(),
            ActionType::Reboot => reboot(),
            ActionType::Media => media(&self.args),
        }
    }
}

fn shutdown() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        Command::new("shutdown")
            .arg("/s")
            .arg("/f")
            .arg("/t")
            .arg("0")
            .spawn()
            .context("Failed to shutdown")?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        Command::new("shutdown")
            .arg("-h")
            .arg("now")
            .spawn()
            .context("Failed to shutdown")?;

        Ok(())
    }
}

fn logoff() -> Result<()> {
    // On windows, run "rundll32.exe user32.dll,LockWorkStation"
    // This returns us to the login screen without closing programs
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        Command::new("rundll32.exe")
            .arg("user32.dll,LockWorkStation")
            .spawn()
            .context("Failed to logoff")?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        Command::new("loginctl")
            .arg("terminate-user")
            .arg("philipp")
            .spawn()
            .context("Failed to logoff")?;

        Ok(())
    }
}

fn reboot() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        Command::new("shutdown")
            .arg("/r")
            .arg("/f")
            .arg("/t")
            .arg("0")
            .spawn()
            .context("Failed to reboot")?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        Command::new("shutdown")
            .arg("-r")
            .arg("now")
            .spawn()
            .context("Failed to reboot")?;

        Ok(())
    }
}

fn media(args: &Vec<String>) -> Result<()> {
    // we basically want to send keyboard events, depending on the first argument
    if args.len() != 1 {
        return Err(anyhow::anyhow!("Invalid number of arguments"));
    }

    let key = args[0].to_uppercase();

    #[cfg(target_os = "windows")]
    {
        use winapi::um::winuser::{
            keybd_event, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VK_LEFT, VK_RIGHT, VK_SCROLL,
            VK_SPACE, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
        };

        let key_code = match key.as_str() {
            "NEXT" => VK_RIGHT as u8,
            "PREV" => VK_LEFT as u8,
            "PLAYPAUSE" => VK_SPACE as u8,
            "VOLUP" => VK_VOLUME_UP as u8,
            "VOLDOWN" => VK_VOLUME_DOWN as u8,
            "MUTE" => VK_SCROLL as u8,
            "VFORWARD" => VK_RIGHT as u8,
            "VBACKWARD" => VK_LEFT as u8,
            "VPAUSE" => VK_SPACE as u8,
            _ => {
                return Err(anyhow::anyhow!("Invalid argument"));
            }
        };

        unsafe {
            keybd_event(key_code, 0, 0, 0);
            keybd_event(key_code, 0, KEYEVENTF_KEYUP, 0);
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        _ = key;
        return Err(anyhow::anyhow!("Not implemented"));
    }
}
