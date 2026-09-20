use super::Result;
use super::catalog::Catalog;
use super::form::Form;
use super::form::Target;
use super::storage;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use std::path::Path;

pub struct Editor {
    pub catalog: Catalog,
    pub selected: Vec<String>,
    pub task: String,
    pub budget: usize,
    pub closed: bool,
    pub prompt: Option<String>,
    pub(crate) focus: usize,
    pub(crate) class: usize,
    pub(crate) tree: usize,
    pub(crate) skill: usize,
    pub(crate) query: String,
    pub(crate) searching: bool,
    pub(crate) form: Option<Form>,
    pub(crate) status: String,
}

impl Editor {
    pub fn open(root: &Path, task: String) -> Result<Self> {
        let catalog = Catalog::open(root)?;
        let budget = catalog.config.default_budget.min(10_000);
        let mut editor = Self {
            catalog,
            budget,
            task,
            selected: Vec::new(),
            closed: false,
            prompt: None,
            focus: 0,
            class: 0,
            tree: 0,
            skill: 0,
            query: String::new(),
            searching: false,
            form: None,
            status: "Changes are saved in your local library.".into(),
        };
        if !editor.task.trim().is_empty() {
            editor.suggest()?;
        }
        Ok(editor)
    }

    pub(crate) fn visible(&self) -> Vec<usize> {
        let class = &self.catalog.config.classes[self.class];
        let tree = &class.trees[self.tree];
        self.catalog
            .skills
            .iter()
            .enumerate()
            .filter_map(|(index, skill)| {
                let matches = if self.query.is_empty() {
                    skill.class == class.id && skill.tree == tree.id
                } else {
                    let haystack = format!(
                        "{} {} {} {}",
                        skill.id,
                        skill.name,
                        skill.description,
                        skill.tags.join(" ")
                    )
                    .to_lowercase();
                    self.query
                        .to_lowercase()
                        .split_whitespace()
                        .all(|word| haystack.contains(word))
                };
                matches.then_some(index)
            })
            .collect()
    }

    fn focused_id(&self) -> Result<String> {
        self.visible()
            .get(self.skill)
            .map(|i| self.catalog.skills[*i].id.clone())
            .ok_or_else(|| "Choose a skill first".into())
    }

    fn suggest(&mut self) -> Result<()> {
        self.selected = self.catalog.suggest(&self.task, self.budget)?;
        if let Some(id) = self.selected.first() {
            let skill = self.catalog.resolve(id)?;
            self.class = self
                .catalog
                .config
                .classes
                .iter()
                .position(|c| c.id == skill.class)
                .ok_or("Unknown class")?;
            self.tree = self.catalog.config.classes[self.class]
                .trees
                .iter()
                .position(|t| t.id == skill.tree)
                .ok_or("Unknown tree")?;
            self.focus = 2;
            self.query.clear();
            self.skill = self
                .visible()
                .iter()
                .position(|i| self.catalog.skills[*i].id == *id)
                .unwrap_or(0);
        }
        self.status = format!(
            "{} suggested skills selected. Space changes the selection; Ctrl+L loads it.",
            self.selected.len()
        );
        Ok(())
    }

    fn start_form(&mut self, edit: bool) -> Result<()> {
        let class = &self.catalog.config.classes[self.class];
        let tree = &class.trees[self.tree];
        self.form = Some(match (self.focus, edit) {
            (0, _) => Form::new(
                if edit { "Edit class" } else { "New class" },
                Target::Class(edit.then(|| class.id.clone())),
                vec![
                    (
                        "Name",
                        if edit {
                            class.name.clone()
                        } else {
                            String::new()
                        },
                    ),
                    (
                        "Description",
                        if edit {
                            class.description.clone()
                        } else {
                            String::new()
                        },
                    ),
                ],
            ),
            (1, _) => Form::new(
                if edit { "Edit tree" } else { "New tree" },
                Target::Tree(class.id.clone(), edit.then(|| tree.id.clone())),
                vec![
                    (
                        "Name",
                        if edit {
                            tree.name.clone()
                        } else {
                            String::new()
                        },
                    ),
                    (
                        "Description",
                        if edit {
                            tree.description.clone()
                        } else {
                            String::new()
                        },
                    ),
                ],
            ),
            (_, false) => Form::new(
                "New local skill",
                Target::NewSkill(class.id.clone(), tree.id.clone()),
                vec![
                    ("Name", String::new()),
                    ("Description", String::new()),
                    ("Instructions", String::new()),
                ],
            ),
            (_, true) => {
                let id = self.focused_id()?;
                let skill = self.catalog.resolve(&id)?;
                Form::new(
                    "Edit skill classification",
                    Target::Metadata(id),
                    vec![
                        ("Tags (comma separated)", skill.tags.join(", ")),
                        (
                            "Prerequisites (source/skill IDs, comma separated)",
                            skill.requires.join(", "),
                        ),
                    ],
                )
            }
        });
        self.status.clear();
        Ok(())
    }

    fn save_form(&mut self) -> Result<()> {
        let form = self.form.as_ref().ok_or("No form open")?;
        let value = |i: usize| form.fields[i].value.as_str();
        let split = |i| {
            value(i)
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        };
        let updated = match &form.target {
            Target::Class(id) => Some(storage::save_class(
                &self.catalog,
                id.as_deref(),
                value(0),
                value(1),
            )?),
            Target::Tree(class, id) => Some(storage::save_tree(
                &self.catalog,
                class,
                id.as_deref(),
                value(0),
                value(1),
            )?),
            Target::NewSkill(class, tree) => Some(storage::create_skill(
                &self.catalog,
                value(0),
                value(1),
                value(2),
                class,
                tree,
            )?),
            Target::Metadata(id) => Some(storage::save_metadata(
                &self.catalog,
                id,
                split(0),
                split(1),
            )?),
            Target::Instructions(id) => {
                Some(storage::save_body(&self.catalog, id, value(0), value(1))?)
            }
            Target::Source => Some(storage::add_source(&self.catalog, value(0), value(1))?),
            Target::Budget => {
                let budget: usize = value(0)
                    .trim()
                    .parse()
                    .map_err(|_| "Use a budget from 1 to 10000")?;
                if !(1..=10_000).contains(&budget) {
                    return Err("Use a budget from 1 to 10000".into());
                }
                self.budget = budget;
                None
            }
            Target::Task => {
                self.task = value(0).trim().into();
                None
            }
        };
        if let Some(catalog) = updated {
            self.catalog = catalog;
        }
        self.form = None;
        self.status = "Saved.".into();
        self.skill = self.skill.min(self.visible().len().saturating_sub(1));
        Ok(())
    }

    fn action(&mut self, key: KeyEvent) -> Result<()> {
        if self.form.is_some() {
            if key.code == KeyCode::Esc {
                self.form = None;
                self.status.clear();
            } else if key.code == KeyCode::Char('s')
                && key.modifiers.contains(KeyModifiers::CONTROL)
            {
                self.save_form()?;
            } else if let Some(form) = &mut self.form {
                form.key(key);
            }
            return Ok(());
        }
        if self.searching {
            match key.code {
                KeyCode::Esc => {
                    self.query.clear();
                    self.searching = false;
                }
                KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.query.pop();
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.query.push(c)
                }
                _ => {}
            }
            self.skill = 0;
            return Ok(());
        }
        match key.code {
            KeyCode::Esc => self.closed = true,
            KeyCode::Tab => self.focus = (self.focus + 1) % 3,
            KeyCode::BackTab => self.focus = (self.focus + 2) % 3,
            KeyCode::Left => self.focus = self.focus.saturating_sub(1),
            KeyCode::Right => self.focus = (self.focus + 1).min(2),
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End => {
                let len = match self.focus {
                    0 => self.catalog.config.classes.len(),
                    1 => self.catalog.config.classes[self.class].trees.len(),
                    _ => self.visible().len(),
                };
                let position = match self.focus {
                    0 => &mut self.class,
                    1 => &mut self.tree,
                    _ => &mut self.skill,
                };
                *position = match key.code {
                    KeyCode::Up => position.saturating_sub(1),
                    KeyCode::Down => (*position + 1).min(len.saturating_sub(1)),
                    KeyCode::PageUp => position.saturating_sub(8),
                    KeyCode::PageDown => (*position + 8).min(len.saturating_sub(1)),
                    KeyCode::Home => 0,
                    _ => len.saturating_sub(1),
                };
                if self.focus == 0 {
                    self.tree = 0;
                    self.skill = 0;
                    self.query.clear();
                }
                if self.focus == 1 {
                    self.skill = 0;
                    self.query.clear();
                }
            }
            KeyCode::Enter if self.focus < 2 => self.focus += 1,
            KeyCode::Enter | KeyCode::Char(' ') if self.focus == 2 => {
                let id = self.focused_id()?;
                if self.selected.contains(&id) {
                    self.selected.retain(|s| s != &id);
                } else {
                    self.selected.push(id);
                }
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.prompt = Some(self.catalog.load(&self.selected, &self.task, self.budget)?);
                self.closed = true;
            }
            KeyCode::Char('n') => self.start_form(false)?,
            KeyCode::Char('e') => self.start_form(true)?,
            KeyCode::Char('i') => {
                let id = self.focused_id()?;
                if !storage::editable(&self.catalog, &id) {
                    return Err("Linked skills are read only. Use e to edit their local tags and prerequisites.".into());
                }
                self.form = Some(Form::new(
                    "Edit local instructions",
                    Target::Instructions(id.clone()),
                    vec![
                        (
                            "Description",
                            self.catalog.resolve(&id)?.description.clone(),
                        ),
                        ("Instructions", storage::body(&self.catalog, &id)?),
                    ],
                ));
            }
            KeyCode::Char('a') => {
                let class = &self.catalog.config.classes[self.class];
                self.catalog = storage::assign(
                    &self.catalog,
                    &self.selected,
                    &class.id,
                    &class.trees[self.tree].id,
                )?;
                self.skill = 0;
                self.status = "Selected skills assigned to this tree.".into();
            }
            KeyCode::Char('x') => self.selected.clear(),
            KeyCode::Char('/') => {
                self.searching = true;
                self.focus = 2;
                self.skill = 0;
            }
            KeyCode::Char('r') => self.suggest()?,
            KeyCode::Char('b') => {
                self.form = Some(Form::new(
                    "Instruction budget (approximate tokens)",
                    Target::Budget,
                    vec![("Budget", self.budget.to_string())],
                ))
            }
            KeyCode::Char('t') => {
                self.form = Some(Form::new(
                    "Task for Codex",
                    Target::Task,
                    vec![("Task", self.task.clone())],
                ))
            }
            KeyCode::Char('s') => {
                self.form = Some(Form::new(
                    "Add local skill source",
                    Target::Source,
                    vec![
                        ("Source ID", String::new()),
                        ("Folder relative to this library", "library/local/".into()),
                    ],
                ))
            }
            KeyCode::F(5) => {
                self.catalog = Catalog::open(&self.catalog.root)?;
                self.class = 0;
                self.tree = 0;
                self.skill = 0;
                self.selected.retain(|id| self.catalog.resolve(id).is_ok());
                self.status = "Library refreshed.".into();
            }
            _ => {}
        }
        Ok(())
    }

    pub fn key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.closed = true;
            return;
        }
        if let Err(error) = self.action(key) {
            self.status = error;
        }
    }

    pub fn paste(&mut self, text: &str) {
        if let Some(form) = &mut self.form {
            form.paste(text);
        } else if self.searching {
            self.query.push_str(&text.replace(['\n', '\r'], " "));
            self.skill = 0;
        }
    }
}
