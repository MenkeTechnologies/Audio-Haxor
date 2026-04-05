//! PDF metadata extraction (page count).
//!
//! Uses `lopdf` to read the document catalog and return the page count.
//! Designed for bulk extraction — returns `None` on any parse error so
//! one bad file doesn't stop a batch job.

use rayon::prelude::*;

/// Page count for a single PDF. Returns None if the file can't be parsed.
pub fn extract_page_count(path: &str) -> Option<u32> {
    let doc = lopdf::Document::load(path).ok()?;
    Some(doc.get_pages().len() as u32)
}

/// Batch page-count extraction with parallel parsing. Returns (path, pages) pairs
/// only for PDFs that parsed successfully.
pub fn extract_pages_batch(paths: &[String]) -> Vec<(String, u32)> {
    paths
        .par_iter()
        .filter_map(|p| extract_page_count(p).map(|n| (p.clone(), n)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_pages_missing_file_returns_none() {
        assert!(extract_page_count("/nonexistent/file.pdf").is_none());
    }

    #[test]
    fn extract_pages_not_a_pdf_returns_none() {
        let tmp = std::env::temp_dir().join("upum_not_a_pdf.pdf");
        std::fs::write(&tmp, b"this is not a pdf").unwrap();
        let res = extract_page_count(tmp.to_str().unwrap());
        let _ = std::fs::remove_file(&tmp);
        assert!(res.is_none());
    }

    #[test]
    fn extract_pages_batch_skips_bad_files() {
        let paths = vec![
            "/nonexistent/a.pdf".to_string(),
            "/nonexistent/b.pdf".to_string(),
        ];
        let result = extract_pages_batch(&paths);
        assert!(result.is_empty());
    }
}
