use crate::models::HostKeyInfo;
use std::io::Write;
use std::process::{Command, Stdio};

const KEYSCAN_TIMEOUT_SECS: &str = "5";

/// Builds a command that doesn't flash a console window on Windows. Shared by
/// every OpenSSH binary we shell out to.
pub fn hidden_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    cmd
}

/// How OpenSSH keys an entry in known_hosts: a bare host on port 22,
/// `[host]:port` on anything else.
pub fn host_target(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    }
}

fn spawn_error(program: &str, e: std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::NotFound {
        format!("{program} was not found on PATH")
    } else {
        format!("Failed to run {program}: {e}")
    }
}

fn last_line(text: &str, fallback: &str) -> String {
    text.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(fallback)
        .trim()
        .to_string()
}

/// Asks the host what key it's presenting right now and fingerprints it, and
/// looks up what known_hosts currently has saved for it.
pub fn scan(host: &str, port: u16) -> HostKeyInfo {
    let target = host_target(host, port);
    let saved_fingerprints = saved(&target).unwrap_or_default();

    let result = keyscan(host, port).and_then(|keys| fingerprint(&keys));

    match result {
        Ok(fingerprints) => HostKeyInfo {
            target,
            fingerprints,
            scan_error: None,
            saved_fingerprints,
        },
        Err(scan_error) => HostKeyInfo {
            target,
            fingerprints: Vec::new(),
            scan_error: Some(scan_error),
            saved_fingerprints,
        },
    }
}

/// Fingerprints of the key(s) saved in known_hosts for `target`, via
/// `ssh-keygen -l -F`. An empty list means nothing is saved (or we couldn't
/// tell); the UI just leaves the "saved key" row out then.
fn saved(target: &str) -> Result<Vec<String>, String> {
    let output = hidden_command("ssh-keygen")
        .arg("-l")
        .arg("-F")
        .arg(target)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| spawn_error("ssh-keygen", e))?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_saved_line)
        .collect())
}

/// `ssh-keygen -l -F host` prints `# Host ... found: line N` comments and then
/// `<host> <TYPE> <hash>`; we want `<TYPE> <hash>` to match `parse_fingerprint_line`.
fn parse_saved_line(line: &str) -> Option<String> {
    if line.trim_start().starts_with('#') {
        return None;
    }
    let mut parts = line.split_whitespace();
    let _host = parts.next()?;
    let key_type = parts.next()?;
    let hash = parts.next()?;
    if !hash.contains(':') {
        return None;
    }
    Some(format!("{key_type} {hash}"))
}

fn keyscan(host: &str, port: u16) -> Result<String, String> {
    let output = hidden_command("ssh-keyscan")
        .arg("-T")
        .arg(KEYSCAN_TIMEOUT_SECS)
        .arg("-p")
        .arg(port.to_string())
        .arg(host)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| spawn_error("ssh-keyscan", e))?;

    // ssh-keyscan writes comments and diagnostics to stderr and can exit 0
    // having reached nothing, so judge it by whether a key actually came back.
    let keys = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    if keys.is_empty() {
        // An unreachable host exits non-zero with nothing on either stream, so
        // there is often no diagnostic to pass along.
        return Err(last_line(
            &String::from_utf8_lossy(&output.stderr),
            &format!("could not reach {host} on port {port}"),
        ));
    }

    Ok(keys)
}

fn fingerprint(keys: &str) -> Result<Vec<String>, String> {
    let mut child = hidden_command("ssh-keygen")
        .arg("-l")
        .arg("-f")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| spawn_error("ssh-keygen", e))?;

    // Taking stdin and letting it drop at the end of this statement closes the
    // pipe, which is what makes ssh-keygen finish reading.
    child
        .stdin
        .take()
        .ok_or_else(|| "Failed to open ssh-keygen stdin".to_string())?
        .write_all(keys.as_bytes())
        .map_err(|e| format!("Failed to write to ssh-keygen: {e}"))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to read ssh-keygen output: {e}"))?;

    let fingerprints: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_fingerprint_line)
        .collect();

    if fingerprints.is_empty() {
        return Err(last_line(
            &String::from_utf8_lossy(&output.stderr),
            "could not fingerprint the host key",
        ));
    }

    Ok(fingerprints)
}

/// `ssh-keygen -l` prints `<bits> <hash> <comment> (<TYPE>)`; we want
/// `<TYPE> <hash>`, e.g. `ED25519 SHA256:9k2Lx...mQ4t`.
fn parse_fingerprint_line(line: &str) -> Option<String> {
    let hash = line.split_whitespace().nth(1)?;
    if !hash.contains(':') {
        return None;
    }
    let key_type = line.rsplit_once('(')?.1.trim_end_matches(')').trim();
    Some(format!("{key_type} {hash}"))
}

/// Removes every known_hosts entry for this host. This is `ssh-keygen -R`, the
/// same thing you'd run by hand; it leaves a backup at `known_hosts.old`.
pub fn remove(host: &str, port: u16) -> Result<String, String> {
    let target = host_target(host, port);

    let output = hidden_command("ssh-keygen")
        .arg("-R")
        .arg(&target)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| spawn_error("ssh-keygen", e))?;

    if !output.status.success() {
        return Err(last_line(
            &String::from_utf8_lossy(&output.stderr),
            "ssh-keygen -R failed",
        ));
    }

    Ok(format!("Removed the saved host key for {target}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_22_uses_a_bare_host_name() {
        assert_eq!(host_target("bastion.example.com", 22), "bastion.example.com");
    }

    #[test]
    fn other_ports_are_bracketed() {
        assert_eq!(
            host_target("bastion.example.com", 2222),
            "[bastion.example.com]:2222"
        );
    }

    #[test]
    fn parses_a_fingerprint_line() {
        assert_eq!(
            parse_fingerprint_line("256 SHA256:9k2LxmQ4t bastion.example.com (ED25519)").as_deref(),
            Some("ED25519 SHA256:9k2LxmQ4t")
        );
    }

    #[test]
    fn parses_a_saved_known_hosts_line() {
        assert_eq!(
            parse_saved_line("[bastion.example.com]:2222 ED25519 SHA256:Xq8rT0m2Lk9v").as_deref(),
            Some("ED25519 SHA256:Xq8rT0m2Lk9v")
        );
        assert_eq!(parse_saved_line("# Host [bastion.example.com]:2222 found: line 3"), None);
        assert_eq!(parse_saved_line(""), None);
    }

    #[test]
    fn ignores_lines_that_are_not_fingerprints() {
        assert_eq!(parse_fingerprint_line("# bastion.example.com:22 SSH-2.0"), None);
        assert_eq!(parse_fingerprint_line(""), None);
    }
}
