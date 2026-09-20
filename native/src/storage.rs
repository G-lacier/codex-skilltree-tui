use super::Result;
use super::catalog::Branch;
use super::catalog::Catalog;
use super::catalog::Class;
use super::catalog::read_json;
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::path::Path;

pub fn slug(name: &str) -> Result<String> {
    let slug = name
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        return Err("Use a name with letters or numbers".into());
    }
    Ok(slug)
}

pub fn write_local(root: &Path, path: &Path, text: &str) -> Result<()> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let relative = path
        .strip_prefix(&root)
        .map_err(|_| "Writes must stay inside the skill library")?;
    if relative
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Writes must stay inside the skill library".into());
    }
    let mut existing = path;
    while !existing.exists() {
        existing = existing.parent().ok_or("Missing output parent")?;
    }
    if !existing
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(&root)
    {
        return Err("Linked source files are read only".into());
    }
    fs::create_dir_all(path.parent().ok_or("Missing output parent")?).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

fn local(root: &Path) -> Result<Value> {
    let path = root.join("skill-trees.local.json");
    if path.exists() {
        read_json(&path)
    } else {
        Ok(json!({}))
    }
}

fn list<'a>(local: &'a mut Value, key: &str) -> Result<&'a mut Vec<Value>> {
    local
        .as_object_mut()
        .ok_or("Local settings must be an object")?
        .entry(key)
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| format!("{key} must be a list"))
}

fn save(root: &Path, local: &Value) -> Result<Catalog> {
    let catalog = Catalog::from_local(root, local)?;
    write_local(
        root,
        &root.join("skill-trees.local.json"),
        &(serde_json::to_string_pretty(local).map_err(|e| e.to_string())? + "\n"),
    )?;
    Ok(catalog)
}

fn put_class(local: &mut Value, class: Class) -> Result<()> {
    let classes = list(local, "classes")?;
    classes.retain(|c| c["id"] != class.id);
    classes.push(serde_json::to_value(class).map_err(|e| e.to_string())?);
    Ok(())
}

pub fn save_class(
    catalog: &Catalog,
    id: Option<&str>,
    name: &str,
    description: &str,
) -> Result<Catalog> {
    if name.trim().is_empty() || description.trim().is_empty() {
        return Err("Give the class a name and description".into());
    }
    let mut class = if let Some(id) = id {
        catalog
            .config
            .classes
            .iter()
            .find(|c| c.id == id)
            .ok_or("Unknown class")?
            .clone()
    } else {
        let id = slug(name)?;
        if catalog.config.classes.iter().any(|c| c.id == id) {
            return Err("A class with this name already exists".into());
        }
        Class {
            id,
            name: String::new(),
            description: String::new(),
            trees: vec![Branch {
                id: "general".into(),
                name: "General".into(),
                description: "Skills for this area of work.".into(),
            }],
        }
    };
    class.name = name.trim().into();
    class.description = description.trim().into();
    let mut local = local(&catalog.root)?;
    put_class(&mut local, class)?;
    save(&catalog.root, &local)
}

pub fn save_tree(
    catalog: &Catalog,
    class_id: &str,
    id: Option<&str>,
    name: &str,
    description: &str,
) -> Result<Catalog> {
    if name.trim().is_empty() || description.trim().is_empty() {
        return Err("Give the tree a name and description".into());
    }
    let mut class = catalog
        .config
        .classes
        .iter()
        .find(|c| c.id == class_id)
        .ok_or("Unknown class")?
        .clone();
    let branch = Branch {
        id: id.map(str::to_string).unwrap_or(slug(name)?),
        name: name.trim().into(),
        description: description.trim().into(),
    };
    if let Some(id) = id {
        *class
            .trees
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or("Unknown tree")? = branch;
    } else if class.trees.iter().any(|t| t.id == branch.id) {
        return Err("This class already has a tree with this name".into());
    } else {
        class.trees.push(branch);
    }
    let mut local = local(&catalog.root)?;
    put_class(&mut local, class)?;
    save(&catalog.root, &local)
}

pub fn assign(catalog: &Catalog, ids: &[String], class: &str, tree: &str) -> Result<Catalog> {
    if ids.is_empty() {
        return Err("Select skills with Space, then choose the destination class and tree".into());
    }
    let mut local = local(&catalog.root)?;
    let rules = list(&mut local, "rules")?;
    for id in ids {
        let skill = catalog.resolve(id)?;
        rules.retain(|r| r["id"] != *id);
        rules.push(json!({"id": id, "class": class, "tree": tree, "tags": skill.tags, "requires": skill.requires}));
    }
    save(&catalog.root, &local)
}

pub fn save_metadata(
    catalog: &Catalog,
    id: &str,
    tags: Vec<String>,
    requires: Vec<String>,
) -> Result<Catalog> {
    let skill = catalog.resolve(id)?;
    let mut local = local(&catalog.root)?;
    let rules = list(&mut local, "rules")?;
    rules.retain(|r| r["id"] != id);
    rules.push(json!({"id": id, "class": skill.class, "tree": skill.tree, "tags": tags, "requires": requires}));
    save(&catalog.root, &local)
}

pub fn create_skill(
    catalog: &Catalog,
    name: &str,
    description: &str,
    body: &str,
    class: &str,
    tree: &str,
) -> Result<Catalog> {
    let name = slug(name)?;
    if description.trim().is_empty() || body.trim().is_empty() {
        return Err("Give the skill a description and instructions".into());
    }
    if !catalog
        .config
        .classes
        .iter()
        .any(|c| c.id == class && c.trees.iter().any(|t| t.id == tree))
    {
        return Err("Choose a class and tree".into());
    }
    if catalog
        .config
        .sources
        .iter()
        .any(|s| s.id == "local-created" && s.path != "library/local/created")
    {
        return Err("The local-created source ID is already in use".into());
    }
    let path = catalog
        .root
        .join("library/local/created")
        .join(&name)
        .join("SKILL.md");
    if path.exists() {
        return Err("A local skill with this name already exists".into());
    }
    let description = serde_json::to_string(description.trim()).map_err(|e| e.to_string())?;
    write_local(
        &catalog.root,
        &path,
        &format!(
            "---\nname: {name}\ndescription: {description}\n---\n\n{}\n",
            body.trim()
        ),
    )?;
    let mut local = local(&catalog.root)?;
    if !catalog
        .config
        .sources
        .iter()
        .any(|s| s.id == "local-created")
    {
        list(&mut local, "sources")?
            .push(json!({"id": "local-created", "path": "library/local/created"}));
    }
    list(&mut local, "rules")?
        .push(json!({"id": format!("local-created/{name}"), "class": class, "tree": tree}));
    save(&catalog.root, &local)
}

pub fn editable(catalog: &Catalog, id: &str) -> bool {
    catalog
        .resolve(id)
        .ok()
        .and_then(|s| s.path.canonicalize().ok())
        .is_some_and(|path| path.starts_with(catalog.root.join("library/local/created")))
}

pub fn body(catalog: &Catalog, id: &str) -> Result<String> {
    let text = fs::read_to_string(&catalog.resolve(id)?.path).map_err(|e| e.to_string())?;
    let mut lines = text.lines();
    lines.next();
    for line in lines.by_ref() {
        if line.trim() == "---" {
            break;
        }
    }
    Ok(lines.collect::<Vec<_>>().join("\n").trim().to_string())
}

pub fn save_body(catalog: &Catalog, id: &str, description: &str, body: &str) -> Result<Catalog> {
    if !editable(catalog, id) {
        return Err("Instruction editing is available for skills created in this library".into());
    }
    if description.trim().is_empty() || body.trim().is_empty() {
        return Err("Give the skill a description and instructions".into());
    }
    let skill = catalog.resolve(id)?;
    let description = serde_json::to_string(description.trim()).map_err(|e| e.to_string())?;
    write_local(
        &catalog.root,
        &skill.path,
        &format!(
            "---\nname: {}\ndescription: {description}\n---\n\n{}\n",
            skill.name,
            body.trim()
        ),
    )?;
    Catalog::open(&catalog.root)
}

pub fn add_source(catalog: &Catalog, id: &str, path: &str) -> Result<Catalog> {
    let id = slug(id)?;
    let folder = catalog.root.join(path);
    if !folder.is_dir() {
        return Err("Source must be an existing directory".into());
    }
    if catalog.config.sources.iter().any(|s| s.id == id) {
        return Err("That source ID already exists".into());
    }
    let mut local = local(&catalog.root)?;
    list(&mut local, "sources")?.push(json!({"id": id, "path": path}));
    save(&catalog.root, &local)
}
