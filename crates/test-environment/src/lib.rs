mod io;
mod test_environment;

pub mod http;
pub mod services;

// A runtime which can be tested
pub trait Runtime {
    /// Return an error if the runtime has errored
    fn error(&mut self) -> anyhow::Result<()>;
}

#[doc(inline)]
pub use test_environment::{RuntimeCreator, TestEnvironment, TestEnvironmentConfig};
