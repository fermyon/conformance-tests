use std::{collections::HashMap, path::Path};

use crate::{
    services::{Services, ServicesConfig},
    Runtime,
};
use anyhow::Context as _;

/// A callback to create a runtime given a path to a temporary directory and a set of services
pub type RuntimeCreator<R> = dyn FnOnce(&mut TestEnvironment<R>) -> anyhow::Result<R>;

/// An environment for running tests
pub struct TestEnvironment<R> {
    temp: temp_dir::TempDir,
    services: Services,
    runtime: Option<R>,
    env_vars: HashMap<String, String>,
}

impl<R: Runtime> TestEnvironment<R> {
    /// Spin up a test environment with a runtime
    ///
    /// `config` specifies the services to run and how to create the runtime.
    pub fn up(
        config: TestEnvironmentConfig<R>,
        init_env: impl FnOnce(&mut Self) -> anyhow::Result<()> + 'static,
    ) -> anyhow::Result<Self> {
        let mut env = Self::boot(&config.services_config)?;
        init_env(&mut env)?;
        let runtime = (config.create_runtime)(&mut env)?;
        env.start_runtime(runtime)
    }

    /// Returns an error if the environment is not healthy.
    ///
    /// If a runtime is present, it will also be checked for errors.
    fn error(&mut self) -> anyhow::Result<()> {
        self.services.healthy()?;
        if let Some(runtime) = &mut self.runtime {
            runtime.error()?;
        }
        Ok(())
    }
}

impl<R> TestEnvironment<R> {
    /// Spin up a test environment without a runtime
    ///
    /// `services` specifies the services to run.
    pub fn boot(services: &ServicesConfig) -> anyhow::Result<Self> {
        let temp = temp_dir::TempDir::new()
            .context("failed to produce a temporary directory to run the test in")?;
        let mut services =
            Services::start(services, temp.path()).context("failed to start services")?;
        services.healthy().context("services have failed")?;
        Ok(Self {
            temp,
            services,
            runtime: None,
            env_vars: HashMap::new(),
        })
    }

    /// Start the runtime
    ///
    /// Will error if the environment is not healthy.
    pub fn start_runtime<N: Runtime>(self, runtime: N) -> anyhow::Result<TestEnvironment<N>> {
        let mut this = TestEnvironment {
            temp: self.temp,
            services: self.services,
            runtime: Some(runtime),
            env_vars: self.env_vars,
        };
        this.error().context("testing environment is not healthy")?;
        Ok(this)
    }

    /// Get the services that are running for the test
    pub fn services_mut(&mut self) -> &mut Services {
        &mut self.services
    }

    /// Get the runtime that is running for the test
    pub fn runtime_mut(&mut self) -> &mut R {
        self.runtime
            .as_mut()
            .expect("runtime has not been initialized")
    }

    /// Copy a file into the test environment at the given relative path
    pub fn copy_into(&self, from: impl AsRef<Path>, into: impl AsRef<Path>) -> anyhow::Result<()> {
        fn copy_dir_all(from: &Path, into: &Path) -> anyhow::Result<()> {
            std::fs::create_dir_all(into)?;
            for entry in std::fs::read_dir(from)? {
                let entry = entry?;
                let ty = entry.file_type()?;
                if ty.is_dir() {
                    copy_dir_all(&entry.path(), &into.join(entry.file_name()))?;
                } else {
                    std::fs::copy(entry.path(), into.join(entry.file_name()))?;
                }
            }
            Ok(())
        }
        let from = from.as_ref();
        let into = into.as_ref();
        if from.is_dir() {
            copy_dir_all(from, &self.temp.path().join(into)).with_context(|| {
                format!(
                    "failed to copy directory '{}' to temporary directory",
                    from.display()
                )
            })?;
        } else {
            std::fs::copy(from, self.temp.path().join(into)).with_context(|| {
                format!(
                    "failed to copy file '{}' to temporary directory",
                    from.display()
                )
            })?;
        }
        Ok(())
    }

    /// Get the host port that is mapped to the given guest port
    pub fn get_port(&mut self, guest_port: u16) -> anyhow::Result<Option<u16>> {
        self.services.get_port(guest_port)
    }

    /// Write a file into the test environment at the given relative path
    pub fn write_file(
        &self,
        to: impl AsRef<Path>,
        contents: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        std::fs::write(self.temp.path().join(to), contents)?;
        Ok(())
    }

    /// Read a file from the test environment at the given relative path
    pub fn read_file(&self, path: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
        let path = path.as_ref();
        std::fs::read(self.temp.path().join(path))
            .with_context(|| format!("failed to read file '{}'", path.display()))
    }

    /// Run a command in the test environment
    ///
    /// This blocks until the command has finished running and will error if the command fails
    pub fn run_in(&self, cmd: &mut std::process::Command) -> anyhow::Result<std::process::Output> {
        let output = cmd
            .current_dir(self.temp.path())
            // TODO: figure out how not to hardcode this
            // We do this so that if `spin build` is run with a Rust app,
            // it builds inside the test environment
            .env("CARGO_TARGET_DIR", self.path().join("target"))
            .output()?;
        if !output.status.success() {
            anyhow::bail!(
                "'{cmd:?}' failed with status code {:?}\nstdout:\n{}\nstderr:\n{}\n",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(output)
    }

    /// Get the path to test environment
    pub fn path(&self) -> &Path {
        self.temp.path()
    }

    /// Set an environment variable in the test environment
    pub fn set_env_var(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env_vars.insert(key.into(), value.into());
    }

    /// Get the environment variables in the test environment
    pub fn env_vars(&self) -> &HashMap<String, String> {
        &self.env_vars
    }
}

/// Configuration for a test environment
pub struct TestEnvironmentConfig<R> {
    /// A callback to create a runtime given a path to a temporary directory
    pub create_runtime: Box<RuntimeCreator<R>>,
    /// The services that the test requires
    pub services_config: ServicesConfig,
}
