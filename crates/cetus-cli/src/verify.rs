//! `cetus verify` implementation.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportFormat {
    Markdown,
    Json,
}

impl ReportFormat {
    fn render(self, report: &cetus_verify::Report) -> String {
        match self {
            Self::Markdown => report.to_markdown(),
            Self::Json => report.to_json().to_string(),
        }
    }
}

pub(crate) fn run(path: &Path, level: &str, report_format: &str, json: bool) -> Result<(), u8> {
    let level = cetus_verify::Level::parse(level).map_err(|error| {
        eprintln!("error: {error}");
        2
    })?;
    let formats = parse_report_formats(report_format).map_err(|error| {
        eprintln!("error: {error}");
        1
    })?;
    let report = cetus_verify::verify_path(path, level).map_err(|error| {
        eprintln!("error: {error}");
        1
    })?;
    if json {
        println!("{}", report.to_json());
    } else {
        let rendered = formats
            .iter()
            .map(|format| format.render(&report))
            .collect::<Vec<_>>()
            .join("\n\n");
        println!("{rendered}");
    }
    if report.passed() { Ok(()) } else { Err(1) }
}

fn parse_report_formats(text: &str) -> Result<Vec<ReportFormat>, String> {
    let mut formats = Vec::new();
    for part in text.split(',') {
        let format = match part.trim().to_ascii_lowercase().as_str() {
            "md" | "markdown" => ReportFormat::Markdown,
            "json" => ReportFormat::Json,
            _ => return Err(format!("unknown report format `{}`", part.trim())),
        };
        if !formats.contains(&format) {
            formats.push(format);
        }
    }
    Ok(formats)
}
