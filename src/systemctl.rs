use anyhow::bail;
use std::ffi::OsStr;
use tokio::process::Command;

fn to_str(out: Vec<u8>) -> String {
    String::from_utf8(out).unwrap()
}

async fn systemctl_do<S: AsRef<OsStr>, T: AsRef<OsStr>>(verb: S, unit: T) -> anyhow::Result<()> {
    let output = Command::new("systemctl")
        .arg(verb)
        .arg(unit)
        .output()
        .await?;
    if !output.status.success() {
        bail!(
            "systemctl failed with {}\n\n{}",
            output.status,
            to_str(output.stderr)
        );
    }
    Ok(())
}

pub async fn start<S: AsRef<OsStr>>(unit: S) -> anyhow::Result<()> {
    systemctl_do("start", &unit).await
}

pub async fn stop<S: AsRef<OsStr>>(unit: S) -> anyhow::Result<()> {
    systemctl_do("stop", &unit).await
}

pub async fn restart<S: AsRef<OsStr>>(unit: S) -> anyhow::Result<()> {
    systemctl_do("restart", &unit).await
}
