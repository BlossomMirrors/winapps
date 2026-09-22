use std::path::Path;

pub(crate) type Values = Vec<(&'static str, String)>;

pub(crate) fn set(prefix: &Path, keys: &[(&str, Values)]) -> Result<(), String> {
    let path = prefix.join("user.reg");
    let mut text =
        std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    for (key, values) in keys {
        text = set_key(&text, key, values);
    }
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn set_key(text: &str, key: &str, values: &Values) -> String {
    let header = format!("[{}]", key.replace('\\', "\\\\"));
    let header_with_stamp = format!("{header} ");
    let mut pending: Vec<&(&str, String)> = values.iter().collect();
    let mut out = String::with_capacity(text.len() + 64 * values.len());
    let mut in_key = false;
    let mut found = false;
    let mut skipping_continuation = false;

    for line in text.lines() {
        if skipping_continuation {
            skipping_continuation = line.ends_with('\\');
            continue;
        }
        if line.starts_with('[') {
            if in_key {
                flush(&mut out, &mut pending);
            }
            in_key = line == header || line.starts_with(&header_with_stamp);
            found |= in_key;
        } else if in_key {
            if line.is_empty() {
                flush(&mut out, &mut pending);
            } else if let Some(index) = pending.iter().position(|(name, _)| names(line, name)) {
                let (name, value) = pending.remove(index);
                out.push_str(&format!("\"{name}\"={value}\n"));
                skipping_continuation = line.ends_with('\\');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if in_key {
        flush(&mut out, &mut pending);
    }
    if !found {
        while out.ends_with("\n\n") {
            out.pop();
        }
        out.push('\n');
        out.push_str(&header);
        out.push('\n');
        flush(&mut out, &mut pending);
    }
    out
}

fn names(line: &str, name: &str) -> bool {
    let quoted = format!("\"{name}\"=");
    line.get(..quoted.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(&quoted))
}

fn flush(out: &mut String, pending: &mut Vec<&(&str, String)>) {
    for (name, value) in pending.drain(..) {
        out.push_str(&format!("\"{name}\"={value}\n"));
    }
}
