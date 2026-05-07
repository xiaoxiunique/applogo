use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub struct ZipEntry {
    pub path: String,
    pub data: Vec<u8>,
}

pub fn build_zip(output: &Path, entries: Vec<ZipEntry>) -> Result<()> {
    let file = File::create(output)
        .with_context(|| format!("Failed to create output file: {}", output.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6));

    let mut seen = HashSet::new();
    for entry in &entries {
        if !seen.insert(&entry.path) {
            continue; // skip duplicate paths
        }
        zip.start_file(&entry.path, options)
            .with_context(|| format!("Failed to write ZIP entry: {}", entry.path))?;
        zip.write_all(&entry.data)
            .with_context(|| format!("Failed to write data for: {}", entry.path))?;
    }

    zip.finish().context("Failed to finalize ZIP")?;
    Ok(())
}
