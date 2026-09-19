//! Flattening an ACP prompt into the text Ante takes as user input.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_client_protocol::schema::v1::{ContentBlock, EmbeddedResourceResource};
use anyhow::{Context, Result};
use base64::Engine;
use tracing::warn;

/// Where client images land; Ante's own TUI pastes into the same directory.
pub fn image_cache_dir() -> PathBuf {
    std::env::temp_dir().join("ante-paste-cache")
}

/// Flatten `blocks` into one input text. Files the host can read itself
/// become `@path` mentions, images are written to `cache_dir` and mentioned
/// the same way, and embedded text is quoted inline.
pub fn flatten(blocks: Vec<ContentBlock>, cache_dir: &Path) -> Result<String> {
    let mut text = Text::default();
    for block in blocks {
        match block {
            ContentBlock::Text(content) => text.push(&content.text),
            ContentBlock::ResourceLink(link) => match file_path(&link.uri) {
                Some(path) => text.push_mention(&path),
                None => text.push_token(&link.uri),
            },
            ContentBlock::Resource(resource) => match resource.resource {
                EmbeddedResourceResource::TextResourceContents(contents) => {
                    text.push(&format!("\n{}:\n```\n{}\n```\n", contents.uri, contents.text));
                }
                EmbeddedResourceResource::BlobResourceContents(contents) => {
                    warn!(uri = %contents.uri, "ignoring an embedded binary resource");
                }
                _ => warn!("ignoring an unknown embedded resource"),
            },
            ContentBlock::Image(image) => {
                let path = write_image(cache_dir, &image.data, &image.mime_type)?;
                text.push_mention(&path.display().to_string());
            }
            _ => warn!("ignoring unsupported prompt content"),
        }
    }
    Ok(text.0)
}

/// Input text under construction. Ante reads a mention up to the next
/// unescaped whitespace, so a token gets a space on either side unless the
/// neighbouring text already provides one.
#[derive(Default)]
struct Text(String, bool);

impl Text {
    fn push(&mut self, text: &str) {
        if self.1 && !text.starts_with(char::is_whitespace) {
            self.0.push(' ');
        }
        self.0.push_str(text);
        self.1 = false;
    }

    fn push_token(&mut self, token: &str) {
        if !self.0.is_empty() && !self.0.ends_with(char::is_whitespace) {
            self.0.push(' ');
        }
        self.0.push_str(token);
        self.1 = true;
    }

    /// Whitespace inside the path is escaped so Ante reads it as one token.
    fn push_mention(&mut self, path: &str) {
        let mut token = String::from("@");
        for ch in path.chars() {
            if ch.is_whitespace() {
                token.push('\\');
            }
            token.push(ch);
        }
        self.push_token(&token);
    }
}

/// The local path a `file://` URI names, percent-decoded. A URI with a host
/// other than `localhost` is not local.
fn file_path(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    (authority.is_empty() || authority == "localhost").then(|| percent_decode(path))
}

fn percent_decode(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let escaped = (bytes[i] == b'%' && i + 2 < bytes.len())
            .then(|| std::str::from_utf8(&bytes[i + 1..i + 3]).ok())
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                i += 3;
            }
            None => {
                decoded.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn write_image(cache_dir: &Path, data: &str, mime_type: &str) -> Result<PathBuf> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .context("image data is not valid base64")?;
    let extension = match mime_type.strip_prefix("image/") {
        Some("jpeg") => "jpg",
        Some(subtype) if !subtype.is_empty() => subtype,
        _ => "img",
    };
    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("could not create {}", cache_dir.display()))?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let path = cache_dir.join(format!("acp-{}-{stamp}.{extension}", std::process::id()));
    std::fs::write(&path, bytes).with_context(|| format!("could not write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{
        EmbeddedResource, ImageContent, ResourceLink, TextContent, TextResourceContents,
    };

    fn cache() -> PathBuf {
        std::env::temp_dir().join(format!("ante-acp-prompt-test-{}", std::process::id()))
    }

    fn text(s: &str) -> ContentBlock {
        ContentBlock::Text(TextContent::new(s))
    }

    fn link(uri: &str) -> ContentBlock {
        ContentBlock::ResourceLink(ResourceLink::new("name", uri))
    }

    #[test]
    fn text_and_file_links_interleave_with_spacing() {
        let blocks = vec![text("explain"), link("file:///src/main.rs"), text("please")];
        assert_eq!(flatten(blocks, &cache()).unwrap(), "explain @/src/main.rs please");

        let blocks = vec![text("explain "), link("file:///src/main.rs"), text(" please")];
        assert_eq!(flatten(blocks, &cache()).unwrap(), "explain @/src/main.rs please");
    }

    #[test]
    fn file_uris_are_decoded_and_whitespace_escaped() {
        let blocks = vec![link("file:///my%20dir/f%C3%BCr%09x.rs")];
        assert_eq!(flatten(blocks, &cache()).unwrap(), "@/my\\ dir/für\\\tx.rs");
    }

    #[test]
    fn file_uri_hosts_are_local_only_for_localhost() {
        let blocks = vec![link("file://localhost/etc/hosts")];
        assert_eq!(flatten(blocks, &cache()).unwrap(), "@/etc/hosts");

        let blocks = vec![link("file://nas/share/doc.md")];
        assert_eq!(flatten(blocks, &cache()).unwrap(), "file://nas/share/doc.md");
    }

    #[test]
    fn other_links_stay_as_urls() {
        let blocks = vec![text("see"), link("https://example.com/doc")];
        assert_eq!(flatten(blocks, &cache()).unwrap(), "see https://example.com/doc");
    }

    #[test]
    fn embedded_text_is_quoted_with_its_uri() {
        let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
            "let x = 1;",
            "file:///a.rs",
        ));
        let blocks =
            vec![text("fix this"), ContentBlock::Resource(EmbeddedResource::new(resource))];
        assert_eq!(
            flatten(blocks, &cache()).unwrap(),
            "fix this\nfile:///a.rs:\n```\nlet x = 1;\n```\n"
        );
    }

    #[test]
    fn images_are_written_to_the_cache_and_mentioned() {
        let data = base64::engine::general_purpose::STANDARD.encode(b"not really a png");
        let blocks =
            vec![text("what is this"), ContentBlock::Image(ImageContent::new(data, "image/png"))];

        let flattened = flatten(blocks, &cache()).unwrap();

        let path = flattened.strip_prefix("what is this @").expect("a mention follows the text");
        assert!(path.ends_with(".png"), "{path}");
        assert_eq!(std::fs::read(path).unwrap(), b"not really a png");
    }
}
