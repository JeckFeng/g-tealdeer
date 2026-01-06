use std::{
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

use log::{debug, warn};
use wait_timeout::ChildExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

pub fn run_command(spec: &CommandSpec, timeout: Duration) -> Result<ExecOutput, String> {
    debug!(
        "Spawning command: {} {}",
        spec.program.display(),
        spec.args.join(" ")
    );
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {e}", spec.program.display()))?;

    let status = child
        .wait_timeout(timeout)
        .map_err(|e| format!("Failed to wait for process: {e}"))?;

    let mut timed_out = false;
    let exit_status = match status {
        Some(status) => status,
        None => {
            timed_out = true;
            let _ = child.kill();
            child
                .wait()
                .map_err(|e| format!("Failed to wait after kill: {e}"))?
        }
    };
    if timed_out {
        warn!(
            "Command timed out after {:?}: {}",
            timeout,
            spec.program.display()
        );
    }

    let mut stdout = String::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_string(&mut stdout)
            .map_err(|e| format!("Failed to read stdout: {e}"))?;
    }

    let mut stderr = String::new();
    if let Some(mut err) = child.stderr.take() {
        err.read_to_string(&mut stderr)
            .map_err(|e| format!("Failed to read stderr: {e}"))?;
    }
    debug!(
        "Command finished: {} status={:?}",
        spec.program.display(),
        exit_status.code()
    );

    Ok(ExecOutput {
        status: exit_status.code(),
        stdout,
        stderr,
        timed_out,
    })
}
