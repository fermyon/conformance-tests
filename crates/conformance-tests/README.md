# Conformance Tests

Helpers for running conformance tests.

## Example

```rust
fn main() -> anyhow::Result<()> {
    // Download the "canary" tests and run each of them with `run_test`.
    // Use `Config::ignore` to skip tests the runtime does not support yet.
    let config = conformance_tests::Config::new("canary");
    let conclusion = conformance_tests::run_tests(config, run_test)?;
    conclusion.exit_if_failed();
    Ok(())
}

fn run_test(test: conformance_tests::Test) -> anyhow::Result<()> {
    // TODO: Here is where the specific runtime being tested would do any set up

    // Loop over each app invocation
    for invocation in test.config.invocations {
        let conformance_tests::config::Invocation::Http(invocation) = invocation;
        // Run the invocation which asserts that the response matches what we expect
        invocation.run(|request| {
            todo!("here the runtime must produce a `Response` given the given `request`");
        })?;
    }
    Ok(())
}
```

To manage the download yourself, call `download_tests(version)` and iterate
the result with `tests_iter(dir)`.
