use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

mod docker;
mod python;

use anyhow::{bail, Context};

use docker::DockerService;
use python::PythonService;

pub use docker::DockerImage;

/// All the services that are running for a test.
#[derive(Default)]
pub struct Services {
    services: Vec<Box<dyn Service>>,
}

impl Services {
    /// Start all the required services given a path to service definitions
    pub fn start(config: ServicesConfig, working_dir: &Path) -> anyhow::Result<Self> {
        let lock_dir = working_dir.join(".service-locks");
        std::fs::create_dir(&lock_dir).context("could not create service lock dir")?;
        let mut services = Vec::new();
        for service_def in config.service_definitions {
            let mut service: Box<dyn Service> = match service_def.kind {
                ServiceKind::Python { script } => Box::new(PythonService::start(
                    &service_def.name,
                    &script,
                    working_dir,
                    &lock_dir,
                )?),
                ServiceKind::Docker { image } => {
                    Box::new(DockerService::start(&service_def.name, image, &lock_dir)?)
                }
            };
            service.ready()?;
            services.push(service);
        }

        Ok(Services { services })
    }

    /// Block all services until they are ready.
    ///
    /// Returns an error if any service is in a bad state.
    pub fn healthy(&mut self) -> anyhow::Result<()> {
        for service in &mut self.services {
            service.ready()?;
        }
        Ok(())
    }

    /// Get the host port that one of the services exposes a guest port on.
    pub fn get_port(&mut self, guest_port: u16) -> anyhow::Result<Option<u16>> {
        let mut previous_result = None;
        for service in &mut self.services {
            let next_result = service.ports().unwrap().get(&guest_port);
            match (previous_result, next_result) {
                // If we haven't yet found a port, store the lookup in `result`.
                (None, next_result) => {
                    previous_result = next_result.copied().map(|p| (service.name(), p))
                }
                // If a service already exposed the port, and we found it again, error.
                (Some((name, _)), Some(_)) => {
                    anyhow::bail!("service '{name}' already exposes port {guest_port} to the host");
                }
                // If a previous service exposed the port, but the next service doesn't, just continue.
                (Some(_), None) => {}
            }
        }
        Ok(previous_result.map(|(_, p)| p))
    }
}

impl<'a> IntoIterator for &'a Services {
    type Item = &'a Box<dyn Service>;
    type IntoIter = std::slice::Iter<'a, Box<dyn Service>>;

    fn into_iter(self) -> Self::IntoIter {
        self.services.iter()
    }
}

pub struct ServicesConfig {
    /// Definitions of all services to be used.
    service_definitions: Vec<ServiceDefinition>,
}

impl ServicesConfig {
    /// Create a new services config with a list of built-in services to start.
    ///
    /// The built-in services are expected to have a definition file in the `services` directory with the same name as the service.
    pub fn new<'a>(builtins: impl Into<Vec<&'a str>>) -> anyhow::Result<Self> {
        let definitions_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("services");
        let service_definitions = get_builtin_service_definitions(
            builtins.into().into_iter().collect(),
            &definitions_path,
        )?;
        Ok(Self {
            service_definitions,
        })
    }

    pub fn add_service(&mut self, service: ServiceDefinition) {
        self.service_definitions.push(service);
    }

    /// Configure no services
    pub fn none() -> Self {
        Self {
            service_definitions: Vec::new(),
        }
    }
}

/// Get all of the service definitions returning a HashMap of the service name to the service definition file extension.
fn get_builtin_service_definitions(
    mut builtins: HashSet<&str>,
    service_definitions_path: &Path,
) -> anyhow::Result<Vec<ServiceDefinition>> {
    if builtins.is_empty() {
        return Ok(Vec::new());
    }

    let result = std::fs::read_dir(service_definitions_path)
        .with_context(|| {
            format!(
                "no service definitions found at '{}'",
                service_definitions_path.display()
            )
        })?
        .map(|d| {
            let d = d?;
            if !d.file_type()?.is_file() {
                bail!("directories are not allowed in the service definitions directory")
            }
            let file_name = d.file_name();
            let file_name = file_name.to_str().unwrap();
            let (file_name, file_extension) = file_name
                .find('.')
                .map(|i| (&file_name[..i], &file_name[i + 1..]))
                .context("service definition did not have an extension")?;
            Ok((file_name.to_owned(), file_extension.to_owned()))
        })
        .filter(|r| !matches!(r, Ok((_, extension)) if extension == "lock"))
        .filter(|r| match r {
            Ok((service, _)) => builtins.remove(service.as_str()),
            _ => false,
        })
        .map(|r| {
            let (name, extension) = r?;
            Ok(ServiceDefinition {
                name: name.clone(),
                kind: match extension.as_str() {
                    "py" => ServiceKind::Python {
                        script: service_definitions_path.join(format!("{name}.py")),
                    },
                    "Dockerfile" => ServiceKind::Docker {
                        image: docker::DockerImage::FromDockerfile(
                            service_definitions_path.join(format!("{name}.Dockerfile")),
                        ),
                    },
                    _ => bail!("unsupported service definition extension '{extension}'"),
                },
            })
        })
        .collect();
    if !builtins.is_empty() {
        bail!("no service definitions found for: {builtins:?}",);
    }
    result
}

/// A service definition.
pub struct ServiceDefinition {
    pub name: String,
    pub kind: ServiceKind,
}

/// The kind of service.
pub enum ServiceKind {
    Python { script: PathBuf },
    Docker { image: DockerImage },
}

/// An external service a test may depend on.
pub trait Service {
    /// The name of the service.
    fn name(&self) -> &str;

    /// Block until the service is ready and error if service is in bad state.
    fn ready(&mut self) -> anyhow::Result<()>;

    /// Get a mapping of ports that the service exposes.
    fn ports(&mut self) -> anyhow::Result<&HashMap<u16, u16>>;
}
