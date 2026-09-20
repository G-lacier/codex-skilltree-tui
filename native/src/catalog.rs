use super::Result;
use super::scan;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Branch {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Class {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trees: Vec<Branch>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Source {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Rule {
    pub id: Option<String>,
    pub source: Option<String>,
    pub names: Option<Vec<String>>,
    pub class: Option<String>,
    pub tree: Option<String>,
    pub tags: Option<Vec<String>>,
    pub requires: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub version: u32,
    pub default_budget: usize,
    pub sources: Vec<Source>,
    pub classes: Vec<Class>,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub class: String,
    pub tree: String,
    pub tags: Vec<String>,
    pub requires: Vec<String>,
    pub bytes: usize,
}

pub struct Catalog {
    pub root: PathBuf,
    pub config: Config,
    pub skills: Vec<Skill>,
}

pub fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if !value.is_object() {
        return Err(format!("Expected a JSON object in {}", path.display()));
    }
    Ok(value)
}

pub fn merge(shared: Value, local: &Value) -> Result<Config> {
    let mut config: Config = serde_json::from_value(shared).map_err(|e| e.to_string())?;
    for key in ["sources", "classes", "rules"] {
        if local.get(key).is_some_and(|value| !value.is_array()) {
            return Err(format!("Local {key} must be a list"));
        }
    }
    if let Some(sources) = local.get("sources") {
        config.sources.extend(
            serde_json::from_value::<Vec<Source>>(sources.clone()).map_err(|e| e.to_string())?,
        );
    }
    if let Some(rules) = local.get("rules") {
        config
            .rules
            .extend(serde_json::from_value::<Vec<Rule>>(rules.clone()).map_err(|e| e.to_string())?);
    }
    if let Some(classes) = local.get("classes") {
        for class in
            serde_json::from_value::<Vec<Class>>(classes.clone()).map_err(|e| e.to_string())?
        {
            if let Some(existing) = config.classes.iter_mut().find(|c| c.id == class.id) {
                *existing = class;
            } else {
                config.classes.push(class);
            }
        }
    }
    if let Some(budget) = local.get("default_budget") {
        config.default_budget =
            budget.as_u64().ok_or("Budget must be a positive integer")? as usize;
    }
    if config.version != 1 || config.default_budget == 0 {
        return Err("The catalog needs version 1 and a positive instruction budget".into());
    }
    let mut ids = HashSet::new();
    for class in &config.classes {
        if !ids.insert(&class.id) || class.name.trim().is_empty() || class.trees.is_empty() {
            return Err("Classes need unique IDs, a name, and at least one tree".into());
        }
        let mut trees = HashSet::new();
        for tree in &class.trees {
            if !trees.insert(&tree.id) || tree.name.trim().is_empty() {
                return Err("Trees need unique IDs within their class and a name".into());
            }
        }
    }
    if !config
        .classes
        .iter()
        .any(|c| c.id == "unassigned" && c.trees.iter().any(|t| t.id == "general"))
    {
        return Err("Keep the unassigned class and general tree for unclassified skills".into());
    }
    let mut sources = HashSet::new();
    if config.sources.iter().any(|s| !sources.insert(&s.id)) {
        return Err("Source IDs must be unique".into());
    }
    Ok(config)
}

impl Catalog {
    pub fn open(root: &Path) -> Result<Self> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let local = root.join("skill-trees.local.json");
        let local = if local.exists() {
            read_json(&local)?
        } else {
            serde_json::json!({})
        };
        Self::from_local(&root, &local)
    }

    pub fn from_local(root: &Path, local: &Value) -> Result<Self> {
        let config = merge(read_json(&root.join("skill-trees.json"))?, local)?;
        let skills = scan::scan(root, &config)?;
        let catalog = Self {
            root: root.to_path_buf(),
            config,
            skills,
        };
        for skill in &catalog.skills {
            catalog.expand(std::slice::from_ref(&skill.id))?;
        }
        Ok(catalog)
    }

    pub fn resolve(&self, selector: &str) -> Result<&Skill> {
        if let Some(skill) = self.skills.iter().find(|s| s.id == selector) {
            return Ok(skill);
        }
        let matches: Vec<_> = self
            .skills
            .iter()
            .filter(|s| s.name.eq_ignore_ascii_case(selector))
            .collect();
        match matches.as_slice() {
            [skill] => Ok(skill),
            [] => Err(format!("Unknown skill: {selector}")),
            _ => Err(format!(
                "Several skills are named {selector}; use a source/skill ID"
            )),
        }
    }

    pub fn expand(&self, ids: &[String]) -> Result<Vec<&Skill>> {
        fn visit<'a>(
            catalog: &'a Catalog,
            id: &str,
            path: &mut HashSet<String>,
            out: &mut Vec<&'a Skill>,
        ) -> Result<()> {
            let skill = catalog.resolve(id)?;
            if !path.insert(skill.id.clone()) {
                return Err(format!("Circular prerequisites at {}", skill.id));
            }
            if !out.iter().any(|s| s.id == skill.id) {
                for required in &skill.requires {
                    visit(catalog, required, path, out)?;
                }
                out.push(skill);
            }
            path.remove(&skill.id);
            Ok(())
        }
        let mut out = Vec::new();
        for id in ids {
            visit(self, id, &mut HashSet::new(), &mut out)?;
        }
        Ok(out)
    }

    pub fn estimate(&self, ids: &[String]) -> Result<usize> {
        Ok(80
            + self
                .expand(ids)?
                .iter()
                .map(|s| s.bytes.div_ceil(4) + 80)
                .sum::<usize>())
    }

    pub fn suggest(&self, task: &str, budget: usize) -> Result<Vec<String>> {
        let words: HashSet<_> = task
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .map(str::to_lowercase)
            .collect();
        let mut ranked: Vec<_> = self
            .skills
            .iter()
            .map(|skill| {
                let score: usize = words
                    .iter()
                    .map(|word| {
                        usize::from(skill.name.to_lowercase().contains(word)) * 8
                            + usize::from(
                                skill.tags.iter().any(|t| t.to_lowercase().contains(word)),
                            ) * 5
                            + usize::from(skill.description.to_lowercase().contains(word))
                    })
                    .sum();
                (score, &skill.id)
            })
            .collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
        let cutoff = ranked.first().map_or(4, |r| 4.max(r.0 * 2 / 5));
        let mut selected = Vec::new();
        for (score, id) in ranked {
            if score < cutoff || selected.len() == 3 {
                break;
            }
            if self.expand(&selected)?.iter().any(|s| &s.id == id) {
                continue;
            }
            let mut proposed = selected.clone();
            proposed.push(id.clone());
            if self.estimate(&proposed)? <= budget.min(10_000) {
                selected = proposed;
            }
        }
        Ok(selected)
    }

    pub fn load(&self, ids: &[String], task: &str, budget: usize) -> Result<String> {
        if ids.is_empty() {
            return Err("Select at least one skill first".into());
        }
        let mut prompt = String::from(
            "Apply these selected skills to the task below. Read supporting resources only when needed.\n\n",
        );
        for skill in self.expand(ids)? {
            let source = skill.path.canonicalize().map_err(|e| e.to_string())?;
            let body = fs::read_to_string(&source).map_err(|e| e.to_string())?;
            prompt.push_str(&format!(
                "## {} ({})\nSource: {}\nResource base: {}\n\n{}\n\n",
                skill.name,
                skill.id,
                source.display(),
                source.parent().ok_or("Missing skill folder")?.display(),
                body.trim()
            ));
        }
        prompt.push_str(&format!("Task: {}", task.trim()));
        let estimate = prompt.len().div_ceil(4);
        if estimate > budget.min(10_000) {
            return Err(format!(
                "Selection needs about {estimate} tokens. Budget is {}. Select fewer skills.",
                budget.min(10_000)
            ));
        }
        Ok(prompt)
    }
}
