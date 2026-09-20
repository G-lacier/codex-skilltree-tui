use super::catalog::Catalog;
use super::editor::Editor;
use super::storage;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub(crate) struct Library {
    pub(crate) root: PathBuf,
    _directory: TempDir,
}

impl Library {
    pub(crate) fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let config = json!({
            "version":1,"default_budget":6000,
            "sources":[{"id":"bundled","path":"library/bundled"}],
            "classes":[
                {"id":"development","name":"Software Development","description":"Build software.","trees":[{"id":"engineering","name":"Codebase analysis","description":"Understand project code."}]},
                {"id":"debugging","name":"Debugging and Inspection","description":"Investigate failures.","trees":[{"id":"debugging","name":"Bug diagnosis","description":"Find the cause of a failure."}]},
                {"id":"unassigned","name":"Unassigned","description":"Unclassified skills.","trees":[{"id":"general","name":"General","description":"Unclassified skills."}]}
            ],
            "rules":[
                {"id":"bundled/project-map","class":"development","tree":"engineering","tags":["architecture"]},
                {"id":"bundled/trace-bug","class":"debugging","tree":"debugging","tags":["debug","bug"],"requires":["bundled/project-map"]}
            ]
        });
        fs::write(
            root.join("skill-trees.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
        for (name, description, body) in [
            (
                "project-map",
                "Map a codebase before making changes.",
                "Inspect the project entry points.",
            ),
            (
                "trace-bug",
                "Investigate bugs and failures.",
                "Trace the failure from its observable result.",
            ),
        ] {
            let folder = root.join("library/bundled").join(name);
            fs::create_dir_all(&folder).unwrap();
            fs::write(
                folder.join("SKILL.md"),
                format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}\n"),
            )
            .unwrap();
        }
        Self {
            root: root.canonicalize().unwrap(),
            _directory: directory,
        }
    }
}

fn key(editor: &mut Editor, code: KeyCode) {
    editor.key(KeyEvent::new(code, KeyModifiers::NONE));
}
fn control(editor: &mut Editor, c: char) {
    editor.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}
fn screen(editor: &Editor, width: u16) -> String {
    let rect = Rect::new(0, 0, width, 26);
    let mut buffer = Buffer::empty(rect);
    editor.render(rect, &mut buffer);
    buffer
        .content
        .chunks(usize::from(width))
        .map(|row| {
            row.iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn routing_uses_only_metadata_and_loading_reads_only_selected_bodies() {
    let lib = Library::new();
    let unused = lib.root.join("library/bundled/unused");
    fs::create_dir_all(&unused).unwrap();
    fs::write(
        unused.join("SKILL.md"),
        b"---\nname: unrelated\ndescription: unrelated\n---\n\xff",
    )
    .unwrap();
    let catalog = Catalog::open(&lib.root).unwrap();
    let selected = catalog.suggest("debug a bug", 6000).unwrap();
    assert_eq!(selected, vec!["bundled/trace-bug"]);
    let loaded = catalog.load(&selected, "debug a bug", 6000).unwrap();
    assert!(
        loaded.find("Inspect the project").unwrap() < loaded.find("Trace the failure").unwrap()
    );
    assert!(!loaded.contains("unrelated"));
    assert!(catalog.load(&["bundled/unused".into()], "", 6000).is_err());
}

#[test]
fn dependency_order_deduplication_and_budget_apply_to_whole_selection() {
    let lib = Library::new();
    let catalog = Catalog::open(&lib.root).unwrap();
    let ids = vec!["bundled/trace-bug".into(), "bundled/project-map".into()];
    assert_eq!(
        catalog
            .expand(&ids)
            .unwrap()
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        vec!["bundled/project-map", "bundled/trace-bug"]
    );
    assert!(catalog.load(&ids, "debug", 2).is_err());
    assert!(catalog.suggest("debug a bug", 2).unwrap().is_empty());
}

#[test]
fn local_changes_preserve_shared_definitions_and_persist_assignments() {
    let lib = Library::new();
    let shared = fs::read(lib.root.join("skill-trees.json")).unwrap();
    let catalog = Catalog::open(&lib.root).unwrap();
    let catalog = storage::save_class(
        &catalog,
        Some("development"),
        "Web Development",
        "Build websites.",
    )
    .unwrap();
    let catalog = storage::save_tree(
        &catalog,
        "development",
        None,
        "Accessibility",
        "Check keyboard access.",
    )
    .unwrap();
    let catalog = storage::assign(
        &catalog,
        &["bundled/project-map".into()],
        "development",
        "accessibility",
    )
    .unwrap();
    assert_eq!(
        catalog.resolve("project-map").unwrap().tree,
        "accessibility"
    );
    assert_eq!(fs::read(lib.root.join("skill-trees.json")).unwrap(), shared);
    assert_eq!(
        Catalog::open(&lib.root)
            .unwrap()
            .resolve("project-map")
            .unwrap()
            .tree,
        "accessibility"
    );
    assert_eq!(
        Catalog::open(&lib.root).unwrap().config.classes[0].name,
        "Web Development"
    );
}

#[test]
fn invalid_prerequisite_edits_leave_saved_settings_unchanged() {
    let lib = Library::new();
    let catalog = Catalog::open(&lib.root).unwrap();
    let catalog = storage::save_metadata(
        &catalog,
        "bundled/project-map",
        vec!["architecture".into()],
        Vec::new(),
    )
    .unwrap();
    let before = fs::read(lib.root.join("skill-trees.local.json")).unwrap();
    assert!(
        storage::save_metadata(
            &catalog,
            "bundled/project-map",
            Vec::new(),
            vec!["bundled/trace-bug".into()]
        )
        .is_err()
    );
    assert!(
        storage::save_metadata(
            &catalog,
            "bundled/project-map",
            Vec::new(),
            vec!["missing".into()]
        )
        .is_err()
    );
    assert_eq!(
        fs::read(lib.root.join("skill-trees.local.json")).unwrap(),
        before
    );
}

#[test]
fn local_skill_create_edit_and_multiline_headers_round_trip() {
    let lib = Library::new();
    let catalog = Catalog::open(&lib.root).unwrap();
    let catalog = storage::create_skill(
        &catalog,
        "Review API",
        "Review API behavior.",
        "Check the API.\nCompare responses.",
        "development",
        "engineering",
    )
    .unwrap();
    let catalog = storage::save_body(
        &catalog,
        "local-created/review-api",
        "Updated description.",
        "Read the API contract.",
    )
    .unwrap();
    assert_eq!(
        storage::body(&catalog, "local-created/review-api").unwrap(),
        "Read the API contract."
    );
    assert_eq!(
        catalog.resolve("review-api").unwrap().description,
        "Updated description."
    );
    assert!(storage::save_body(&catalog, "bundled/project-map", "test", "test").is_err());
    fs::write(lib.root.join("library/bundled/project-map/SKILL.md"), "---\nname: 'project-map'\ndescription: >-\n  Read the code\n  before editing it.\n---\nBody").unwrap();
    assert_eq!(
        Catalog::open(&lib.root)
            .unwrap()
            .resolve("project-map")
            .unwrap()
            .description,
        "Read the code before editing it."
    );
}

#[test]
fn keyboard_class_creation_selection_and_composer_handoff() {
    let lib = Library::new();
    let mut editor = Editor::open(&lib.root, String::new()).unwrap();
    key(&mut editor, KeyCode::Char('n'));
    editor.paste("Documentation");
    key(&mut editor, KeyCode::Tab);
    editor.paste("Write product documentation.");
    control(&mut editor, 's');
    assert!(editor.form.is_none());
    assert!(
        Catalog::open(&lib.root)
            .unwrap()
            .config
            .classes
            .iter()
            .any(|c| c.name == "Documentation")
    );
    key(&mut editor, KeyCode::Char('/'));
    editor.paste("trace-bug");
    key(&mut editor, KeyCode::Enter);
    key(&mut editor, KeyCode::Char(' '));
    control(&mut editor, 'l');
    assert!(editor.closed);
    let prompt = editor.prompt.unwrap();
    assert!(prompt.contains("Trace the failure"));
    assert!(prompt.contains("Inspect the project"));
}

#[test]
fn keyboard_assignment_moves_selected_skills_to_target_tree() {
    let lib = Library::new();
    let mut editor = Editor::open(&lib.root, String::new()).unwrap();
    key(&mut editor, KeyCode::Right);
    key(&mut editor, KeyCode::Right);
    key(&mut editor, KeyCode::Char(' '));
    key(&mut editor, KeyCode::Left);
    key(&mut editor, KeyCode::Left);
    key(&mut editor, KeyCode::Down);
    key(&mut editor, KeyCode::Char('a'));
    assert_eq!(
        Catalog::open(&lib.root)
            .unwrap()
            .resolve("project-map")
            .unwrap()
            .class,
        "debugging"
    );
}

#[test]
fn unicode_form_edits_and_cancellation_preserve_data() {
    let lib = Library::new();
    let mut editor = Editor::open(&lib.root, String::new()).unwrap();
    key(&mut editor, KeyCode::Char('n'));
    editor.paste("Résumé");
    key(&mut editor, KeyCode::Left);
    key(&mut editor, KeyCode::Backspace);
    assert_eq!(editor.form.as_ref().unwrap().fields[0].value, "Résué");
    key(&mut editor, KeyCode::Esc);
    assert!(!lib.root.join("skill-trees.local.json").exists());
    assert!(!editor.closed);
    key(&mut editor, KeyCode::Esc);
    assert!(editor.closed);
    assert!(editor.prompt.is_none());
}

#[test]
fn native_library_and_forms_render_at_wide_and_narrow_terminal_sizes() {
    let lib = Library::new();
    let mut editor = Editor::open(&lib.root, "debug a bug".into()).unwrap();
    insta::with_settings!({prepend_module_to_snapshot => false}, {
        insta::assert_snapshot!("skilltree_wide", screen(&editor, 104));
        insta::assert_snapshot!("skilltree_narrow", screen(&editor, 58));
        key(&mut editor, KeyCode::Char('n'));
        insta::assert_snapshot!("skilltree_new_skill", screen(&editor, 104));
    });
}

#[cfg(unix)]
#[test]
fn linked_sources_are_scanned_once_and_external_writes_are_rejected() {
    use std::os::unix::fs::symlink;
    let lib = Library::new();
    let external = Library::new();
    symlink(
        lib.root.join("library/bundled"),
        lib.root.join("library/bundled/loop"),
    )
    .unwrap();
    assert_eq!(Catalog::open(&lib.root).unwrap().skills.len(), 2);
    let catalog = storage::add_source(
        &Catalog::open(&lib.root).unwrap(),
        "external",
        external.root.join("library/bundled").to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(catalog.skills.len(), 4);
    assert!(catalog.resolve("external/project-map").is_ok());
    symlink(&external.root, lib.root.join("outside")).unwrap();
    assert!(storage::write_local(&lib.root, &lib.root.join("outside/test.md"), "changed").is_err());
    assert!(!external.root.join("test.md").exists());
}
