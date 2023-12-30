use std::{env, process::Command};

pub fn client_name() -> String {
    // Get current user name
    let user_name = if let Ok(user) = env::var("USER") {
        user
    } else if let Ok(user) = env::var("USERNAME") {
        user
    } else {
        "UnknownUser".to_string()
    };

    // Get computer name (hostname)
    let hostname = Command::new("hostname")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "UnknownHost".to_string());

    // Get distribution name or Windows version
    let dist_name = {
        #[cfg(target_os = "linux")]
        {
            let mut distro: Option<String> = None;

            use std::fs::File;
            use std::io::{BufRead, BufReader};

            if let Ok(file) = File::open("/etc/os-release") {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if line.starts_with("PRETTY_NAME=") {
                            distro = Some(
                                line.trim_start_matches("PRETTY_NAME=")
                                    .trim_matches('"')
                                    .to_string(),
                            );
                            break;
                        }
                    }
                }
            }

            distro.unwrap_or("UnknownLinux".to_string())
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("powershell")
                .arg("-Command")
                .arg(
                    "Get-CimInstance Win32_OperatingSystem | Select-Object -ExpandProperty Caption",
                )
                .output()
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                .unwrap_or("UnknownWindows".to_string())
        }

        #[cfg(target_os = "macos")]
        {
            // Mac is not really supported tbh, but might work?
            distro.unwrap_or("UnknownMac".to_string())
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            "UnknownOS".to_string()
        }
    };

    format!("{} on {} ({})", user_name, hostname, dist_name)
}
