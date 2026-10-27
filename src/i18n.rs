//! Embedded message catalog: the same executable works in local and Flatpak installs.
use std::{
    collections::BTreeMap,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};
static ENGLISH: AtomicBool = AtomicBool::new(false);
static CATALOG: OnceLock<BTreeMap<String, String>> = OnceLock::new();
pub fn init(language: &str) {
    let selected = if language.is_empty() {
        std::env::var("LANGUAGE")
            .or_else(|_| std::env::var("LC_ALL"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default()
    } else {
        language.into()
    };
    ENGLISH.store(selected.starts_with("en"), Ordering::Relaxed);
}
pub fn tr(text: &str) -> String {
    if !ENGLISH.load(Ordering::Relaxed) {
        return text.into();
    }
    let catalog = CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../po/en.json")).expect("validated translation catalog")
    });
    if let Some(translated) = catalog.get(text) {
        return translated.clone();
    }
    for (source, translated) in catalog
        .iter()
        .filter(|(key, value)| key.contains('{') && key != value)
    {
        if let Some(result) = substitute(source, translated, text) {
            return result;
        }
    }
    text.into()
}
pub fn dropdown(strings: &[&str]) -> gtk::DropDown {
    let labels: Vec<_> = strings.iter().map(|s| tr(s)).collect();
    gtk::DropDown::from_strings(&labels.iter().map(|s| s.as_str()).collect::<Vec<_>>())
}
fn pieces(template: &str) -> (Vec<&str>, Vec<String>) {
    let mut literals = vec![];
    let mut names = vec![];
    let mut rest = template;
    let mut position = 0;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else {
            break;
        };
        literals.push(&rest[..start]);
        let key = rest[start + 1..start + end].split(':').next().unwrap_or("");
        names.push(if key.is_empty() {
            let n = position;
            position += 1;
            n.to_string()
        } else {
            key.into()
        });
        rest = &rest[start + end + 1..];
    }
    literals.push(rest);
    (literals, names)
}
fn substitute(source: &str, translated: &str, text: &str) -> Option<String> {
    let (literals, names) = pieces(source);
    if names.is_empty() {
        return None;
    }
    let mut rest = text.strip_prefix(literals[0])?;
    let mut values = BTreeMap::new();
    for (i, name) in names.iter().enumerate() {
        let next = literals[i + 1];
        let end = if next.is_empty() {
            if i + 1 == names.len() {
                rest.len()
            } else {
                return None;
            }
        } else {
            rest.find(next)?
        };
        values.insert(name.as_str(), &rest[..end]);
        rest = &rest[end + next.len()..];
    }
    if !rest.is_empty() {
        return None;
    }
    let (parts, translated_names) = pieces(translated);
    let mut result = parts[0].to_string();
    for (i, name) in translated_names.iter().enumerate() {
        result.push_str(values.get(name.as_str())?);
        result.push_str(parts[i + 1]);
    }
    Some(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatted_messages_preserve_user_values() {
        assert_eq!(
            substitute(
                "Registro {}/{} · {name}",
                "{name} · Record {}/{}",
                "Registro 2/4 · Mi rutina"
            ),
            Some("Mi rutina · Record 2/4".into())
        );
        assert_eq!(substitute("Registro {}", "Record {}", "Mi rutina"), None);
    }
    #[test]
    fn catalog_is_valid_and_placeholders_are_preserved() {
        let catalog: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../po/en.json")).unwrap();
        for (source, target) in catalog {
            let (_, mut a) = pieces(&source);
            let (_, mut b) = pieces(&target);
            a.sort();
            b.sort();
            assert_eq!(a, b, "{source}");
        }
    }
}
