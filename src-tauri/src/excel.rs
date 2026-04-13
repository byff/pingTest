// Excel module - stub that re-exports from main project
// In a full setup, this would be copied from pingTest/src/excel/mod.rs

use calamine::{open_workbook, Reader, Xlsx, Xls, Data};
use rust_xlsxwriter::{Workbook, Format};
use std::io::BufReader;
use std::fs::File;
use std::path::Path;

use crate::ping::PingTarget;
use crate::config::ExportConfig;

pub fn read_excel(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "xlsx" => read_xlsx(path),
        "xls" => read_xls(path),
        "txt" | "csv" => read_text(path),
        _ => Err(format!("Unsupported file format: {}", ext)),
    }
}

fn read_xlsx(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut workbook: Xlsx<BufReader<File>> = open_workbook(path).map_err(|e: calamine::XlsxError| e.to_string())?;
    let sheets = workbook.sheet_names().to_vec();
    let sheet_name = sheets.first().ok_or("No sheets found")?.clone();
    let range = workbook.worksheet_range(&sheet_name).map_err(|e: calamine::XlsxError| e.to_string())?;
    extract_from_range(&range)
}

fn read_xls(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut workbook: Xls<BufReader<File>> = open_workbook(path).map_err(|e: calamine::XlsError| e.to_string())?;
    let sheets = workbook.sheet_names().to_vec();
    let sheet_name = sheets.first().ok_or("No sheets found")?.clone();
    let range = workbook.worksheet_range(&sheet_name).map_err(|e: calamine::XlsError| e.to_string())?;
    extract_from_range(&range)
}

fn extract_from_range(range: &calamine::Range<Data>) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut rows_iter = range.rows();

    let headers: Vec<String> = match rows_iter.next() {
        Some(row) => row.iter().map(|c: &Data| c.to_string()).collect(),
        None => return Ok((Vec::new(), Vec::new())),
    };

    let rows: Vec<Vec<String>> = rows_iter
        .map(|row: &[Data]| row.iter().map(|c: &Data| c.to_string()).collect())
        .collect();

    Ok((headers, rows))
}

fn read_text(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines: Vec<Vec<String>> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(vec![trimmed.to_string()]);
        }
    }

    if lines.is_empty() {
        return Ok((vec!["IP".to_string()], Vec::new()));
    }

    let headers = vec!["IP".to_string()];
    Ok((headers, lines))
}

pub fn export_results(
    path: &Path,
    targets: &[PingTarget],
    config: &ExportConfig,
) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    let header_fmt = Format::new().set_bold();

    let mut col: u16 = 0;
    let mut columns: Vec<&str> = Vec::new();

    macro_rules! add_header {
        ($cond:expr, $name:expr, $key:expr) => {
            if $cond {
                sheet.write_string_with_format(0, col, $name, &header_fmt).map_err(|e| e.to_string())?;
                columns.push($key);
                col += 1;
            }
        };
    }

    add_header!(config.export_ip, "IP", "ip");
    add_header!(config.export_hostname, "主机名", "hostname");
    add_header!(config.export_success_count, "成功次数", "success");
    add_header!(config.export_fail_count, "失败次数", "fail");
    add_header!(config.export_fail_rate, "失败率(%)", "fail_rate");
    add_header!(config.export_total_sent, "总发送", "total");
    add_header!(config.export_last_rtt, "延迟(ms)", "last_rtt");
    add_header!(config.export_max_rtt, "最大延迟(ms)", "max_rtt");
    add_header!(config.export_min_rtt, "最小延迟(ms)", "min_rtt");
    add_header!(config.export_avg_rtt, "平均延迟(ms)", "avg_rtt");

    for (row_idx, target) in targets.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        let stats = target.stats.read();
        let mut c: u16 = 0;

        for &col_type in &columns {
            match col_type {
                "ip" => { sheet.write_string(row, c, target.ip.to_string()).map_err(|e| e.to_string())?; }
                "hostname" => { sheet.write_string(row, c, &target.hostname).map_err(|e| e.to_string())?; }
                "success" => { sheet.write_number(row, c, stats.success_count as f64).map_err(|e| e.to_string())?; }
                "fail" => { sheet.write_number(row, c, stats.fail_count as f64).map_err(|e| e.to_string())?; }
                "fail_rate" => { sheet.write_number(row, c, stats.fail_rate()).map_err(|e| e.to_string())?; }
                "total" => { sheet.write_number(row, c, stats.total_sent as f64).map_err(|e| e.to_string())?; }
                "last_rtt" => {
                    let v = stats.last_rtt_us.map(|u| u as f64 / 1000.0).unwrap_or(0.0);
                    sheet.write_number(row, c, v).map_err(|e| e.to_string())?;
                }
                "max_rtt" => { sheet.write_number(row, c, stats.max_rtt_us as f64 / 1000.0).map_err(|e| e.to_string())?; }
                "min_rtt" => { sheet.write_number(row, c, stats.min_rtt_us as f64 / 1000.0).map_err(|e| e.to_string())?; }
                "avg_rtt" => { sheet.write_number(row, c, stats.avg_rtt_us() as f64 / 1000.0).map_err(|e| e.to_string())?; }
                _ => {}
            }
            c += 1;
        }
    }

    workbook.save(path).map_err(|e| e.to_string())?;
    Ok(())
}
