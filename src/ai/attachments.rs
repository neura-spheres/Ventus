use std::io::{Cursor, Read};
use std::path::Path;

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use uuid::Uuid;
use zip::ZipArchive;

use super::provider::{AiAttachment, AiAttachmentKind};

pub const MAX_ATTACHMENTS: usize = 5;
pub const MAX_ATTACHMENT_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 10 * 1024 * 1024;
const MAX_TEXT_CHARS: usize = 120_000;
const MAX_DOC_XML_BYTES: u64 = 24 * 1024 * 1024;

pub fn pick(
    existing: &[AiAttachment],
    used_count: usize,
    used_bytes: u64,
) -> Result<Option<Vec<AiAttachment>>> {
    let paths = rfd::FileDialog::new()
        .set_title("Add files to AI chat")
        .add_filter(
            "Supported files",
            &[
                "pdf", "docx", "txt", "md", "csv", "json", "html", "htm", "xml", "yaml", "yml",
                "toml", "log", "rs", "js", "ts", "jsx", "tsx", "py", "java", "c", "cpp", "h",
                "css", "png", "jpg", "jpeg", "webp", "gif",
            ],
        )
        .pick_files();
    let Some(paths) = paths else {
        return Ok(None);
    };
    let mut attachments = existing.to_vec();
    if used_count + attachments.len() + paths.len() > MAX_ATTACHMENTS {
        return Err(anyhow!(
            "You can attach up to {MAX_ATTACHMENTS} files at once"
        ));
    }
    let mut total = used_bytes + attachments.iter().map(|item| item.size).sum::<u64>();
    for path in paths {
        let item = load(&path)?;
        total = total.saturating_add(item.size);
        if total > MAX_TOTAL_BYTES {
            return Err(anyhow!("Attachments can be up to 10 MB per chat"));
        }
        attachments.push(item);
    }
    Ok(Some(attachments))
}

fn load(path: &Path) -> Result<AiAttachment> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("This file name is not supported"))?
        .to_string();
    let size = std::fs::metadata(path)?.len();
    if size == 0 {
        return Err(anyhow!("{name} is empty"));
    }
    if size > MAX_ATTACHMENT_BYTES {
        return Err(anyhow!("{name} is larger than 8 MB"));
    }
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let bytes = std::fs::read(path)?;
    let (mime_type, kind, data_base64, text) = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" => image_attachment(&ext, &bytes)?,
        "pdf" => (
            "application/pdf".to_string(),
            AiAttachmentKind::Pdf,
            Some(STANDARD.encode(&bytes)),
            pdf_extract::extract_text_from_mem(&bytes)
                .ok()
                .map(|text| trim_text(&text))
                .filter(|text| !text.trim().is_empty()),
        ),
        "docx" => (
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
            AiAttachmentKind::Text,
            None,
            Some(docx_text(&bytes)?),
        ),
        _ if is_text_extension(&ext) => (
            text_mime(&ext).to_string(),
            AiAttachmentKind::Text,
            None,
            Some(plain_text(&name, &bytes)?),
        ),
        _ => return Err(anyhow!("{name} is not a supported file type")),
    };
    Ok(AiAttachment {
        id: Uuid::new_v4().to_string(),
        name,
        mime_type,
        kind,
        size,
        data_base64,
        text,
    })
}

fn image_attachment(
    ext: &str,
    bytes: &[u8],
) -> Result<(String, AiAttachmentKind, Option<String>, Option<String>)> {
    image::load_from_memory(bytes).map_err(|_| anyhow!("This image could not be read"))?;
    let mime = match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return Err(anyhow!("This image type is not supported")),
    };
    Ok((
        mime.to_string(),
        AiAttachmentKind::Image,
        Some(STANDARD.encode(bytes)),
        None,
    ))
}

fn plain_text(name: &str, bytes: &[u8]) -> Result<String> {
    if bytes.iter().take(4096).any(|byte| *byte == 0) {
        return Err(anyhow!("{name} does not look like a text file"));
    }
    let text = trim_text(&String::from_utf8_lossy(bytes));
    if text.trim().is_empty() {
        return Err(anyhow!("{name} has no readable text"));
    }
    Ok(text)
}

fn docx_text(bytes: &[u8]) -> Result<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| anyhow!("This Word document could not be opened"))?;
    let mut file = archive
        .by_name("word/document.xml")
        .map_err(|_| anyhow!("This Word document has no readable content"))?;
    if file.size() > MAX_DOC_XML_BYTES {
        return Err(anyhow!("This Word document is too complex"));
    }
    let mut xml = String::new();
    file.read_to_string(&mut xml)
        .map_err(|_| anyhow!("This Word document has invalid text"))?;
    let doc = roxmltree::Document::parse(&xml)
        .map_err(|_| anyhow!("This Word document has invalid content"))?;
    let mut out = String::new();
    for paragraph in doc
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "p")
    {
        let line = paragraph
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == "t")
            .filter_map(|node| node.text())
            .collect::<String>();
        if line.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&line);
    }
    if out.trim().is_empty() {
        return Err(anyhow!("This Word document has no readable text"));
    }
    Ok(trim_text(&out))
}

fn trim_text(text: &str) -> String {
    let mut out = text.chars().take(MAX_TEXT_CHARS).collect::<String>();
    if text.chars().count() > MAX_TEXT_CHARS {
        out.push_str("\n[Attachment text truncated]");
    }
    out
}

fn is_text_extension(ext: &str) -> bool {
    matches!(
        ext,
        "txt"
            | "md"
            | "csv"
            | "json"
            | "html"
            | "htm"
            | "xml"
            | "yaml"
            | "yml"
            | "toml"
            | "log"
            | "rs"
            | "js"
            | "ts"
            | "jsx"
            | "tsx"
            | "py"
            | "java"
            | "c"
            | "cpp"
            | "h"
            | "css"
    )
}

fn text_mime(ext: &str) -> &'static str {
    match ext {
        "csv" => "text/csv",
        "json" => "application/json",
        "html" | "htm" => "text/html",
        "xml" => "application/xml",
        "yaml" | "yml" => "application/yaml",
        "md" => "text/markdown",
        _ => "text/plain",
    }
}

pub fn metadata_json(attachments: &[AiAttachment]) -> serde_json::Value {
    serde_json::Value::Array(
        attachments
            .iter()
            .map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "name": item.name,
                    "kind": item.kind,
                    "size": item.size,
                })
            })
            .collect(),
    )
}

pub fn remove(attachments: &mut Vec<AiAttachment>, id: &str) {
    attachments.retain(|item| item.id != id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn text_limits_are_bounded() {
        let text = "a".repeat(MAX_TEXT_CHARS + 10);
        let trimmed = trim_text(&text);
        assert!(trimmed.ends_with("[Attachment text truncated]"));
        assert_eq!(
            trimmed.lines().next().unwrap_or_default().len(),
            MAX_TEXT_CHARS
        );
    }

    #[test]
    fn supported_extensions_reject_unknown_files() {
        assert!(is_text_extension("md"));
        assert!(!is_text_extension("exe"));
    }

    #[test]
    fn docx_text_reads_paragraphs() {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut bytes);
            writer
                .start_file(
                    "word/document.xml",
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            writer
                .write_all(br#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Hello</w:t></w:r></w:p><w:p><w:r><w:t>World</w:t></w:r></w:p></w:body></w:document>"#)
                .unwrap();
            writer.finish().unwrap();
        }
        assert_eq!(docx_text(bytes.get_ref()).unwrap(), "Hello\nWorld");
    }
}
