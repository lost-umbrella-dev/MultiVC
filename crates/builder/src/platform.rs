pub enum Platform {
    Windows,
    MacOS,
    Linux(LinuxDistro),
    NixOS,
}

pub enum LinuxDistro {
    Debian, // apt
    Fedora, // dnf
    Arch,   // pacman
    Unknown,
}

pub fn detect() -> Platform {
    if cfg!(target_os = "windows") {
        return Platform::Windows;
    }
    if cfg!(target_os = "macos") {
        return Platform::MacOS;
    }
    // Linux: check NixOS first
    if is_nixos() {
        return Platform::NixOS;
    }
    Platform::Linux(detect_linux_distro())
}

fn is_nixos() -> bool {
    std::path::Path::new("/etc/NIXOS").exists() || std::env::var("NIX_PROFILES").is_ok()
}

fn detect_linux_distro() -> LinuxDistro {
    let Ok(content) = std::fs::read_to_string("/etc/os-release") else {
        return LinuxDistro::Unknown;
    };
    for line in content.lines() {
        if let Some(id) = line.strip_prefix("ID=") {
            let id = id.trim_matches('"').to_lowercase();
            return match id.as_str() {
                "ubuntu" | "debian" | "linuxmint" | "pop" | "elementary" | "zorin" => {
                    LinuxDistro::Debian
                },
                "fedora" | "rhel" | "centos" | "rocky" | "alma" => LinuxDistro::Fedora,
                "arch" | "manjaro" | "endeavouros" | "garuda" => LinuxDistro::Arch,
                _ => LinuxDistro::Unknown,
            };
        }
    }
    LinuxDistro::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_valid_platform() {
        let p = detect();
        // Verify it doesn't panic and returns a valid variant
        match p {
            Platform::Windows | Platform::MacOS | Platform::NixOS | Platform::Linux(_) => {},
        }
    }
}
