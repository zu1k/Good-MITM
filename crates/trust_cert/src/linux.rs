use std::{env, fs, io::Write, path::Path, process::Command};

pub fn install_cert() {
    let (system_trust_filename, system_trust_cmd) = {
        if path_exist("/etc/pki/ca-trust/source/anchors/") {
            (
                "/etc/pki/ca-trust/source/anchors/{cert-name}.pem",
                vec!["update-ca-trust", "extract"],
            )
        } else if path_exist("/usr/local/share/ca-certificates/") {
            (
                "/usr/local/share/ca-certificates/{cert-name}.crt",
                vec!["update-ca-certificates"],
            )
        } else if path_exist("/etc/ca-certificates/trust-source/anchors/") {
            (
                "/etc/ca-certificates/trust-source/anchors/{cert-name}.crt",
                vec!["trust", "extract-compat"],
            )
        } else if path_exist("/usr/share/pki/trust/anchors") {
            (
                "/usr/share/pki/trust/anchors/{cert-name}.pem",
                vec!["update-ca-certificates"],
            )
        } else {
            (
                "/etc/pki/ca-trust/source/anchors/{cert-name}.pem",
                vec!["update-ca-trust", "extract"],
            )
        }
    };

    let cert = Path::new(&get_ca_root()).join("mitm-vip-unlocker.pem");
    let cert = fs::read(cert).unwrap();

    let system_trust_name = system_trust_filename.replace("{cert-name}", "mitm-vip-unlocker");
    let mut cmd = cmd_with_sudo(vec!["tee", &system_trust_name])
        .spawn()
        .unwrap();
    let stdin = cmd.stdin.as_mut().unwrap();
    stdin.write_all(&cert).unwrap();

    cmd_with_sudo(system_trust_cmd).status().unwrap();
}

fn cmd_with_sudo(cmd: Vec<&str>) -> Command {
    let mut cmd = cmd;
    if unsafe { libc::getegid() } == 0 {
        let mut command = Command::new(cmd[0]);
        command.args(&cmd[1..]);
        return command;
    }

    let mut sudo_cmd = vec!["--prompt=Sudo password:", "--"];
    sudo_cmd.append(&mut cmd);
    let mut command = Command::new("sudo");
    command.args(&sudo_cmd);
    command
}

fn get_ca_root() -> String {
    if let Ok(v) = env::var("CAROOT") {
        return v;
    }

    let dir = {
        if let Ok(v) = env::var("XDG_DATA_HOM") {
            return v;
        }
        if let Ok(v) = env::var("HOME") {
            return Path::new(&v)
                .join(".local")
                .join("share")
                .into_os_string()
                .into_string()
                .unwrap();
        }
        String::new()
    };

    if dir.is_empty() {
        String::new()
    } else {
        Path::new(&dir)
            .join("mitm")
            .into_os_string()
            .into_string()
            .unwrap()
    }
}

fn path_exist(path: &str) -> bool {
    Path::new(path).exists()
}
