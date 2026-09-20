use super::*;
use std::str::FromStr;

#[tokio::test]
async fn skilltree_command_opens_native_view_and_loads_selection_into_same_chat() {
    let library = crate::skilltree::tests::Library::new();
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.config.cwd = library.root.clone().abs();
    let command = SlashCommand::from_str("skilltree").unwrap();
    chat.dispatch_command_with_args(command, "debug a bug".to_string(), Vec::new());
    assert!(chat.bottom_pane.has_active_view());
    chat.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL));
    let prompt = loop {
        match rx.try_recv().expect("native editor must emit a selection") {
            AppEvent::SkillTreeApply { prompt } => break prompt,
            _ => continue,
        }
    };
    assert!(prompt.contains("trace-bug"));
    assert!(prompt.contains("project-map"));
    assert!(prompt.contains("Task: debug a bug"));
    assert!(
        op_rx.try_recv().is_err(),
        "Browsing must not start a model turn"
    );
    chat.insert_str(&prompt);
    assert_eq!(chat.bottom_pane.composer_text(), prompt);
}

#[tokio::test]
async fn skilltree_escape_closes_editor_without_submitting_a_turn() {
    let library = crate::skilltree::tests::Library::new();
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.config.cwd = library.root.clone().abs();
    chat.dispatch_command(SlashCommand::Skilltree);
    assert!(chat.bottom_pane.has_active_view());
    chat.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!chat.bottom_pane.has_active_view());
    assert!(chat.bottom_pane.composer_text().is_empty());
    assert!(op_rx.try_recv().is_err());
    while let Ok(event) = rx.try_recv() {
        assert!(!matches!(event, AppEvent::SkillTreeApply { .. }));
    }
}
