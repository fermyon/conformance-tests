use super::Service;
use anyhow::{bail, Context as _};
use std::{
    cell::OnceCell,
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// A docker container as a service
pub struct DockerService {
    name: String,
    container: Container,
    // We declare lock after container so that the lock is dropped after the container is
    _lock: fslock::LockFile,
    ports: OnceCell<HashMap<u16, u16>>,
    ready: bool,
}

impl DockerService {
    /// Start a docker container as a service
    pub fn start(
        name: impl Into<String>,
        image: DockerImage,
        lock_dir: &Path,
    ) -> anyhow::Result<Self> {
        let name = name.into();
        let lock_path = lock_dir.join(format!("{name}.lock"));
        // TODO: ensure that `docker` is installed and available
        let mut lock =
            fslock::LockFile::open(&lock_path).context("failed to open service file lock")?;
        lock.lock().context("failed to obtain service file lock")?;

        let image_name = match image {
            DockerImage::FromDockerfile(dockerfile_path) => {
                let image_name = format!("test-environment/services/{name}");
                build_image(&dockerfile_path, &image_name)?;
                image_name
            }
            DockerImage::FromRegistry(image_name) => image_name,
        };
        let container = run_container(image_name)?;

        Ok(Self {
            name,
            container,
            _lock: lock,
            ports: OnceCell::new(),
            ready: false,
        })
    }
}

struct Container {
    id: String,
    image: String,
}

impl Container {
    fn get_ports(&self) -> anyhow::Result<HashMap<u16, u16>> {
        let output = Command::new("docker")
            .arg("port")
            .arg(&self.id)
            .output()
            .with_context(|| {
                format!(
                    "docker failed to run command to fetch ports for container for image '{}'",
                    self.image,
                )
            })?;
        if !output.status.success() {
            let stdout = String::from_utf8(output.stdout).unwrap_or_else(|_| "<non-utf8>".into());
            let stderr = String::from_utf8(output.stderr).unwrap_or_else(|_| "<non-utf8>".into());
            bail!(
                "failed to fetch ports for docker container for image {}:\n{stdout}\n{stderr}",
                self.image
            );
        }
        let output = String::from_utf8(output.stdout)?;
        output
            .lines()
            .map(|s| {
                // 3306/tcp -> 0.0.0.0:32770
                let parse = || -> anyhow::Result<(u16, u16)> {
                    let s = s.trim();
                    let (guest, host) = s
                        .split_once(" -> ")
                        .context("failed to parse port mapping")?;
                    let (guest_port, _) = guest
                        .split_once('/')
                        .context("guest mapping does not contain '/'")?;
                    let host_port = host
                        .rsplit(':')
                        .next()
                        .expect("`rsplit` should always return one element but somehow did not");
                    Ok((guest_port.parse()?, host_port.parse()?))
                };
                parse().with_context(|| {
                    format!(
                        "failed to parse port mapping for container for image '{}' from string '{s}'",
                        self.image
                    )
                })
            })
            .collect()
    }
}

#[derive(Debug)]
pub enum DockerImage {
    FromDockerfile(PathBuf),
    FromRegistry(String),
}

impl Drop for Container {
    fn drop(&mut self) {
        let _ = stop_containers(&[std::mem::take(&mut self.id)]);
    }
}

impl Service for DockerService {
    fn name(&self) -> &str {
        "docker"
    }

    fn ready(&mut self) -> anyhow::Result<()> {
        // docker container inspect -f '{{.State.Health.Status}}'
        while !self.ready {
            let output = Command::new("docker")
                .arg("container")
                .arg("inspect")
                .arg("-f")
                // Ensure that .State.Health exists and otherwise just print that it's healthy
                .arg("{{with .State.Health}}{{.Status}}{{else}}healthy{{end}}")
                .arg(&self.container.id)
                .output()
                .with_context(|| {
                    format!(
                        "failed to determine container health for '{}' service",
                        self.name
                    )
                })?;
            if !output.status.success() {
                let stderr = std::str::from_utf8(&output.stderr).unwrap_or("<non-utf8>");
                bail!(
                    "docker health status check failed for service '{}': {stderr}",
                    self.name
                );
            }
            let output = String::from_utf8(output.stdout)?;
            match output.trim() {
                "healthy" => self.ready = true,
                "unhealthy" => {
                    let output = Command::new("docker")
                        .arg("container")
                        .arg("inspect")
                        .arg("-f")
                        // Ensure that .State.Health exists and otherwise just print that there are no logs
                        .arg("{{with .State.Health}}{{json .Log}}{{else}}<NO LOG>{{end}}")
                        .arg(&self.container.id)
                        .output();
                    let logs = output
                        .as_ref()
                        .map(|o| String::from_utf8_lossy(&o.stdout))
                        .unwrap_or_else(|_| "<failed to get health check logs>".into());
                    bail!(
                        "docker container for '{}' service is unhealthy:\n{logs}",
                        self.name
                    )
                }
                _ => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
        Ok(())
    }

    fn ports(&mut self) -> anyhow::Result<&HashMap<u16, u16>> {
        match self.ports.get() {
            Some(p) => Ok(p),
            None => {
                let ports = self.container.get_ports()?;
                Ok(self.ports.get_or_init(|| ports))
            }
        }
    }
}

fn build_image(dockerfile_path: &Path, image_name: &String) -> anyhow::Result<()> {
    let docker_context_dir = dockerfile_path.parent().unwrap();
    let output = Command::new("docker")
        .arg("build")
        .arg("-f")
        .arg(dockerfile_path)
        .arg("-t")
        .arg(image_name)
        .arg(docker_context_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| {
            format!(
                "docker build failed to spawn for Dockerfile '{}'",
                dockerfile_path.display()
            )
        })?;

    if !output.status.success() {
        let stderr = std::str::from_utf8(&output.stderr).unwrap_or("<non-utf8>");
        bail!(
            "failed to build docker '{image_name}' image: status={} stderr={stderr}",
            output.status
        );
    }
    Ok(())
}

fn run_container(image_name: String) -> anyhow::Result<Container> {
    let output = Command::new("docker")
        .arg("run")
        .arg("-d")
        .arg("-P")
        .arg("--health-start-period=1s")
        .arg(&image_name)
        .output()
        .with_context(|| format!("docker run failed to spawn for image '{image_name}'"))?;
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;
        bail!("failed to run docker image for image '{image_name}': {stderr}");
    }
    let output = String::from_utf8(output.stdout)?;
    let id = output.trim().to_owned();
    Ok(Container {
        id,
        image: image_name,
    })
}

fn stop_containers(ids: &[String]) -> anyhow::Result<()> {
    for id in ids {
        Command::new("docker")
            .arg("stop")
            .arg(id)
            .output()
            .with_context(|| format!("failed to stop container with id '{id}'"))?;
        let _ = Command::new("docker").arg("rm").arg(id).output();
    }
    Ok(())
}
