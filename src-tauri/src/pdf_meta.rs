//! PDF metadata extraction (page count + Info dictionary dates).
//!
//! Uses `lopdf::Document::load_metadata` so we read page count and `CreationDate` /
//! `ModDate` without loading the full document. Returns `None` on any parse error
//! so one bad file doesn't stop a batch job.

use rayon::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PdfMetaFields {
    pub pages: u32,
    pub pdf_creation_date: Option<String>,
    pub pdf_mod_date: Option<String>,
}

fn norm_info_date(s: Option<String>) -> Option<String> {
    let s = s?.trim().to_owned();
    if s.is_empty() { None } else { Some(s) }
}

/// Page count, PDF CreationDate, and ModDate from the Info dictionary.
/// Returns None if the file can't be parsed.
pub fn extract_pdf_meta(path: &str) -> Option<PdfMetaFields> {
    let m = lopdf::Document::load_metadata(path).ok()?;
    Some(PdfMetaFields {
        pages: m.page_count,
        pdf_creation_date: norm_info_date(m.creation_date),
        pdf_mod_date: norm_info_date(m.modification_date),
    })
}

/// Page count for a single PDF. Returns None if the file can't be parsed.
pub fn extract_page_count(path: &str) -> Option<u32> {
    extract_pdf_meta(path).map(|m| m.pages)
}

/// Parallel metadata extraction. Maps each successfully parsed path to fields.
pub fn extract_pdf_meta_batch(paths: &[String]) -> HashMap<String, PdfMetaFields> {
    paths
        .par_iter()
        .filter_map(|p| extract_pdf_meta(p).map(|m| (p.clone(), m)))
        .collect()
}

/// Batch page-count extraction with parallel parsing. Returns (path, pages) pairs
/// only for PDFs that parsed successfully.
pub fn extract_pages_batch(paths: &[String]) -> Vec<(String, u32)> {
    extract_pdf_meta_batch(paths)
        .into_iter()
        .map(|(p, m)| (p, m.pages))
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
    fn extract_pdf_meta_batch_skips_bad_files() {
        let paths = vec![
            "/nonexistent/a.pdf".to_string(),
            "/nonexistent/b.pdf".to_string(),
        ];
        assert!(extract_pdf_meta_batch(&paths).is_empty());
        assert!(extract_pages_batch(&paths).is_empty());
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
        let p1 = PdfPage::new(
            Mm(40.0),
            Mm(40.0),
            vec![Op::SaveGraphicsState, Op::RestoreGraphicsState],
        );
        let p2 = PdfPage::new(
            Mm(40.0),
            Mm(40.0),
            vec![Op::SaveGraphicsState, Op::RestoreGraphicsState],
        );
        let p3 = PdfPage::new(
            Mm(40.0),
            Mm(40.0),
            vec![Op::SaveGraphicsState, Op::RestoreGraphicsState],
        );
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

    #[test]
    fn extract_pdf_meta_reads_info_dates() {
        use lopdf::content::Content;
        use lopdf::{Document, Object, Stream, dictionary};

        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let info_id = doc.add_object(dictionary! {
            "Creator" => "audio_haxor_test",
            "CreationDate" => Object::string_literal("D:20260101120000"),
            "ModDate" => Object::string_literal("D:20260202123045"),
        });
        let content = Content { operations: vec![] };
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            content.encode().expect("encode empty content"),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.trailer.set("Info", info_id);

        let tmp =
            std::env::temp_dir().join(format!("ah_pdf_info_dates_{}.pdf", std::process::id()));
        doc.save(&tmp).expect("save test pdf");

        let m = extract_pdf_meta(tmp.to_str().unwrap()).expect("meta");
        let _ = std::fs::remove_file(&tmp);
        assert_eq!(m.pages, 1);
        assert_eq!(m.pdf_creation_date.as_deref(), Some("D:20260101120000"));
        assert_eq!(m.pdf_mod_date.as_deref(), Some("D:20260202123045"));
    }
}
