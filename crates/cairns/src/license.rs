//! The project's licenses, when `cairns.toml` does not say.
//!
//! Only what is certain: a license declared where a package manifest declares
//! one, or a license file whose text is recognisably one license. A wrong
//! license shown on every page is worse than none, so nothing is guessed from
//! a partial match.
//!
//! The text - the log, the reference - has a license of its own when a file
//! says so: one named for it (`LICENSE-docs`), one in the directory it covers
//! (`docs/LICENSE`), or a Creative Commons license beside a software one, which
//! cannot be the code's - Creative Commons advise against their licenses for
//! software, so the pair is read as code and prose, not as a choice.

use std::path::Path;

use cairns_core::log::LicenseFile;

/// A license as an SPDX expression and what it was found in.
pub struct Found {
    pub expression: String,
    pub from: String,
}

/// What was found: the code's license, the text's when it has its own, and
/// every license file, each with the license its text is, for the site to
/// link a license to its own file.
pub struct Licenses {
    pub code: Option<Found>,
    pub text: Option<Found>,
    pub files: Vec<LicenseFile>,
}

/// Look in the root, and in `text_dirs` - the worklog's and the reference's
/// directories - for a license of the text's own.
pub fn detect(root: &Path, text_dirs: &[&str]) -> Licenses {
    let at_root: Vec<LicenseFile> = license_files(root, "");
    let software = |file: &LicenseFile| {
        file.license
            .as_deref()
            .is_some_and(|id| !creative_commons_for_prose(id))
    };
    let for_text = |file: &LicenseFile| {
        named_for_text(&file.path)
            || (file
                .license
                .as_deref()
                .is_some_and(creative_commons_for_prose)
                && at_root.iter().any(software))
    };
    let (text_files, code_files): (Vec<&LicenseFile>, Vec<&LicenseFile>) =
        at_root.iter().partition(|file| for_text(file));
    let mut text_files: Vec<LicenseFile> = text_files.into_iter().cloned().collect();
    for dir in text_dirs {
        let dir = dir.trim_matches('/');
        if !dir.is_empty() && dir != "." {
            text_files.extend(license_files(&root.join(dir), dir));
        }
    }
    let code = manifest(root).or_else(|| one_license(&code_files));
    // The text's files are in different places, not offered side by side: two
    // different licenses among them is a disagreement, not a choice.
    let text = one_license(&text_files.iter().collect::<Vec<_>>())
        .filter(|text| !text.expression.contains(" OR "));
    let mut files: Vec<LicenseFile> = code_files.into_iter().cloned().collect();
    files.extend(text_files);
    Licenses { code, text, files }
}

/// The license some files hold between them, when every one is known:
/// `LICENSE-MIT` and `LICENSE-APACHE` side by side are the convention for a
/// choice between the two.
fn one_license(files: &[&LicenseFile]) -> Option<Found> {
    let mut ids: Vec<&str> = Vec::new();
    for file in files {
        let id = file.license.as_deref()?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    (!ids.is_empty()).then(|| Found {
        expression: ids.join(" OR "),
        from: files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    })
}

/// A license file whose name says it is the text's: `LICENSE-docs`,
/// `LICENSE.content.md`, `COPYING-TEXT`.
fn named_for_text(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    let rest = ["license", "licence", "copying"]
        .iter()
        .find_map(|prefix| name.strip_prefix(prefix))
        .unwrap_or("");
    ["doc", "text", "content", "prose", "writing"]
        .iter()
        .any(|word| rest.contains(word))
}

/// A Creative Commons license meant for prose, not code. CC0 is not one: it is
/// used for code as often as for anything.
fn creative_commons_for_prose(id: &str) -> bool {
    id.starts_with("CC-BY")
}

/// A manifest's own declaration: Cargo's, npm's, Python's.
fn manifest(root: &Path) -> Option<Found> {
    let read = |name: &str| std::fs::read_to_string(root.join(name)).ok();
    let found = |expression: &str, from: &str| {
        let expression = expression.trim();
        (!expression.is_empty()).then(|| Found {
            expression: expression.to_string(),
            from: from.to_string(),
        })
    };
    if let Some(text) = read("Cargo.toml")
        && let Ok(cargo) = text.parse::<toml::Table>()
    {
        let license = cargo
            .get("package")
            .and_then(|p| p.get("license"))
            .and_then(|l| l.as_str())
            .or_else(|| {
                cargo
                    .get("workspace")
                    .and_then(|w| w.get("package"))
                    .and_then(|p| p.get("license"))
                    .and_then(|l| l.as_str())
            });
        if let Some(license) = license {
            return found(license, "Cargo.toml");
        }
    }
    if let Some(text) = read("package.json")
        && let Ok(package) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(license) = package.get("license").and_then(|l| l.as_str())
    {
        return found(license, "package.json");
    }
    if let Some(text) = read("pyproject.toml")
        && let Ok(project) = text.parse::<toml::Table>()
        && let Some(license) = project
            .get("project")
            .and_then(|p| p.get("license"))
            .and_then(|l| l.as_str().or_else(|| l.get("text").and_then(|t| t.as_str())))
        // `license = { text = "..." }` may hold a whole license's text; only a
        // short value is an identifier.
        && license.len() < 60
        && !license.contains('\n')
    {
        return found(license, "pyproject.toml");
    }
    None
}

/// The license files in `dir` - LICENSE, LICENSE.md, LICENSE-MIT, COPYING
/// and the like - each with the license its text is, named by their path from
/// the root: `prefix` is `dir`'s.
fn license_files(dir: &Path, prefix: &str) -> Vec<LicenseFile> {
    let Ok(items) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = items
        .flatten()
        .filter(|item| item.path().is_file())
        .map(|item| item.file_name().to_string_lossy().to_string())
        .filter(|name| {
            let upper = name.to_ascii_uppercase();
            upper.starts_with("LICENSE")
                || upper.starts_with("LICENCE")
                || upper.starts_with("COPYING")
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| LicenseFile {
            license: std::fs::read_to_string(dir.join(&name))
                .ok()
                .and_then(|text| identify(&text))
                .map(str::to_string),
            path: if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            },
        })
        .collect()
}

/// A license's SPDX id from its text, when the text is unmistakably that
/// license: each is known by phrases its own text has and the others' do not.
pub fn identify(text: &str) -> Option<&'static str> {
    let text: String = text
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let has = |phrase: &str| text.contains(phrase);
    let all = |phrases: &[&str]| phrases.iter().all(|p| has(p));
    let id = if all(&["apache license", "version 2.0, january 2004"]) {
        "Apache-2.0"
    } else if all(&["gnu affero general public license", "version 3"]) {
        "AGPL-3.0"
    } else if all(&["gnu lesser general public license", "version 3"]) {
        "LGPL-3.0"
    } else if all(&["gnu lesser general public license", "version 2.1"]) {
        "LGPL-2.1"
    } else if all(&["gnu general public license", "version 3, 29 june 2007"]) {
        "GPL-3.0"
    } else if all(&["gnu general public license", "version 2, june 1991"]) {
        "GPL-2.0"
    } else if has("mozilla public license version 2.0") {
        "MPL-2.0"
    } else if all(&[
        "permission is hereby granted, free of charge, to any person obtaining a copy",
        "the above copyright notice and this permission notice shall be included",
    ]) {
        "MIT"
    } else if has(
        "permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted",
    ) {
        if has("the above copyright notice and this permission notice appear in all copies") {
            "ISC"
        } else {
            "0BSD"
        }
    } else if has(
        "redistribution and use in source and binary forms, with or without modification, are permitted",
    ) {
        if has("neither the name of") {
            "BSD-3-Clause"
        } else {
            "BSD-2-Clause"
        }
    } else if has("this is free and unencumbered software released into the public domain") {
        "Unlicense"
    } else if has("cc0 1.0 universal") {
        "CC0-1.0"
    } else {
        creative_commons(&text)?
    };
    Some(id)
}

/// Every Creative Commons attribution license SPDX names, ported ones
/// included. Nothing outside this list is ever produced.
const CREATIVE_COMMONS: &[&str] = &[
    "CC-BY-1.0",
    "CC-BY-2.0",
    "CC-BY-2.5",
    "CC-BY-2.5-AU",
    "CC-BY-3.0",
    "CC-BY-3.0-AT",
    "CC-BY-3.0-AU",
    "CC-BY-3.0-DE",
    "CC-BY-3.0-IGO",
    "CC-BY-3.0-NL",
    "CC-BY-3.0-US",
    "CC-BY-4.0",
    "CC-BY-NC-1.0",
    "CC-BY-NC-2.0",
    "CC-BY-NC-2.5",
    "CC-BY-NC-3.0",
    "CC-BY-NC-3.0-DE",
    "CC-BY-NC-3.0-IGO",
    "CC-BY-NC-4.0",
    "CC-BY-NC-ND-1.0",
    "CC-BY-NC-ND-2.0",
    "CC-BY-NC-ND-2.5",
    "CC-BY-NC-ND-3.0",
    "CC-BY-NC-ND-3.0-DE",
    "CC-BY-NC-ND-3.0-IGO",
    "CC-BY-NC-ND-4.0",
    "CC-BY-NC-SA-1.0",
    "CC-BY-NC-SA-2.0",
    "CC-BY-NC-SA-2.0-DE",
    "CC-BY-NC-SA-2.0-FR",
    "CC-BY-NC-SA-2.0-UK",
    "CC-BY-NC-SA-2.5",
    "CC-BY-NC-SA-3.0",
    "CC-BY-NC-SA-3.0-DE",
    "CC-BY-NC-SA-3.0-IGO",
    "CC-BY-NC-SA-4.0",
    "CC-BY-ND-1.0",
    "CC-BY-ND-2.0",
    "CC-BY-ND-2.5",
    "CC-BY-ND-3.0",
    "CC-BY-ND-3.0-DE",
    "CC-BY-ND-4.0",
    "CC-BY-SA-1.0",
    "CC-BY-SA-2.0",
    "CC-BY-SA-2.0-UK",
    "CC-BY-SA-2.1-JP",
    "CC-BY-SA-2.5",
    "CC-BY-SA-3.0",
    "CC-BY-SA-3.0-AT",
    "CC-BY-SA-3.0-DE",
    "CC-BY-SA-3.0-IGO",
    "CC-BY-SA-4.0",
];

/// A Creative Commons attribution license, any version, from the three ways a
/// file names one in full:
///
/// - its name - "Creative Commons Attribution-ShareAlike 3.0 Unported", as the
///   notice has it, or after "Creative Commons Legal Code", as the legal code
///   does;
/// - its address - `creativecommons.org/licenses/by-sa/3.0/de/`;
/// - its short form - "CC BY-SA 4.0".
///
/// Before 4.0 a license could be ported to a jurisdiction, and a port is a
/// different license: "3.0 Germany" is `CC-BY-3.0-DE`, not `CC-BY-3.0`. So
/// for those versions whatever follows the version must be a jurisdiction
/// known here or a word that says there is none - anything else and nothing
/// is named.
fn creative_commons(text: &str) -> Option<&'static str> {
    if !(text.contains("creative commons")
        || text.contains("creativecommons.org")
        || text.contains("cc by")
        || text.contains("cc-by"))
    {
        return None;
    }
    // Words, with an empty one wherever punctuation ends a phrase, so that
    // "CC BY 3.0." is known to end at the version.
    let mut words: Vec<&str> = Vec::new();
    for raw in text.split(|c: char| c.is_whitespace() || c == '/' || c == '-' || c == '_') {
        let mut rest = raw;
        let mut ended = false;
        while let Some(stripped) = rest
            .strip_suffix(|c: char| ",;:()[]<>\"'!?*.".contains(c))
            .or_else(|| rest.strip_prefix(|c: char| ",;:()[]<>\"'!?*".contains(c)))
        {
            rest = stripped;
            ended = true;
        }
        if !rest.is_empty() {
            words.push(rest);
        }
        if ended {
            words.push("");
        }
    }
    (0..words.len()).find_map(|at| {
        let after = match &words[at..] {
            ["creative", "commons", "attribution", ..] => at + 3,
            // The legal code before 4.0 opens "Creative Commons Legal Code",
            // then the license's name.
            ["creative", "commons", "legal", "code", "attribution", ..] => at + 5,
            ["cc", "by", ..] => at + 2,
            ["ccby", ..] => at + 1,
            [address, "licenses", "by", ..] if address.ends_with("creativecommons.org") => at + 3,
            _ => return None,
        };
        let mut next = after;
        let (mut nc, mut sa, mut nd) = (false, false, false);
        loop {
            match words.get(next).copied() {
                Some("noncommercial" | "nc") => nc = true,
                Some("sharealike" | "sa") => sa = true,
                Some("noderivs" | "noderivatives" | "nd") => nd = true,
                _ => break,
            }
            next += 1;
        }
        let version = *words.get(next)?;
        if !["1.0", "2.0", "2.1", "2.5", "3.0", "4.0"].contains(&version) {
            return None;
        }
        let place = match version {
            "4.0" => None,
            _ => match words.get(next + 1).copied().unwrap_or("") {
                "" | "unported" | "generic" | "international" | "license" | "licence"
                | "public" | "legalcode" | "deed" | "deed.en" | "legalcode.en" => None,
                "igo" => Some("IGO"),
                "us" | "united" => Some("US"),
                "de" | "germany" | "deutschland" => Some("DE"),
                "at" | "austria" | "österreich" => Some("AT"),
                "au" | "australia" => Some("AU"),
                "nl" | "netherlands" => Some("NL"),
                "uk" | "england" => Some("UK"),
                "fr" | "france" => Some("FR"),
                "jp" | "japan" => Some("JP"),
                _ => return None,
            },
        };
        let mut id = String::from("CC-BY");
        for (on, part) in [(nc, "-NC"), (sa, "-SA"), (nd, "-ND")] {
            if on {
                id.push_str(part);
            }
        }
        id.push('-');
        id.push_str(version);
        if let Some(place) = place {
            id.push('-');
            id.push_str(place);
        }
        CREATIVE_COMMONS.iter().find(|known| **known == id).copied()
    })
}

#[cfg(test)]
mod tests {
    use super::{Found, detect, identify};

    #[test]
    fn a_license_is_known_by_its_own_wording() {
        let mit = "MIT License\n\nCopyright (c) 2026 Toyz\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\nof this software ... The above copyright notice and this permission notice shall be included in all\ncopies";
        assert_eq!(identify(mit), Some("MIT"));
        assert_eq!(
            identify(
                "                                 Apache License\n                           Version 2.0, January 2004\n"
            ),
            Some("Apache-2.0")
        );
        assert_eq!(
            identify("GNU GENERAL PUBLIC LICENSE\n Version 2, June 1991"),
            Some("GPL-2.0")
        );
        assert_eq!(
            identify(
                "Redistribution and use in source and binary forms, with or without modification, are permitted ... Neither the name of"
            ),
            Some("BSD-3-Clause")
        );
        // A text that names a license without being it is not that license.
        assert_eq!(
            identify("This project is MIT licensed, see the website."),
            None
        );
        assert_eq!(identify("All rights reserved."), None);
    }

    #[test]
    fn a_creative_commons_license_is_known_by_its_notice_too() {
        assert_eq!(
            identify(
                "This work is licensed under a Creative Commons\nAttribution-ShareAlike 4.0 International License."
            ),
            Some("CC-BY-SA-4.0")
        );
        assert_eq!(
            identify("Creative Commons Attribution 4.0 International Public License"),
            Some("CC-BY-4.0")
        );
        // Before 4.0: by name, by address, by short form, ported or not.
        let cases = [
            (
                "Creative Commons Attribution-ShareAlike 3.0 Unported License",
                Some("CC-BY-SA-3.0"),
            ),
            (
                "Creative Commons Attribution-NonCommercial-NoDerivs 3.0 Germany",
                Some("CC-BY-NC-ND-3.0-DE"),
            ),
            (
                "Creative Commons Attribution-NoDerivs-NonCommercial 1.0",
                Some("CC-BY-NC-ND-1.0"),
            ),
            (
                "Creative Commons Attribution 2.5 Generic",
                Some("CC-BY-2.5"),
            ),
            (
                "see https://creativecommons.org/licenses/by-nc-sa/2.0/uk/",
                Some("CC-BY-NC-SA-2.0-UK"),
            ),
            (
                "<http://creativecommons.org/licenses/by/3.0/>",
                Some("CC-BY-3.0"),
            ),
            ("The docs are CC BY-SA 3.0.", Some("CC-BY-SA-3.0")),
            ("The docs are CC BY 4.0 like the rest", Some("CC-BY-4.0")),
            // A port named here but not one SPDX has, or a word that could be
            // one: not the unported license.
            ("Creative Commons Attribution 3.0 Spain", None),
            ("Creative Commons Attribution-ShareAlike 2.1 Germany", None),
            ("We used CC BY 3.0 for a while", None),
        ];
        for (text, id) in cases {
            assert_eq!(identify(text), id, "{text}");
        }
    }

    #[test]
    fn the_text_has_its_own_license_when_a_file_says_so() {
        let mit = "Permission is hereby granted, free of charge, to any person obtaining a copy ... The above copyright notice and this permission notice shall be included";
        let cc = "Licensed under a Creative Commons Attribution 4.0 International License.";
        let scratch = |files: &[(&str, &str)]| {
            let root = std::env::temp_dir().join(format!(
                "cairns-license-{}-{}",
                std::process::id(),
                files.len() * 31 + files[0].0.len()
            ));
            let _ = std::fs::remove_dir_all(&root);
            for (path, text) in files {
                let path = root.join(path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, text).unwrap();
            }
            let found = detect(&root, &["docs", "worklog"]);
            let _ = std::fs::remove_dir_all(&root);
            let take = |found: Option<Found>| found.map(|f| f.expression);
            (take(found.code), take(found.text))
        };
        // Beside the code's: a CC license is the prose's, not a choice.
        assert_eq!(
            scratch(&[("LICENSE", mit), ("LICENSE-CC-BY", cc)]),
            (Some("MIT".into()), Some("CC-BY-4.0".into()))
        );
        // Named for the text, or in the text's own directory.
        assert_eq!(
            scratch(&[("LICENSE", mit), ("LICENSE-docs.md", cc)]).1,
            Some("CC-BY-4.0".into())
        );
        assert_eq!(
            scratch(&[("LICENSE", mit), ("docs/LICENSE.md", cc)]).1,
            Some("CC-BY-4.0".into())
        );
        // Alone, it is everything's.
        assert_eq!(
            scratch(&[("LICENSE", cc)]),
            (Some("CC-BY-4.0".into()), None)
        );
        // Two different licenses for the text is not one.
        assert_eq!(
            scratch(&[
                ("LICENSE", mit),
                ("docs/LICENSE", cc),
                ("worklog/LICENSE", mit)
            ])
            .1,
            None
        );
    }
}
