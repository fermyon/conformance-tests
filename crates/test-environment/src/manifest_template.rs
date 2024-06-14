use anyhow::Context as _;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::TestEnvironment;

/// A template with variables that can be substituted with information from the testing environment.
pub struct EnvTemplate {
    content: String,
}

static TEMPLATE_REGEX: OnceLock<regex::Regex> = OnceLock::new();
impl EnvTemplate {
    /// Instantiate a template.
    pub fn new(content: String) -> anyhow::Result<Self> {
        Ok(Self { content })
    }

    /// Read a template from a file.
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("could not read template at '{}'", path.display()))?;
        Ok(Self { content })
    }

    /// Substitute template variables in the template.
    pub fn substitute<R>(
        &mut self,
        env: &mut TestEnvironment<R>,
        path_for: impl Fn(&str) -> Option<PathBuf>,
    ) -> Result<(), anyhow::Error> {
        let regex = TEMPLATE_REGEX.get_or_init(|| regex::Regex::new(r"%\{(.*?)\}").unwrap());
        while let Some(captures) = regex.captures(&self.content) {
            let (Some(full), Some(capture)) = (captures.get(0), captures.get(1)) else {
                continue;
            };
            let template = capture.as_str();
            let (template_key, template_value) = template.split_once('=').with_context(|| {
                format!("invalid template '{template}'(template should be in the form $KEY=$VALUE)")
            })?;
            let replacement = match template_key.trim() {
                "source" => {
                    let component_binary = path_for(template_value)
                        .with_context(|| format!("no such component '{template_value}'"))?;
                    let wasm_name = component_binary.file_name().unwrap().to_str().unwrap();
                    env.copy_into(&component_binary, wasm_name)?;
                    wasm_name.to_owned()
                }
                "port" => {
                    let guest_port = template_value
                        .parse()
                        .with_context(|| format!("failed to parse '{template_value}' as port"))?;
                    let port = env
                        .get_port(guest_port)?
                        .with_context(|| format!("no port {guest_port} exposed by any service"))?;
                    port.to_string()
                }
                _ => {
                    anyhow::bail!("unknown template key: {template_key}");
                }
            };
            self.content.replace_range(full.range(), &replacement);
        }
        Ok(())
    }

    pub fn substitute_value(
        &mut self,
        key: &str,
        replacement: impl Fn(&str) -> Option<String>,
    ) -> anyhow::Result<()> {
        replace_template(&mut self.content, |k, v| {
            if k == key {
                Ok(replacement(v))
            } else {
                Ok(None)
            }
        })
    }

    /// Get the contents of the template.
    pub fn contents(&self) -> &str {
        &self.content
    }

    /// Consume the template and return its contents.
    pub fn into_contents(self) -> String {
        self.content
    }
}

/// Replace template variables in a string.
///
/// Every time a template is found, the `replacement` function is called with the template key and value.
pub fn replace_template(
    content: &mut String,
    mut replacement: impl FnMut(&str, &str) -> anyhow::Result<Option<String>>,
) -> Result<(), anyhow::Error> {
    let regex = TEMPLATE_REGEX.get_or_init(|| regex::Regex::new(r"%\{(.*?)\}").unwrap());
    'outer: loop {
        'inner: for captures in regex.captures_iter(content) {
            let (Some(full), Some(capture)) = (captures.get(0), captures.get(1)) else {
                continue 'inner;
            };
            let template = capture.as_str();
            let (template_key, template_value) = template.split_once('=').with_context(|| {
                format!("invalid template '{template}'(template should be in the form $KEY=$VALUE)")
            })?;
            let (template_key, template_value) = (template_key.trim(), template_value.trim());
            if let Some(replacement) = replacement(template_key, template_value)? {
                content.replace_range(full.range(), &replacement);
                // Restart the search after a substitution
                continue 'outer;
            }
        }
        // Break the outer loop if no substitutions were made
        break 'outer;
    }
    Ok(())
}
