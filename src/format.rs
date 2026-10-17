//! Display formatting that matches the web app's conventions.

/// A missing value, shown as an em dash like the web tables.
pub const DASH: &str = "—";

pub fn int(value: Option<i64>) -> String {
    value.map(group_thousands).unwrap_or_else(|| DASH.into())
}

pub fn year(value: Option<i32>) -> String {
    value.map(|y| y.to_string()).unwrap_or_else(|| DASH.into())
}

pub fn text(value: Option<&str>) -> String {
    match value {
        Some(v) if !v.trim().is_empty() => v.to_string(),
        _ => DASH.into(),
    }
}

/// Rate stats such as AVG/OBP/SLG: three decimals, leading zero dropped (.342).
pub fn rate(value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_finite() => {
            let s = format!("{v:.3}");
            match s.strip_prefix("0") {
                Some(rest) => rest.to_string(),
                None => s,
            }
        }
        _ => DASH.into(),
    }
}

/// ERA, WHIP and similar: two decimals.
pub fn decimal2(value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_finite() => format!("{v:.2}"),
        _ => DASH.into(),
    }
}

/// Percentages already expressed in 0–100 (vote shares): one decimal and %.
pub fn percent(value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_finite() => format!("{v:.1}%"),
        _ => DASH.into(),
    }
}

/// Innings pitched, kept in baseball notation (1221.1 = 1221⅓ innings).
pub fn innings(value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_finite() => {
            if v.fract() == 0.0 {
                format!("{v:.0}")
            } else {
                format!("{v:.1}")
            }
        }
        _ => DASH.into(),
    }
}

/// A count that may be fractional (award points): whole numbers without decimals.
pub fn decimal_trim(value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_finite() && v.fract() == 0.0 => format!("{v:.0}"),
        Some(v) if v.is_finite() => format!("{v:.1}"),
        _ => DASH.into(),
    }
}

pub fn money(value: Option<i64>) -> String {
    match value {
        Some(v) => format!("${}", group_thousands(v)),
        None => DASH.into(),
    }
}

pub fn year_span(from: Option<i32>, to: Option<i32>) -> String {
    match (from, to) {
        (Some(a), Some(b)) if a == b => a.to_string(),
        (Some(a), Some(b)) => format!("{a}–{b}"),
        (Some(a), None) => format!("{a}–"),
        (None, Some(b)) => b.to_string(),
        (None, None) => DASH.into(),
    }
}

pub fn record(wins: Option<i64>, losses: Option<i64>) -> String {
    match (wins, losses) {
        (Some(w), Some(l)) => format!("{w}–{l}"),
        _ => DASH.into(),
    }
}

pub fn full_name(first: Option<&str>, last: Option<&str>) -> String {
    let name = [first.unwrap_or(""), last.unwrap_or("")]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() { DASH.into() } else { name }
}

/// Height in inches as feet and inches (74 → 6′ 2″).
pub fn height(inches: Option<&str>) -> String {
    match inches.and_then(|s| s.trim().parse::<u32>().ok()) {
        Some(i) if i > 0 => format!("{}′ {}″", i / 12, i % 12),
        _ => DASH.into(),
    }
}

/// Lahman dates arrive as `Y-M-D` without zero padding; render them long form.
pub fn date(value: Option<&str>) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let Some(raw) = value.filter(|v| !v.trim().is_empty()) else {
        return DASH.into();
    };
    let parts: Vec<_> = raw.split('-').map(|p| p.parse::<usize>().ok()).collect();
    match parts.as_slice() {
        [Some(y), Some(m), Some(d)] if (1..=12).contains(m) && *d > 0 => {
            format!("{} {d}, {y}", MONTHS[m - 1])
        }
        [Some(y), Some(m), ..] if (1..=12).contains(m) => format!("{} {y}", MONTHS[m - 1]),
        [Some(y), ..] => y.to_string(),
        _ => raw.to_string(),
    }
}

pub fn bats_throws(code: Option<&str>) -> String {
    match code.map(str::trim) {
        Some("L") => "Left".into(),
        Some("R") => "Right".into(),
        Some("B") => "Both".into(),
        Some("S") => "Switch".into(),
        other => text(other),
    }
}

fn group_thousands(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    if value < 0 {
        out.insert(0, '-');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_match_web_conventions() {
        assert_eq!(rate(Some(0.34210)), ".342");
        assert_eq!(rate(Some(1.162)), "1.162");
        assert_eq!(decimal2(Some(2.2834)), "2.28");
        assert_eq!(int(Some(1164015)), "1,164,015");
        assert_eq!(int(None), DASH);
        assert_eq!(money(Some(33000000)), "$33,000,000");
        assert_eq!(innings(Some(9.0)), "9");
        assert_eq!(innings(Some(1221.3)), "1221.3");
        assert_eq!(height(Some("74")), "6′ 2″");
        assert_eq!(date(Some("1895-2-6")), "February 6, 1895");
        assert_eq!(year_span(Some(1914), Some(1935)), "1914–1935");
        assert_eq!(full_name(Some("Babe"), Some("Ruth")), "Babe Ruth");
        assert_eq!(percent(Some(95.1)), "95.1%");
    }
}
