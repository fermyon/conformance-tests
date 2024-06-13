# Conformance Testing

## What is conformance testing?

Conformance testing is a way of specifying the set of behavior that Spin compliant runtimes *must* implement. While we refrain from any formal specification (as such an activity at this point would likely be *much* more work than is worthwhile), conformance testing is the start of the journey in the ultimate goal of clearly defining what a Spin compliant runtime is. 

## What are we testing, and what are we not testing?

Conformance tests would include assertions on all behaviors that a Spin compliant runtime must exhibit *from the perspective of the guest application*. Conformance tests do not aim to verify the all around correctness of a runtime, nor do they seek to make assertions about all runtime semantics, but rather they seek to test behaviors that all runtimes must guarantee to a Spin application. 

In other words, conformance tests answer the question: **what behavior which is visible to a guest Spin application must a runtime provide?**

At a high-level, this is a non-exhaustive list of the category of items a conformance test will want to test:

- The Spin *world* (i.e., set of required imports and exports) that a component can have or is required to have to run (e.g., what set of imports are all Spin applications allowed to rely on? What trigger export can they expose and be guaranteed to run?).
- The input to the various trigger exports (e.g., the `incoming-request` to the HTTP trigger must have a `spin-path-info` header set)
- Basic behavior of the imports to the guest application (e.g., if I get `none` for a key from the `key-value` store, then set that key with a value, and then get the key again, I should expect to see the value set.)

At a high-level, this is a non-exhaustive list of the category of items a conformance test will *not* test:

- Behavior that is not visible to the guest (e.g., host logging)
- Runtime configuration is a specific class of semantics not visible to the guest that keep the same semantics from the guest perspective, but change some runtime semantics (e.g., key-value store can be changed from local storage to some network storage solution)
- Transient errors (i.e., errors that are not conditional on reproducible state such as network outage) are also not tested. Transient errors are more than likely to be runtime dependent and thus not in scope for conformance testing.

## What do conformance tests include:

Conformance tests seek to answer the question of what the guest application can rely on. In order to answer this question, a spin.toml must be provided as guest visible capabilities are defined in the spin.toml manifest.

Therefore conformance tests will be composed of the following:

- A guest Spin application
- A spin.toml manifest
- A conformance test manifest (which specifies the following):
    - how the conformance test is triggered
    - what the success condition of the trigger is
    - what abstract external services the conformance test relies on
    - what state the abstract external services are required to be in

For more information on the conformance test manifest, see [the docs here](./docs/test-manifest.md).

## Running Tests

The conformance test suite does not provide a way to run the tests by default. Each Spin compliant runtime is different enough in structure that providing a test suite runner that can handle all of them is likely not possible. At the very least, this is out of scope for the near term. 

This means each runtime will have to provide their own test runner.

## Helper Crates

The crates found in the `crates` directory provide functionality related to conformance testing:
* `conformance-tests`: helpers for downloading and running the conformance test suite.
* `test-environment`: a framework for building a conformance test runner using a test environment
