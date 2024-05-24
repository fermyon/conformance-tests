pub mod config;

use anyhow::Context as _;
use std::path::{Path, PathBuf};

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
pub fn tests(tests_dir: &Path) -> anyhow::Result<impl Iterator<Item = Test>> {
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

            let component_name = format!("{name}.wasm");
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

pub struct Test {
    pub name: String,
    pub config: config::TestConfig,
    pub manifest: PathBuf,
    pub component: PathBuf,
}
