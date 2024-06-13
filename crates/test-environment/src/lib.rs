pub mod http;
pub mod io;
pub mod manifest_template;
pub mod services;
pub mod test_environment;

// A runtime which can be tested
pub trait Runtime {
    /// Return an error if the runtime has errored
    fn error(&mut self) -> anyhow::Result<()>;
}

#[doc(inline)]
pub use test_environment::{TestEnvironment, TestEnvironmentConfig};
