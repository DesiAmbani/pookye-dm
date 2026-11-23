//! Path helpers: host names, reveal in folder, open a file.

use super::prelude::*;

pub(crate) fn host_of(url: &str) -> String {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

pub(crate) fn show_in_folder(path: &Path) {
    if path.exists() {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("explorer")
                .arg(format!("/select,{}", path.display()))
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if let Some(folder) = path.parent() {
                let _ = std::process::Command::new("xdg-open").arg(folder).spawn();
            }
        }
    } else if let Some(folder) = path.parent() {
        let folder = folder.to_path_buf();
        if folder.exists() {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("explorer").arg(&folder).spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&folder).spawn();
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                let _ = std::process::Command::new("xdg-open").arg(&folder).spawn();
            }
        }
    }
}

pub(crate) fn open_file(path: &Path) {
    if !path.exists() {
        return;
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}
