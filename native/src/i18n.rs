// SPDX-License-Identifier: GPL-3.0-or-later
//! Six UN languages. Bidi runs use egui's native HarfRust shaping, preserving data.
use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

#[derive(Clone, Debug, Default)]
pub struct UiText(String);

impl UiText {
    pub fn heading(self) -> eframe::egui::WidgetText {
        let widget: eframe::egui::WidgetText = self.into();
        if let eframe::egui::WidgetText::LayoutJob(mut job) = widget {
            for section in &mut std::sync::Arc::make_mut(&mut job).sections {
                section.format.font_id.size = 20.;
            }
            eframe::egui::WidgetText::LayoutJob(job)
        } else if let eframe::egui::WidgetText::RichText(text) = widget {
            std::sync::Arc::unwrap_or_clone(text).heading().into()
        } else {
            widget
        }
    }
}

impl std::fmt::Display for UiText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::ops::Deref for UiText {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}
impl From<String> for UiText {
    fn from(text: String) -> Self {
        Self(text)
    }
}
impl From<&str> for UiText {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}
impl From<UiText> for String {
    fn from(text: UiText) -> Self {
        text.0
    }
}
impl From<UiText> for eframe::egui::RichText {
    fn from(text: UiText) -> Self {
        Self::new(text.0)
    }
}
impl From<&UiText> for eframe::egui::WidgetText {
    fn from(text: &UiText) -> Self {
        text.clone().into()
    }
}
impl From<UiText> for eframe::egui::WidgetText {
    fn from(text: UiText) -> Self {
        use eframe::egui::{
            Color32,
            text::{LayoutJob, TextFormat},
        };
        let bidi = unicode_bidi::BidiInfo::new(&text.0, None);
        if !bidi.has_rtl() {
            return text.0.into();
        }
        let mut job = LayoutJob::default();
        for paragraph in &bidi.paragraphs {
            let (_, runs) = bidi.visual_runs(paragraph, paragraph.range.clone());
            for run in runs {
                // Runs remain logical: HarfRust joins and shapes RTL glyphs.
                // Reversing characters first would reverse the glyphs twice.
                let start = job.text.len();
                job.text.extend(
                    text.0[run].chars().filter(|c| {
                        !matches!(c, '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}')
                    }),
                );
                if job.text.len() == start {
                    continue;
                }
                job.sections.push(eframe::egui::text::LayoutSection {
                    leading_space: 0.,
                    byte_range: eframe::egui::text::ByteIndex(start)
                        ..eframe::egui::text::ByteIndex(job.text.len()),
                    format: TextFormat {
                        color: Color32::PLACEHOLDER,
                        ..Default::default()
                    },
                });
            }
        }
        Self::LayoutJob(std::sync::Arc::new(job))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    English,
    Chinese,
    French,
    Spanish,
    Russian,
    Arabic,
}

impl Language {
    pub const ALL: [Self; 6] = [
        Self::English,
        Self::Chinese,
        Self::French,
        Self::Spanish,
        Self::Russian,
        Self::Arabic,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Chinese => "zh-CN",
            Self::French => "fr",
            Self::Spanish => "es",
            Self::Russian => "ru",
            Self::Arabic => "ar",
        }
    }

    pub fn name(self) -> UiText {
        visual_text(match self {
            Self::English => "English",
            Self::Chinese => "简体中文",
            Self::French => "Français",
            Self::Spanish => "Español",
            Self::Russian => "Русский",
            Self::Arabic => "العربية",
        })
    }

    pub fn parse(code: &str) -> Self {
        match code
            .to_ascii_lowercase()
            .split(['-', '_', '.'])
            .next()
            .unwrap_or("")
        {
            "zh" => Self::Chinese,
            "fr" => Self::French,
            "es" => Self::Spanish,
            "ru" => Self::Russian,
            "ar" => Self::Arabic,
            _ => Self::English,
        }
    }

    pub fn detect() -> Self {
        if let Ok(code) = std::env::var("NPZSEE_LANG") {
            return Self::parse(&code);
        }
        if let Some(path) = settings_path()
            && let Ok(text) = std::fs::read_to_string(path)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(code) = value["language"].as_str()
        {
            return Self::parse(code);
        }
        Self::parse(&sys_locale::get_locale().unwrap_or_default())
    }

    pub fn save(self) -> std::io::Result<()> {
        if let Some(path) = settings_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                path,
                serde_json::json!({"language": self.code()}).to_string(),
            )?;
        }
        Ok(())
    }

    pub fn text(self, key: &str) -> UiText {
        visual_text(self.logical(key))
    }

    pub fn logical_text(self, key: &str) -> String {
        self.logical(key).to_owned()
    }

    pub fn format(self, key: &str, values: &[(&str, String)]) -> UiText {
        let mut text = self.logical(key).to_owned();
        for (name, value) in values {
            // Isolate inserted indices/paths/numbers from surrounding RTL text.
            // This keeps [2, 3], file extensions and negative values readable.
            let value = if self == Self::Arabic {
                let rtl = unicode_bidi::BidiInfo::new(value, None).has_rtl();
                format!(
                    "{}{value}\u{2069}",
                    if rtl { '\u{2067}' } else { '\u{2066}' }
                )
            } else {
                value.clone()
            };
            text = text.replace(&format!("{{{name}}}"), &value);
        }
        visual_text(&text)
    }

    fn logical(self, key: &str) -> &str {
        let dictionaries = dictionaries();
        dictionaries[self as usize]
            .get(key)
            .or_else(|| dictionaries[0].get(key))
            .map(String::as_str)
            .unwrap_or(key)
    }
}

fn settings_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("NPZSEE_CONFIG_DIR") {
        return Some(PathBuf::from(path).join("settings.json"));
    }
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("NPZSee"));
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(|p| PathBuf::from(p).join("Library/Application Support/NPZSee"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .map(|p| p.join("npzsee"));
    base.map(|p| p.join("settings.json"))
}

fn dictionaries() -> &'static [BTreeMap<String, String>; 6] {
    static DICTIONARIES: OnceLock<[BTreeMap<String, String>; 6]> = OnceLock::new();
    DICTIONARIES.get_or_init(|| {
        [
            include_str!("../../locales/en.json"),
            include_str!("../../locales/zh-CN.json"),
            include_str!("../../locales/fr.json"),
            include_str!("../../locales/es.json"),
            include_str!("../../locales/ru.json"),
            include_str!("../../locales/ar.json"),
        ]
        .map(|text| serde_json::from_str(text).expect("validated translation catalogue"))
    })
}

pub fn visual_text(text: &str) -> UiText {
    UiText(text.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholders(text: &str) -> Vec<&str> {
        let mut output: Vec<_> = text
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect();
        output.sort_unstable();
        output
    }

    #[test]
    fn all_languages_have_the_same_keys_and_format_arguments() {
        let dictionaries = dictionaries();
        for language in Language::ALL {
            let dictionary = &dictionaries[language as usize];
            assert_eq!(
                dictionary.keys().collect::<Vec<_>>(),
                dictionaries[0].keys().collect::<Vec<_>>()
            );
            for (key, value) in dictionary {
                assert!(!value.is_empty(), "{} {key}", language.code());
                assert_eq!(
                    placeholders(value),
                    placeholders(&dictionaries[0][key]),
                    "{} {key}",
                    language.code()
                );
            }
        }
    }

    #[test]
    fn arabic_uses_shaping_runs_and_preserves_original_unicode() {
        let original = "فتح NPZSee 123";
        let widget: eframe::egui::WidgetText = visual_text(original).into();
        let eframe::egui::WidgetText::LayoutJob(job) = widget else {
            panic!("expected bidi layout")
        };
        assert!(job.text.contains("NPZSee 123"));
        assert!(job.sections.len() >= 2);
        assert!(job.text.contains("فتح"));
        assert_eq!(String::from(visual_text(original)), original);
    }

    #[test]
    fn array_indices_are_isolated_from_surrounding_arabic_text() {
        let widget: eframe::egui::WidgetText = Language::Arabic
            .format("slice", &[("shape", "[2, 3]".into()), ("rows", "2".into())])
            .into();
        let eframe::egui::WidgetText::LayoutJob(job) = widget else {
            panic!("expected bidi layout")
        };
        assert!(job.sections.iter().any(|section| &job.text
            [section.byte_range.start.0..section.byte_range.end.0]
            == "[2, 3]"));
        assert!(!job.text.contains(['\u{2066}', '\u{2067}', '\u{2069}']));
    }
}
