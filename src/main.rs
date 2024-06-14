fn main() {
    let Some(command) = std::env::args().nth(1) else {
        eprintln!("Usage: test-runner <command>");
        std::process::exit(1);
    };
    let result = match command.as_str() {
        "archive" => archive(),
        "package" => {
            let dir = std::env::args()
                .nth(2)
                .unwrap_or_else(|| "conformance-tests".into());
            std::fs::create_dir(&dir)
                .context("failed to create dir")
                .and_then(|_| {
                    package_into(dir)?;
                    Ok(())
                })
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        print_error_chain(e);
        std::process::exit(1);
    }
}

fn print_error_chain(err: anyhow::Error) {
    if let Some(cause) = err.source() {
        let is_multiple = cause.source().is_some();
        eprintln!("\nCaused by:");
        for (i, err) in err.chain().skip(1).enumerate() {
            if is_multiple {
                eprintln!("{i:>4}: {}", err)
            } else {
                eprintln!("      {}", err)
            }
        }
    }
}

use anyhow::Context;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use tar::Builder;
use tempfile::tempdir;

fn archive() -> anyhow::Result<()> {
    // Ensure the program exits if any command fails
    let output_tar = "tests.tar.gz";

    // Create a temporary directory to store the files to be archived
    let temp_dir = tempdir().context("failed to create temp directory")?;
    let temp_dir_path = temp_dir.path();
    std::fs::File::create(temp_dir_path.join(".gitkeep"))
        .context("failed to create .gitkeep in temp directory")?;

    package_into(temp_dir_path)?;

    // Create the tarball from the temporary directory
    let tar_gz = File::create(output_tar)?;
    let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
    let mut tar = Builder::new(enc);
    tar.append_dir_all(".", temp_dir_path)?;

    println!("Tarball created: {}", output_tar);
    Ok(())
}

/// Packages the components and tests into the provided directory
fn package_into(dir_path: impl AsRef<Path>) -> anyhow::Result<()> {
    let mut components = HashMap::new();
    for entry in std::fs::read_dir("components").context("failed to read 'components' directory")? {
        let component_dir = entry.context("failed to read a component directory")?;
        if !component_dir.path().is_dir() {
            continue;
        }
        let component_dir_path = component_dir.path();
        let component_name = component_dir_path
            .file_name()
            .context("could not determine component name")?
            .to_str()
            .context("could not convert component name to string")?;
        println!("Building component {component_name:?}...",);
        // Build the test, and copy the build artifact to the temporary directory
        let status = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--target=wasm32-unknown-unknown",
                "--target-dir=target",
            ])
            .current_dir(&component_dir_path)
            .status()?;
        anyhow::ensure!(status.success(), "Failed to build component");

        let release_dir = component_dir_path.join("target/wasm32-unknown-unknown/release");
        let wasm_artifact = find_wasm_file(&release_dir)
            .context("error when trying to find build artifact")?
            .context("failed to find wasm artifact in target directory")?;

        components.insert(component_name.to_owned(), wasm_artifact);
    }
    for entry in std::fs::read_dir("tests").unwrap() {
        let test = entry.unwrap();
        if !test.path().is_dir() {
            continue;
        }
        let test_path = test.path();
        println!("Processing test {:?}...", test_path);

        let test_name = test_path
            .file_name()
            .context("could not determine test name")?;

        let test_archive = dir_path.as_ref().join(test_name);
        std::fs::create_dir_all(&test_archive).context("failed to create component directory")?;

        // Copy the configuration and manifest files to the temporary directory
        std::fs::copy(
            test_path.join("test.json5"),
            test_archive.join("test.json5"),
        )
        .context("failed to copy test manifest to temp directory")?;
        let mut manifest = std::fs::read_to_string(test_path.join("spin.toml"))
            .context("failed to read spin manifest")?;
        substitute_source(&mut manifest, &components, &test_archive)
            .context("failed to substitute component template for actual component binary")?;
        std::fs::write(test_archive.join("spin.toml"), manifest.as_bytes())
            .context("failed to copy spin manifest to temp directory")?;
    }

    Ok(())
}

fn find_wasm_file(dir: &Path) -> anyhow::Result<Option<PathBuf>> {
    for entry in std::fs::read_dir(dir).context("failed to read directory")? {
        let entry = entry.context("failed to read entry")?;
        if !entry.path().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) == Some("wasm") {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

/// Substitute templated "source" in the spin.toml manifest
///
/// Also writes the component binary into the test archive.
fn substitute_source(
    manifest: &mut String,
    components: &HashMap<String, PathBuf>,
    test_archive: &Path,
) -> anyhow::Result<()> {
    static TEMPLATE_REGEX: OnceLock<regex::Regex> = OnceLock::new();
    let regex = TEMPLATE_REGEX.get_or_init(|| regex::Regex::new(r"%\{(.*?)\}").unwrap());
    'outer: loop {
        for captures in regex.captures_iter(manifest) {
            let (Some(full), Some(capture)) = (captures.get(0), captures.get(1)) else {
                continue;
            };
            let template = capture.as_str();
            let (template_key, template_value) = template.split_once('=').with_context(|| {
                format!("invalid template '{template}'(template should be in the form $KEY=$VALUE)")
            })?;
            let (template_key, template_value) = (template_key.trim(), template_value.trim());
            if "source" == template_key {
                let path = components
                    .get(template_value)
                    .with_context(|| format!("'{template_value}' is not a known component"))?;
                let component_file = "component.wasm";
                std::fs::copy(path, test_archive.join(component_file))?;
                println!("Substituting {template} with {component_file}...");
                manifest.replace_range(full.range(), component_file);
                // Restart the search after a substitution
                continue 'outer;
            }
        }
        // Break the outer loop if no substitutions were made
        break 'outer;
    }
    Ok(())
}
