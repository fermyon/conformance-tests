# Conformance Tests

Helpers for running conformance tests.

## Example

```rust
// Download the tests
let tests_dir = conformance_tests::download_tests()?;
// Loop over the tests
for test in conformance_tests::tests(&tests_dir)? {
    // TODO: Here is where the specific runtime being tested would do any set up

    // Loop over each app invocation
    for invocation in test.config.invocations {
        let conformance_tests::config::Invocation::Http(invocation) = invocation;
        // Run the invocation which asserts that the response matches what we expect
        invocation
            .run(|request| {
                todo!("here the runtime must produce a `Response` given the given `request`");
            })?;
    }
}
Ok(())
```