//! # SVG Charts — رسوم بيانية بدون أي مكتبة خارجية
//!
//! ## أنواع الرسوم
//! - Bar chart (أعمدة)
//! - Line chart (خطي)
//! - Pie chart (دائري)
//! - HTML Table (جدول)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub label: String,
    pub value: f64,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSeries {
    pub label: String,
    pub color: String,
    pub points: Vec<DataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Table,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartRequest {
    pub chart_type: ChartType,
    pub title: String,
    pub series: Vec<DataSeries>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// توليد SVG chart بناءً على الطلب
pub fn render_chart(req: &ChartRequest) -> Result<String, String> {
    match req.chart_type {
        ChartType::Bar => render_bar_chart(req),
        ChartType::Line => render_line_chart(req),
        ChartType::Pie => render_pie_chart(req),
        ChartType::Table => render_html_table(req),
    }
}

// ─── Bar Chart ─────────────────────────────────────────────────────────────

fn render_bar_chart(req: &ChartRequest) -> Result<String, String> {
    let w = req.width.unwrap_or(600).max(300);
    let h = req.height.unwrap_or(400).max(200);
    let mut svg = String::new();

    svg.push_str(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="100%" height="100%">"#, w, h));
    svg.push_str(&format!(r#"<rect width="100%" height="100%" fill="#1a1a2e" rx="8"/>"#));

    // Title
    svg.push_str(&format!(r#"<text x="{}" y="30" text-anchor="middle" fill="#fff" font-size="16" font-family="sans-serif">{}</text>"#, w/2, escape_svg(&req.title)));

    let margin = 60;
    let chart_w = w as i32 - margin * 2;
    let chart_h = h as i32 - margin * 2 - 20;
    let chart_top = margin + 20;

    // Collect all values
    let all_values: Vec<f64> = req.series.iter().flat_map(|s| s.points.iter().map(|p| p.value)).collect();
    let max_val = all_values.iter().cloned().fold(0f64, f64::max).max(1.0);

    // Y-axis grid
    let grid_lines = 5;
    for i in 0..=grid_lines {
        let y = chart_top + chart_h - (chart_h as f64 * i as f64 / grid_lines as f64) as i32;
        svg.push_str(&format!(r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#333" stroke-width="1"/>"#, margin, y, margin + chart_w, y));
        let val = max_val * i as f64 / grid_lines as f64;
        svg.push_str(&format!(r#"<text x="{}" y="{}" text-anchor="end" fill="#888" font-size="11">{:.1}</text>"#, margin - 5, y + 4, val));
    }

    // Bars
    let colors = ["#00d4ff", "#ff6b6b", "#ffd93d", "#6bcb77", "#4d96ff", "#ff6bff", "#ff9f43"];
    let mut bar_idx = 0;

    for series in &req.series {
        let total_bars = req.series.iter().map(|s| s.points.len()).sum::<usize>();
        let bar_w = ((chart_w as f64 / total_bars as f64) * 0.7).max(8.0) as i32;
        let gap = ((chart_w as f64 / total_bars as f64) * 0.3) as i32;
        let total_w = bar_w + gap;

        for point in &series.points {
            let x = margin + bar_idx * total_w;
            let bar_h = (chart_h as f64 * (point.value / max_val)) as i32;
            let y = chart_top + chart_h - bar_h;
            let color = &point.color.clone().unwrap_or_else(|| colors[bar_idx % colors.len()].to_string());

            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" rx="3"><title>{}: {}</title></rect>"#,
                x, y, bar_w, bar_h, color, escape_svg(&point.label), point.value
            ));

            // Label
            if point.label.len() <= 8 {
                svg.push_str(&format!(
                    r#"<text x="{}" y="{}" text-anchor="middle" fill="#aaa" font-size="10" transform="rotate(-45, {}, {})">{}</text>"#,
                    x + bar_w/2, chart_top + chart_h + 15, x + bar_w/2, chart_top + chart_h + 15, escape_svg(&point.label)
                ));
            }
            bar_idx += 1;
        }
    }

    // Legend
    if req.series.len() > 1 {
        let legend_y = h as i32 - 15;
        let mut legend_x = margin;
        for series in &req.series {
            let color = &series.color;
            svg.push_str(&format!(r#"<rect x="{}" y="{}" width="10" height="10" fill="{}" rx="2"/>"#, legend_x, legend_y - 10, color));
            svg.push_str(&format!(r#"<text x="{}" y="{}" fill="#ccc" font-size="11">{}</text>"#, legend_x + 14, legend_y, escape_svg(&series.label)));
            legend_x += 14 + (series.label.len() as i32 * 8) + 20;
        }
    }

    svg.push_str("</svg>");
    Ok(svg)
}

// ─── Line Chart ────────────────────────────────────────────────────────────

fn render_line_chart(req: &ChartRequest) -> Result<String, String> {
    let w = req.width.unwrap_or(600).max(300);
    let h = req.height.unwrap_or(400).max(200);
    let mut svg = String::new();

    svg.push_str(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="100%" height="100%">"#, w, h));
    svg.push_str(&format!(r#"<rect width="100%" height="100%" fill="#1a1a2e" rx="8"/>"#));
    svg.push_str(&format!(r#"<text x="{}" y="30" text-anchor="middle" fill="#fff" font-size="16" font-family="sans-serif">{}</text>"#, w/2, escape_svg(&req.title)));

    let margin = 60;
    let chart_w = w as i32 - margin * 2;
    let chart_h = h as i32 - margin * 2 - 20;
    let chart_top = margin + 20;

    let all_values: Vec<f64> = req.series.iter().flat_map(|s| s.points.iter().map(|p| p.value)).collect();
    let max_val = all_values.iter().cloned().fold(0f64, f64::max).max(1.0);

    // Y-axis grid
    for i in 0..=5 {
        let y = chart_top + chart_h - (chart_h as f64 * i as f64 / 5.0) as i32;
        svg.push_str(&format!(r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#333" stroke-width="1"/>"#, margin, y, margin + chart_w, y));
        let val = max_val * i as f64 / 5.0;
        svg.push_str(&format!(r#"<text x="{}" y="{}" text-anchor="end" fill="#888" font-size="11">{:.1}</text>"#, margin - 5, y + 4, val));
    }

    let colors = ["#00d4ff", "#ff6b6b", "#ffd93d", "#6bcb77"];
    for (si, series) in req.series.iter().enumerate() {
        let color = &series.color;
        let n = series.points.len().max(2);
        let step_x = chart_w as f64 / (n - 1) as f64;
        let mut points = Vec::new();

        for (i, point) in series.points.iter().enumerate() {
            let x_val = margin as f64 + i as f64 * step_x;
            let y_val = chart_top as f64 + chart_h as f64 * (1.0 - point.value / max_val);
            points.push((x_val, y_val));
        }

        // Line
        if points.len() >= 2 {
            let mut path = format!(r#"<path d="M{} {}"# , points[0].0, points[0].1);
            for p in &points[1..] {
                path.push_str(&format!(" L{} {}", p.0, p.1));
            }
            path.push_str(&format!(r#"" stroke="{}" stroke-width="2" fill="none" stroke-linejoin="round"/>"#, color));
            svg.push_str(&path);

            // Area fill
            if points.len() >= 2 {
                let mut area = format!(r#"<path d="M{} {}"# , points[0].0, chart_top + chart_h);
                area.push_str(&format!(" L{} {}", points[0].0, points[0].1));
                for p in &points[1..] {
                    area.push_str(&format!(" L{} {}", p.0, p.1));
                }
                area.push_str(&format!(" L{} {}", points.last().unwrap().0, chart_top + chart_h));
                area.push_str(&format!(r#" Z" fill="{}" fill-opacity="0.1"/>"#, color));
                svg.push_str(&area);
            }
        }

        // Dots
        for (i, (x, y)) in points.iter().enumerate() {
            svg.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="4" fill="{}"><title>{}: {:.2}</title></circle>"#,
                x, y, color, escape_svg(&series.points[i].label), series.points[i].value
            ));
            // X-axis label
            if i % std::cmp::max(1, n / 10) == 0 {
                svg.push_str(&format!(
                    r#"<text x="{}" y="{}" text-anchor="middle" fill="#aaa" font-size="10">{}</text>"#,
                    x, chart_top + chart_h + 15, escape_svg(&series.points[i].label)
                ));
            }
        }
    }

    svg.push_str("</svg>");
    Ok(svg)
}

// ─── Pie Chart ─────────────────────────────────────────────────────────────

fn render_pie_chart(req: &ChartRequest) -> Result<String, String> {
    let w = req.width.unwrap_or(500).max(300);
    let h = req.height.unwrap_or(500).max(300);
    let mut svg = String::new();

    svg.push_str(&format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="100%" height="100%">"#, w, h));
    svg.push_str(&format!(r#"<rect width="100%" height="100%" fill="#1a1a2e" rx="8"/>"#));
    svg.push_str(&format!(r#"<text x="{}" y="30" text-anchor="middle" fill="#fff" font-size="16" font-family="sans-serif">{}</text>"#, w/2, escape_svg(&req.title)));

    let cx = (w as i32 / 2) - 60;
    let cy = h as i32 / 2 + 10;
    let r = (h as i32 / 2 - 60).min(cx - 20) as f64;

    // Collect all data points from all series
    let mut all_points: Vec<(String, f64, String)> = Vec::new();
    let colors = ["#00d4ff", "#ff6b6b", "#ffd93d", "#6bcb77", "#4d96ff", "#ff6bff", "#ff9f43", "#1dd1a1"];

    for (si, series) in req.series.iter().enumerate() {
        for point in &series.points {
            let color = point.color.clone().unwrap_or_else(|| colors[(all_points.len() + si) % colors.len()].to_string());
            all_points.push((point.label.clone(), point.value, color));
        }
    }

    let total: f64 = all_points.iter().map(|p| p.1).sum();
    if total == 0.0 {
        return Err("مجموع القيم صفر — لا يمكن رسم Pie chart".into());
    }

    let mut start_angle = -90.0f64;
    for (label, value, color) in &all_points {
        let ratio = value / total;
        let angle = 360.0 * ratio;
        if angle <= 0.0 { continue; }
        let end_angle = start_angle + angle;

        let s_rad = start_angle.to_radians();
        let e_rad = end_angle.to_radians();

        let x1 = cx as f64 + r * s_rad.cos();
        let y1 = cy as f64 + r * s_rad.sin();
        let x2 = cx as f64 + r * e_rad.cos();
        let y2 = cy as f64 + r * e_rad.sin();

        let large_arc = if angle > 180.0 { 1 } else { 0 };

        svg.push_str(&format!(
            r#"<path d="M{} {} L{} {} A{} {} 0 {} 1 {} {} Z" fill="{}" stroke="#1a1a2e" stroke-width="2"><title>{}: {:.1}%</title></path>"#,
            cx, cy, x1, y1, r, r, large_arc, x2, y2, color, escape_svg(label), ratio * 100.0
        ));

        start_angle = end_angle;
    }

    // Legend
    let legend_x = w as i32 - 150;
    let mut legend_y = 60;
    for (label, _value, color) in &all_points {
        svg.push_str(&format!(r#"<rect x="{}" y="{}" width="12" height="12" fill="{}" rx="2"/>"#, legend_x, legend_y, color));
        let truncated = if label.len() > 15 { &label[..15] } else { label };
        svg.push_str(&format!(r#"<text x="{}" y="{}" fill="#ccc" font-size="11">{}</text>"#, legend_x + 18, legend_y + 10, escape_svg(truncated)));
        legend_y += 22;
    }

    svg.push_str("</svg>");
    Ok(svg)
}

// ─── HTML Table ────────────────────────────────────────────────────────────

fn render_html_table(req: &ChartRequest) -> Result<String, String> {
    let mut html = format!(r#"<div style="overflow-x:auto;margin:10px 0"><table style="border-collapse:collapse;width:100%;font-family:sans-serif;font-size:14px">"#);
    html.push_str(&format!(r#"<caption style="font-size:16px;font-weight:bold;margin:8px 0;color:#fff">{}</caption>"#, escape_svg(&req.title)));

    // Header row
    html.push_str("<thead><tr style=\"background:#16213e\">");
    html.push_str("<th style=\"padding:8px;border:1px solid #333;color:#00d4ff;text-align:left\">Label</th>");
    for series in &req.series {
        html.push_str(&format!("<th style=\"padding:8px;border:1px solid #333;color:{};text-align:right\">{}</th>", series.color, escape_svg(&series.label)));
    }
    html.push_str("</tr></thead><tbody>");

    // Data rows
    let max_rows = req.series.iter().map(|s| s.points.len()).max().unwrap_or(0);
    for i in 0..max_rows {
        let bg = if i % 2 == 0 { "#1a1a2e" } else { "#16213e" };
        html.push_str(&format!("<tr style=\"background:{}\">", bg));

        let label = req.series.first()
            .and_then(|s| s.points.get(i))
            .map(|p| escape_svg(&p.label))
            .unwrap_or_default();
        html.push_str(&format!("<td style=\"padding:8px;border:1px solid #333;color:#fff\">{}</td>", label));

        for series in &req.series {
            let val = series.points.get(i).map(|p| p.value).unwrap_or(0.0);
            html.push_str(&format!("<td style=\"padding:8px;border:1px solid #333;color:#ccc;text-align:right\">{:.2}</td>", val));
        }
        html.push_str("</tr>");
    }

    html.push_str("</tbody></table></div>");
    Ok(html)
}

fn escape_svg(s: &str) -> String {
    s.chars().map(|c| match c {
        '&' => "&amp;".to_string(),
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '"' => "&apos;".to_string(),
        _ => c.to_string(),
    }).collect()
}
