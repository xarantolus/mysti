use std::env;

pub fn client_name() -> String {
    // Get current user name
    let user_name = if let Some(user) = env::var_os("USER") {
        format!("{}", user.to_string_lossy())
    } else {
        "UnknownUser".to_string()
    };

    // Get computer name (hostname)
    let hostname = if let Ok(hostname) = sys_info::hostname() {
        hostname
    } else {
        "UnknownHost".to_string()
    };

    // Get distribution name or Windows version
    let dist_name = {
		let mut distro = "UnknownDistribution".to_string();

        #[cfg(target_os = "linux")]
        {
            use std::fs::File;
            use std::io::{BufRead, BufReader};

            if let Ok(file) = File::open("/etc/os-release") {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if line.starts_with("PRETTY_NAME=") {
							distro = line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string();
							break;
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use winapi::um::winnt::OSVERSIONINFOEXW;

            let mut os_version_info: OSVERSIONINFOEXW = Default::default();
            os_version_info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;

            distro = if unsafe { crate::winapi::GetVersionExW(&mut os_version_info as *mut OSVERSIONINFOEXW) } != 0 {
                format!("Windows {} (Build {})", os_version_info.dwMajorVersion, os_version_info.dwBuildNumber)
            } else {
				"UnknownWindowsVersion".to_string()
			}
        }

		distro
    };

    format!("{} on {} ({})", user_name, hostname, dist_name)
}
