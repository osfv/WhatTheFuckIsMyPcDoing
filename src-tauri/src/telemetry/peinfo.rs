use std::path::Path;

use pelite::pe32::Pe as Pe32;
use pelite::pe64::Pe as Pe64;

#[derive(Default, Clone)]
pub struct FileMeta {
    pub publisher: Option<String>,
    pub product: Option<String>,
    pub file_version: Option<String>,
    pub has_signature: bool,
    pub signature_status: String,
}

pub fn enrich_details(path: &str) -> FileMeta {
    let path = Path::new(path);
    if !path.exists() {
        return FileMeta {
            signature_status: "File is no longer on disk.".into(),
            ..Default::default()
        };
    }
    let Ok(bytes) = std::fs::read(path) else {
        return FileMeta {
            signature_status: "Could not read this executable.".into(),
            ..Default::default()
        };
    };
    parse_pe(&bytes).unwrap_or(FileMeta {
        signature_status: "Could not parse this executable.".into(),
        ..Default::default()
    })
}

fn parse_pe(bytes: &[u8]) -> Option<FileMeta> {
    match pelite::PeFile::from_bytes(bytes).ok()? {
        pelite::Wrap::T64(file) => Some(from_pe64(file)),
        pelite::Wrap::T32(file) => Some(from_pe32(file)),
    }
}

fn from_pe64(file: pelite::pe64::PeFile<'_>) -> FileMeta {
    let has_signature = file
        .data_directory()
        .get(pelite::image::IMAGE_DIRECTORY_ENTRY_SECURITY)
        .map(|d| d.VirtualAddress != 0 && d.Size != 0)
        .unwrap_or(false);
    let mut meta = base(has_signature);
    if let Ok(res) = file.resources() {
        apply_version(&mut meta, res);
    }
    meta
}

fn from_pe32(file: pelite::pe32::PeFile<'_>) -> FileMeta {
    let has_signature = file
        .data_directory()
        .get(pelite::image::IMAGE_DIRECTORY_ENTRY_SECURITY)
        .map(|d| d.VirtualAddress != 0 && d.Size != 0)
        .unwrap_or(false);
    let mut meta = base(has_signature);
    if let Ok(res) = file.resources() {
        apply_version(&mut meta, res);
    }
    meta
}

fn base(has_signature: bool) -> FileMeta {
    FileMeta {
        has_signature,
        signature_status: if has_signature {
            "Embedded Authenticode signature present. This is not a live Microsoft trust check.".into()
        } else {
            "No embedded Authenticode signature.".into()
        },
        ..Default::default()
    }
}

fn apply_version(meta: &mut FileMeta, res: pelite::resources::Resources<'_>) {
    let Ok(info) = res.version_info() else {
        return;
    };
    let translations = info.translation();
    let Some(lang) = translations.first().copied() else {
        return;
    };
    meta.publisher = info.value(lang, "CompanyName").map(|s| s.to_string());
    meta.product = info
        .value(lang, "FileDescription")
        .or_else(|| info.value(lang, "ProductName"))
        .map(|s| s.to_string());
    meta.file_version = info.value(lang, "FileVersion").map(|s| s.to_string());
    if meta.publisher.as_ref().is_some_and(|s| s.trim().is_empty()) {
        meta.publisher = None;
    }
}
