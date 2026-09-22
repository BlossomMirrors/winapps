use std::path::PathBuf;

#[derive(serde::Serialize, Clone, Default)]
pub(crate) struct Match {
    pub(crate) title: String,
    pub(crate) store: String,
    pub(crate) gameid: String,
    pub(crate) acronym: String,
}

pub(crate) fn database_file() -> Option<PathBuf> {
    let path = crate::proton::installed()?
        .join("protonfixes")
        .join("umu-database.csv");
    path.is_file().then_some(path)
}

fn split_row(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(std::mem::take(&mut field)),
            _ => field.push(c),
        }
    }
    fields.push(field);
    fields
}

fn normalise(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub(crate) fn entries() -> Vec<Match> {
    let Some(path) = database_file() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    text.lines()
        .skip(1)
        .filter_map(|line| {
            let row = split_row(line);
            if row.len() < 4 || row[3].trim().is_empty() {
                return None;
            }
            Some(Match {
                title: row[0].trim().to_string(),
                store: row[1].trim().to_string(),
                gameid: row[3].trim().to_string(),
                acronym: row.get(4).map(|s| s.trim().to_string()).unwrap_or_default(),
            })
        })
        .collect()
}

pub(crate) fn lookup(needle: &str) -> Option<Match> {
    let needle = normalise(needle);
    if needle.is_empty() {
        return None;
    }
    let all = entries();

    if let Some(hit) = all.iter().find(|m| normalise(&m.title) == needle) {
        return Some(hit.clone());
    }
    if let Some(hit) = all
        .iter()
        .find(|m| !m.acronym.is_empty() && normalise(&m.acronym) == needle)
    {
        return Some(hit.clone());
    }
    all.iter()
        .filter(|m| {
            let title = normalise(&m.title);
            title.len() > 3 && (needle.contains(&title) || title.contains(&needle))
        })
        .max_by_key(|m| normalise(&m.title).len())
        .cloned()
}

pub(crate) fn search(needle: &str, limit: usize) -> Vec<Match> {
    let needle = normalise(needle);
    if needle.len() < 2 {
        return Vec::new();
    }
    let mut hits: Vec<Match> = entries()
        .into_iter()
        .filter(|m| normalise(&m.title).contains(&needle))
        .collect();
    hits.sort_by_key(|m| m.title.len());
    hits.truncate(limit);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quoted_rows() {
        let row = split_row(r#"Trackmania,ubisoft,5595,umu-2225070,tm,"also known as 'x, y'","#);
        assert_eq!(row.len(), 7);
        assert_eq!(row[0], "Trackmania");
        assert_eq!(row[3], "umu-2225070");
        assert_eq!(row[5], "also known as 'x, y'");
    }

    #[test]
    fn lookup_finds_known_titles() {
        if database_file().is_none() {
            eprintln!("no compatibility tool installed, database unavailable");
            return;
        }
        let all = entries();
        assert!(all.len() > 100, "only {} rows parsed", all.len());
        assert!(all.iter().all(|m| m.gameid.starts_with("umu-")));

        let sample = all[0].clone();
        let hit = lookup(&sample.title).expect("exact title must match");
        assert_eq!(hit.gameid, sample.gameid);
    }

    #[test]
    fn lookup_ignores_punctuation_and_case() {
        if database_file().is_none() {
            return;
        }
        if let Some(hit) = lookup("age of wonders") {
            assert_eq!(lookup("AgeOfWonders!").map(|m| m.gameid), Some(hit.gameid));
        }
    }
}
