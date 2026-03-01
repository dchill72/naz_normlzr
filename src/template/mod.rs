use anyhow::{bail, Result};
use std::collections::HashMap;

pub type Vars = HashMap<String, String>;

/// Render a template string substituting `{var}` and `{var:spec}` tokens.
///
/// Format spec `02` → zero-pad numeric value to width 2 (e.g. `"1"` → `"01"`).
/// Unknown variables are substituted with an empty string.
pub fn render(template: &str, vars: &Vars) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Escaped `{{`
            if chars.peek() == Some(&'{') {
                chars.next();
                out.push('{');
                continue;
            }
            // Collect until `}`
            let mut token = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some(c) => token.push(c),
                    None => bail!("Unclosed `{{` in template: {template}"),
                }
            }
            // Split on first `:`
            let (name, spec) = match token.find(':') {
                Some(pos) => (&token[..pos], Some(&token[pos + 1..])),
                None => (token.as_str(), None),
            };
            let value = vars.get(name).map(|s| s.as_str()).unwrap_or("");
            match spec {
                Some(s) => out.push_str(&apply_spec(value, s)),
                None => out.push_str(value),
            }
        } else if ch == '}' && chars.peek() == Some(&'}') {
            chars.next();
            out.push('}');
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn apply_spec(value: &str, spec: &str) -> String {
    // Spec format: [0]<width>  — e.g. "02" or "2"
    let zero_pad = spec.starts_with('0');
    let width_str = spec.trim_start_matches('0');
    let width: usize = match width_str.parse() {
        Ok(w) if w > 0 => w,
        _ => return value.to_string(),
    };

    if let Ok(n) = value.parse::<i64>() {
        if zero_pad {
            format!("{:0>width$}", n, width = width)
        } else {
            format!("{:>width$}", n, width = width)
        }
    } else {
        format!("{:>width$}", value, width = width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> Vars {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn plain_substitution() {
        let v = vars(&[("title", "Alien"), ("year", "1979")]);
        assert_eq!(render("{title} ({year})", &v).unwrap(), "Alien (1979)");
    }

    #[test]
    fn zero_padded() {
        let v = vars(&[("season", "1"), ("episode", "2")]);
        assert_eq!(
            render("S{season:02}E{episode:02}", &v).unwrap(),
            "S01E02"
        );
    }

    #[test]
    fn missing_var_is_empty() {
        let v = vars(&[("title", "Alien")]);
        assert_eq!(render("{title} ({year})", &v).unwrap(), "Alien ()");
    }

    #[test]
    fn escaped_braces() {
        let v = vars(&[]);
        assert_eq!(render("{{literal}}", &v).unwrap(), "{literal}");
    }
}
