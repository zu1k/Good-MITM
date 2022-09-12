use nix::unistd::getegid;
use rcgen::Certificate;
use std::{env, fs, path::Path, process::Command};

pub fn install_cert(cert: Certificate) {
    if getegid().as_raw() != 0 {
        println!("Please run with root permission");
        return;
    }

    let (system_trust_filename, trust_cmd, trust_cmd_args) = {
        if path_exist("/etc/pki/ca-trust/source/anchors/") {
            (
                "/etc/pki/ca-trust/source/anchors/{cert-name}.pem",
                "update-ca-trust",
                vec!["extract"],
            )
        } else if path_exist("/usr/local/share/ca-certificates/") {
            (
                "/usr/local/share/ca-certificates/{cert-name}.crt",
                "update-ca-certificates",
                vec![],
            )
        } else if path_exist("/etc/ca-certificates/trust-source/anchors/") {
            (
                "/etc/ca-certificates/trust-source/anchors/{cert-name}.crt",
                "trust",
                vec!["extract-compat"],
            )
        } else if path_exist("/usr/share/pki/trust/anchors") {
            (
                "/usr/share/pki/trust/anchors/{cert-name}.pem",
                "update-ca-certificates",
                vec![],
            )
        } else {
            ("good-mitm.pem", "", vec![])
        }
    };

    let cert = cert.serialize_pem().expect("serialize cert to pem format");
    let system_trust_name = system_trust_filename.replace("{cert-name}", "good-mitm");
    fs::write(system_trust_name, cert).expect("write cert to system trust ca location");

    if trust_cmd.is_empty() {
        println!(
            "Installing to the system store is not yet supported on this Linux 😣 but Firefox and/or Chrome/Chromium will still work.",
        );
        let cert_path = Path::new(&get_ca_root()).join("good-mitm.pem");
        println!(
            "You can also manually install the root certificate at {}.",
            cert_path.to_str().unwrap()
        );
    } else {
        Command::new(trust_cmd)
            .args(trust_cmd_args)
            .status()
            .expect("failed to execute trust command");
    }
}

fn get_ca_root() -> String {
    if let Ok(v) = env::var("CAROOT") {
        return v;
    }

    let mut dir = {
        if let Ok(v) = env::var("XDG_DATA_HOME") {
            return v;
        }
        if let Ok(v) = env::var("HOME") {
            return Path::new(&v)
                .join(".local")
                .join("share")
                .to_str()
                .map(|s| s.to_string())
                .unwrap();
        }
        String::new()
    };

    if !dir.is_empty() {
        dir = Path::new(&dir)
            .join("mitm")
            .into_os_string()
            .into_string()
            .unwrap()
    }

    dir
}

#[inline]
fn path_exist(path: &str) -> bool {
    Path::new(path).exists()
}
