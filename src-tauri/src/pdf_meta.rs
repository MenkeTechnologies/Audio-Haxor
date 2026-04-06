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

    /// printpdf emits a valid file; lopdf must agree on page count (regression for bulk PDF indexing).
    #[test]
    fn extract_page_count_matches_printpdf_three_pages() {
        use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions};
        use std::fs::File;
        use std::io::BufWriter;

        let tmp =
            std::env::temp_dir().join(format!("ah_pdf_meta_three_{}.pdf", std::process::id()));
        let mut doc = PdfDocument::new("pdf_meta_test");
        let p1 = PdfPage::new(Mm(40.0), Mm(40.0), vec![Op::SaveGraphicsState, Op::RestoreGraphicsState]);
        let p2 = PdfPage::new(Mm(40.0), Mm(40.0), vec![Op::SaveGraphicsState, Op::RestoreGraphicsState]);
        let p3 = PdfPage::new(Mm(40.0), Mm(40.0), vec![Op::SaveGraphicsState, Op::RestoreGraphicsState]);
        doc.with_pages(vec![p1, p2, p3]);
        let bytes = doc.save(&PdfSaveOptions::default(), &mut Vec::new());
        std::io::Write::write_all(
            &mut BufWriter::new(File::create(&tmp).expect("temp pdf create")),
            &bytes,
        )
        .expect("printpdf save");

        let n = extract_page_count(tmp.to_str().unwrap());
        let _ = std::fs::remove_file(&tmp);
        assert_eq!(n, Some(3));
    }

    #[test]
    fn extract_pages_batch_merges_valid_paths() {
        use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions};
        use std::fs::File;
        use std::io::BufWriter;

        let id = std::process::id();
        let a = std::env::temp_dir().join(format!("ah_pdf_batch_a_{id}.pdf"));
        let b = std::env::temp_dir().join(format!("ah_pdf_batch_b_{id}.pdf"));

        let mut doc_a = PdfDocument::new("a");
        doc_a.with_pages(vec![PdfPage::new(
            Mm(30.0),
            Mm(30.0),
            vec![Op::SaveGraphicsState, Op::RestoreGraphicsState],
        )]);
        let bytes = doc_a.save(&PdfSaveOptions::default(), &mut Vec::new());
        std::io::Write::write_all(&mut BufWriter::new(File::create(&a).unwrap()), &bytes)
            .expect("save a");

        let mut doc_b = PdfDocument::new("b");
        doc_b.with_pages(vec![
            PdfPage::new(
                Mm(30.0),
                Mm(30.0),
                vec![Op::SaveGraphicsState, Op::RestoreGraphicsState],
            ),
            PdfPage::new(
                Mm(30.0),
                Mm(30.0),
                vec![Op::SaveGraphicsState, Op::RestoreGraphicsState],
            ),
        ]);
        let bytes = doc_b.save(&PdfSaveOptions::default(), &mut Vec::new());
        std::io::Write::write_all(&mut BufWriter::new(File::create(&b).unwrap()), &bytes)
            .expect("save b");

        let paths = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
            "/totally/missing/xyz.pdf".to_string(),
        ];
        let mut pairs = extract_pages_batch(&paths);
        pairs.sort_by(|x, y| x.0.cmp(&y.0));

        let _ = std::fs::remove_file(&a);
        let _ = std::fs::remove_file(&b);

        assert_eq!(pairs.len(), 2);
        assert!(pairs.iter().any(|(_, n)| *n == 1));
        assert!(pairs.iter().any(|(_, n)| *n == 2));
    }
}
