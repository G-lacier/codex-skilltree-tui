use super::Result;
use super::catalog::Config;
use super::catalog::Skill;
use regex_lite::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

fn matches(pattern: &str, value: &str) -> Result<bool> {
    let mut expression = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => expression.push_str(".*"),
            '?' => expression.push('.'),
            '[' => {
                expression.push('[');
                if chars.peek() == Some(&'!') {
                    chars.next();
                    expression.push('^');
                }
                for c in chars.by_ref() {
                    expression.push(c);
                    if c == ']' {
                        break;
                    }
                }
            }
            c => expression.push_str(&regex_lite::escape(&c.to_string())),
        }
    }
    expression.push('$');
    Regex::new(&expression)
        .map(|re| re.is_match(value))
        .map_err(|e| format!("Invalid catalog pattern {pattern}: {e}"))
}

pub fn metadata(path: &Path) -> Result<(String, String)> {
    let mut lines = BufReader::new(fs::File::open(path).map_err(|e| e.to_string())?).lines();
    if lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or_default()
        .trim_start_matches('\u{feff}')
        .trim()
        != "---"
    {
        return Err(format!("Missing skill header: {}", path.display()));
    }
    let mut fields = HashMap::<String, String>::new();
    let mut block: Option<String> = None;
    let mut closed = false;
    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim() == "---" {
            closed = true;
            break;
        }
        if line.starts_with([' ', '\t']) {
            if let Some(key) = &block {
                fields
                    .entry(key.clone())
                    .or_default()
                    .push_str(&format!(" {}", line.trim()));
            }
            continue;
        }
        block = None;
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key != "name" && key != "description" {
            continue;
        }
        let value = value.trim();
        let decoded = if value.starts_with(['>', '|']) {
            block = Some(key.to_string());
            String::new()
        } else if value.starts_with('"') {
            let mut parser = serde_json::Deserializer::from_str(value).into_iter::<String>();
            parser
                .next()
                .ok_or("Empty header value")?
                .map_err(|e| e.to_string())?
        } else if let Some(single) = value.strip_prefix('\'') {
            let mut text = String::new();
            let mut chars = single.chars().peekable();
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '\'' {
                    if chars.peek() == Some(&'\'') {
                        chars.next();
                        text.push('\'');
                    } else {
                        closed = true;
                        break;
                    }
                } else {
                    text.push(c);
                }
            }
            if !closed {
                return Err(format!("Unclosed quote in {}", path.display()));
            }
            text
        } else {
            value.split(" #").next().unwrap_or_default().to_string()
        };
        fields.insert(key.to_string(), decoded);
    }
    let name = fields
        .remove("name")
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let description = fields
        .remove("description")
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if !closed || name.is_empty() || description.is_empty() {
        return Err(format!(
            "Skill needs a closed header, name, and description: {}",
            path.display()
        ));
    }
    Ok((name, description))
}

fn paths(folder: &Path, visited: &mut HashSet<PathBuf>, out: &mut Vec<PathBuf>) -> Result<()> {
    if !visited.insert(folder.canonicalize().map_err(|e| e.to_string())?) {
        return Ok(());
    }
    let skill = folder.join("SKILL.md");
    if skill.is_file() {
        out.push(skill);
        return Ok(());
    }
    let mut entries = fs::read_dir(folder)
        .map_err(|e| e.to_string())?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        if [
            ".git",
            ".skilltree",
            "target",
            "node_modules",
            ".venv",
            "__pycache__",
        ]
        .contains(&entry.file_name().to_string_lossy().as_ref())
        {
            continue;
        }
        if entry.path().is_dir() {
            paths(&entry.path(), visited, out)?;
        }
    }
    Ok(())
}

pub fn scan(root: &Path, config: &Config) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();
    for source in &config.sources {
        let folder = root.join(&source.path);
        if !folder.is_dir() {
            if source.optional {
                continue;
            }
            return Err(format!("Missing source folder: {}", folder.display()));
        }
        let mut files = Vec::new();
        paths(&folder, &mut HashSet::new(), &mut files)?;
        for path in files {
            let suffix = path
                .parent()
                .ok_or("Missing skill parent")?
                .strip_prefix(&folder)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            if source
                .exclude
                .iter()
                .map(|p| matches(p, &suffix))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .any(|m| m)
            {
                continue;
            }
            let (name, description) = metadata(&path)?;
            let mut skill = Skill {
                id: format!(
                    "{}/{}",
                    source.id,
                    if suffix.is_empty() { &name } else { &suffix }
                ),
                name,
                description,
                path,
                class: "unassigned".into(),
                tree: "general".into(),
                tags: Vec::new(),
                requires: Vec::new(),
                bytes: 0,
            };
            for rule in &config.rules {
                if rule.source.as_ref().is_some_and(|id| id != &source.id) {
                    continue;
                }
                if let Some(id) = &rule.id
                    && !matches(id, &skill.id)?
                {
                    continue;
                }
                if let Some(names) = &rule.names
                    && !names.is_empty()
                    && !names
                        .iter()
                        .map(|p| matches(&p.to_lowercase(), &skill.name.to_lowercase()))
                        .collect::<Result<Vec<_>>>()?
                        .into_iter()
                        .any(|m| m)
                {
                    continue;
                }
                if let Some(class) = &rule.class {
                    skill.class.clone_from(class);
                }
                if let Some(tree) = &rule.tree {
                    skill.tree.clone_from(tree);
                }
                if let Some(tags) = &rule.tags {
                    skill.tags.clone_from(tags);
                }
                if let Some(requires) = &rule.requires {
                    skill.requires.clone_from(requires);
                }
            }
            if !config
                .classes
                .iter()
                .any(|c| c.id == skill.class && c.trees.iter().any(|t| t.id == skill.tree))
            {
                return Err(format!("Unknown class/tree for {}", skill.id));
            }
            skill.bytes = fs::metadata(&skill.path).map_err(|e| e.to_string())?.len() as usize;
            skills.push(skill);
        }
    }
    skills.sort_by(|a, b| a.id.cmp(&b.id));
    if skills.windows(2).any(|pair| pair[0].id == pair[1].id) {
        return Err("Duplicate skill IDs".into());
    }
    Ok(skills)
}
