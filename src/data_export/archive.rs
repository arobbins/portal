use std::io::{Cursor, Write};

use time::OffsetDateTime;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::dataset::Dataset;
use super::datasets::LEFT_OUT;
use crate::app::SITE_NAME;

pub const README: &str = "README.txt";

pub fn file_name(at: OffsetDateTime) -> String {
    let site = SITE_NAME.to_lowercase().replace(' ', "-");
    let date = at.to_offset(time::UtcOffset::UTC).date();
    format!(
        "{site}-data-{:04}-{:02}-{:02}.zip",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

pub fn archive(datasets: &[Dataset]) -> topcoat::Result<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    add(&mut zip, options, README, readme(datasets).as_bytes())?;
    for dataset in datasets {
        add(&mut zip, options, dataset.file, &csv(dataset)?)?;
    }
    let bytes = zip.finish().map_err(failed)?.into_inner();
    Ok(bytes)
}

fn add(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    options: SimpleFileOptions,
    name: &str,
    contents: &[u8],
) -> topcoat::Result<()> {
    zip.start_file(name, options).map_err(failed)?;
    zip.write_all(contents).map_err(failed)?;
    Ok(())
}

fn csv(dataset: &Dataset) -> topcoat::Result<Vec<u8>> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(dataset.columns).map_err(failed)?;
    for row in &dataset.rows {
        let safe = row.iter().map(|cell| spreadsheet_safe(cell));
        writer.write_record(safe).map_err(failed)?;
    }
    writer.into_inner().map_err(failed)
}

const FORMULA_STARTS: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

fn spreadsheet_safe(cell: &str) -> String {
    let risky = cell.starts_with(FORMULA_STARTS) && !is_plain_number(cell);
    if risky {
        format!("'{cell}")
    } else {
        cell.to_string()
    }
}

fn is_plain_number(cell: &str) -> bool {
    let digits = cell.strip_prefix(['-', '+']).unwrap_or(cell);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, "0"));
    let all_digits = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
    all_digits(whole) && all_digits(fraction)
}

fn readme(datasets: &[Dataset]) -> String {
    let mut text = format!(
        "Your {SITE_NAME} data\n\nEach .csv file below opens in any spreadsheet app. They are UTF-8 with a header row, and every date and time is in UTC (ISO 8601, e.g. 2026-11-18T14:03:00Z). A value that starts with =, +, -, @, a tab or a carriage return has an apostrophe (') in front, so spreadsheets show it as text instead of running it as a formula; remove that apostrophe to get the original value.\n"
    );
    for dataset in datasets {
        text.push_str(&format!(
            "\n{}\n  {}\n  Columns: {}\n  Stored in: {}\n",
            dataset.file,
            dataset.about,
            dataset.columns.join(", "),
            dataset.tables.join(", ")
        ));
    }
    text.push_str("\nNot included\n");
    for left_out in LEFT_OUT {
        text.push_str(&format!("\n  {}: {}\n", left_out.table, left_out.why));
    }
    text
}

fn failed(error: impl std::fmt::Display) -> topcoat::Error {
    topcoat::Error::msg(format!("building the data export failed: {error}"))
}

#[cfg(test)]
mod tests;
