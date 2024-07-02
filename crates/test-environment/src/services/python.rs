use crate::io::OutputStream;

use super::Service;
use anyhow::Context as _;
use std::{
    cell::OnceCell,
    collections::HashMap,
    path::Path,
    process::{Command, Stdio},
};

/// A python script as a service
pub struct PythonService {
    child: std::process::Child,
    stdout: OutputStream,
    ports: OnceCell<HashMap<u16, u16>>,
    _lock: fslock::LockFile,
    ready: bool,
}

impl PythonService {
    pub fn start(
        name: &str,
        script_path: &Path,
        working_dir: &Path,
        lock_dir: &Path,
    ) -> anyhow::Result<Self> {
        let lock_path = lock_dir.join(format!("{name}.lock"));
        let mut lock =
            fslock::LockFile::open(&lock_path).context("failed to open service file lock")?;
        lock.lock().context("failed to obtain service file lock")?;
        let mut child = python()
            .current_dir(working_dir)
            .arg(script_path.display().to_string())
            .stdout(Stdio::piped())
            .spawn()
            .context("service failed to spawn")?;
        std::thread::sleep(std::time::Duration::from_millis(1000));
        Ok(Self {
            stdout: OutputStream::new(
                child
                    .stdout
                    .take()
                    .expect("child process somehow does not have stdout"),
            ),
            child,
            ports: OnceCell::new(),
            _lock: lock,
            ready: false,
        })
    }
}

impl Service for PythonService {
    fn name(&self) -> &str {
        "python"
    }

    fn ready(&mut self) -> anyhow::Result<()> {
        while !self.ready {
            let stdout = self
                .stdout
                .output_as_str()
                .context("stdout is not valid utf8")?;
            if stdout.contains("READY") {
                self.ready = true;
            }
        }
        let exit = self.child.try_wait()?;
        if exit.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "python service process exited early",
            )
            .into());
        }

        Ok(())
    }

    fn ports(&mut self) -> anyhow::Result<&HashMap<u16, u16>> {
        let stdout = self
            .stdout
            .output_as_str()
            .context("stdout is not valid utf8")?;
        match self.ports.get() {
            Some(ports) => Ok(ports),
            None => {
                let ports = stdout
                    .lines()
                    .filter_map(|l| l.trim().split_once('='))
                    .map(|(k, v)| -> anyhow::Result<_> {
                        let k = k.trim();
                        let v = v.trim();
                        if k == "PORT" {
                            let err = "malformed service port pair - PORT values should be in the form PORT=(80,8080)";
                            let (port_in, port_out) = v.split_once(',').context(err)?;
                            let port_in = port_in.trim().strip_prefix('(').context(err)?;
                            let port_out = port_out.trim().strip_suffix(')').context(err)?;
                            Ok(Some((port_in.parse::<u16>().context("port number was not a number")?, port_out.parse::<u16>().context("port number was not a number")?)))
                        } else {
                            Ok(None)
                        }
                    })
                    .filter_map(|r| r.transpose())
                    .collect::<anyhow::Result<HashMap<_, _>>>()?;
                Ok(self.ports.get_or_init(|| ports))
            }
        }
    }
}

impl Drop for PythonService {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn python() -> Command {
    Command::new("python3")
}
