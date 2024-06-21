pub mod config;

use anyhow::Context as _;
use std::path::{Path, PathBuf};

/// Run the conformance tests
pub fn run_tests(
    run: impl Fn(Test) -> anyhow::Result<()> + Send + Clone + 'static,
) -> anyhow::Result<()> {
    let tests_dir = download_tests()?;
    run_tests_from(tests_dir, run)
}

/// Run the conformance tests located in the given directory
pub fn run_tests_from(
    tests_dir: impl AsRef<Path>,
    run: impl Fn(Test) -> anyhow::Result<()> + Send + Clone + 'static,
) -> anyhow::Result<()> {
    let trials = tests_iter(tests_dir)?
        .map(|test| {
            let run = run.clone();
            libtest_mimic::Trial::test(test.name.clone(), move || {
                Ok(run(test).map_err(FullError::from)?)
            })
        })
        .collect();
    libtest_mimic::run(&Default::default(), trials).exit();
}

/// Download the conformance tests and return the path to the directory where they are written to
pub fn download_tests() -> anyhow::Result<std::path::PathBuf> {
    let response = reqwest::blocking::get(
        "https://github.com/fermyon/conformance-tests/releases/download/canary/tests.tar.gz",
    )
    .context("failed to send request")?
    .error_for_status()?;
    let response = flate2::read::GzDecoder::new(response);
    let dir = std::env::temp_dir().join("conformance-tests");
    for entry in tar::Archive::new(response)
        .entries()
        .context("failed to read archive")?
    {
        let mut entry = entry.context("failed to read archive entry")?;
        if entry.header().entry_type() != tar::EntryType::Regular {
            continue;
        }
        let path = dir.join(entry.path().context("failed to get archive entry's path")?);
        let parent_dir = path
            .parent()
            .expect("somehow archived file has no parent dir");
        std::fs::create_dir_all(parent_dir).context("failed to create directory from archive")?;
        let mut file =
            std::fs::File::create(&path).context("failed to create file from archive")?;
        std::io::copy(&mut entry, &mut file).context("failed to copy file from archive")?;
    }
    Ok(dir)
}

/// Read the tests directory and get an iterator to each test's directory
///
/// The test directory can be downloaded using the `download_tests` function.
pub fn tests_iter(tests_dir: impl AsRef<Path>) -> anyhow::Result<impl Iterator<Item = Test>> {
    // Like `?` but returns error wrapped in `Some` for use in `filter_map`
    macro_rules! r#try {
        ($e:expr) => {
            match $e {
                Ok(e) => e,
                Err(e) => return Some(Err(e.into())),
            }
        };
    }
    let items = std::fs::read_dir(tests_dir)?
        .filter_map(|entry| {
            let e = r#try!(entry);
            if !e.path().is_dir() {
                return None;
            }
            let test_dir = e.path();
            let name = r#try!(test_dir
                .file_name()
                .and_then(|f| f.to_str())
                .context("could not determine test name"))
            .to_owned();
            let config = r#try!(std::fs::read_to_string(test_dir.join("test.json5"))
                .context("failed to read test config"));
            let config = r#try!(json5::from_str::<config::TestConfig>(&config)
                .context("test config could not be parsed"));

            let component_name = "component.wasm";
            Some(Ok(Test {
                name,
                config,
                manifest: test_dir.join("spin.toml"),
                component: test_dir.join(component_name),
            }))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(items.into_iter())
}

#[derive(Debug, Clone)]
pub struct Test {
    pub name: String,
    pub config: config::TestConfig,
    pub manifest: PathBuf,
    pub component: PathBuf,
}

pub mod assertions {
    use crate::indent_lines;

    use super::config::Response as ExpectedResponse;
    use test_environment::http::Response as ActualResponse;

    /// Assert that the actual response matches the expected response
    pub fn assert_response(
        expected: &ExpectedResponse,
        actual: &ActualResponse,
    ) -> anyhow::Result<()> {
        // We assert the status code first, because if it's wrong, the body and headers are likely wrong
        anyhow::ensure!(
            actual.status() == expected.status,
            "actual status {} != expected status {} - body: {}",
            actual.status(),
            expected.status,
            indent_lines(
                &actual
                    .text()
                    .unwrap_or_else(|_| String::from("<invalid utf-8>")),
                2
            )
        );

        // We assert the body next, because if it's wrong, it usually has more information as to why
        let expected_body = expected.body.as_deref().unwrap_or_default();
        let actual_body = actual
            .text()
            .unwrap_or_else(|_| String::from("<invalid utf-8>"));

        anyhow::ensure!(
            actual_body == expected_body,
            "actual body != expected body\nactual: {actual_body}\nexpected: {expected_body}",
            actual_body = indent_lines(&actual_body, 2),
            expected_body = indent_lines(expected_body, 2)
        );

        let mut actual_headers = actual
            .headers()
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.to_lowercase()))
            .collect::<std::collections::HashMap<_, _>>();
        for expected_header in &expected.headers {
            let expected_name = expected_header.name.to_lowercase();
            let expected_value = expected_header.value.as_ref().map(|v| v.to_lowercase());
            let actual_value = actual_headers.remove(&expected_name);
            let Some(actual_value) = actual_value.as_deref() else {
                if expected_header.optional {
                    continue;
                } else {
                    anyhow::bail!(
                        "expected header '{name}' not found in response",
                        name = expected_header.name
                    )
                }
            };
            if let Some(expected_value) = expected_value {
                anyhow::ensure!(
                    actual_value == expected_value,
                    "header '{name}' has unexpected value '{actual_value}' != '{expected_value}'",
                    name = expected_header.name
                );
            }
        }
        if !actual_headers.is_empty() {
            anyhow::bail!("unexpected headers: {actual_headers:?}");
        }

        Ok(())
    }
}

/// A wrapper around `anyhow::Error` that prints the full chain of causes
struct FullError {
    error: anyhow::Error,
}

impl From<anyhow::Error> for FullError {
    fn from(error: anyhow::Error) -> Self {
        Self { error }
    }
}

impl std::fmt::Display for FullError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", indent_lines(&self.error.to_string(), 2))?;
        write_error_chain(f, &self.error)?;
        Ok(())
    }
}

fn write_error_chain(mut f: impl std::fmt::Write, err: &anyhow::Error) -> std::fmt::Result {
    let Some(cause) = err.source() else {
        return Ok(());
    };
    let is_multiple = cause.source().is_some();
    writeln!(f, "\nCaused by:")?;
    for (i, err) in err.chain().skip(1).enumerate() {
        let err = indent_lines(&err.to_string(), 6);
        if is_multiple {
            writeln!(f, "{i:>4}: {err}")?;
        } else {
            writeln!(f, "      {err}")?;
        }
    }
    Ok(())
}

/// Format string such that all lines after the first are indented
fn indent_lines(str: &str, indent: usize) -> String {
    str.lines()
        .enumerate()
        .map(|(i, line)| {
            let indent = if i == 0 { 0 } else { indent };
            format!("{}{}", " ".repeat(indent), line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
