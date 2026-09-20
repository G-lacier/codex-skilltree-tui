//! Codex's own bottom-pane view. No process or model is launched by this editor.
use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::BottomPaneView;
use crate::bottom_pane::CancellationEvent;
use crate::render::renderable::Renderable;
use crate::skilltree::editor::Editor;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::path::Path;
use std::path::PathBuf;

pub(crate) struct SkillTreeView {
    editor: Editor,
    tx: AppEventSender,
}

impl SkillTreeView {
    pub(crate) fn open(cwd: &Path, task: String, tx: AppEventSender) -> Result<Self, String> {
        let root = std::env::var_os("CODEX_SKILLTREE_ROOT").map(PathBuf::from).or_else(|| {
            cwd.ancestors().find(|path| path.join("skill-trees.json").is_file()).map(Path::to_path_buf)
        }).ok_or("Open Codex from a skill-tree repository, or set CODEX_SKILLTREE_ROOT to your library")?;
        Ok(Self {
            editor: Editor::open(&root, task)?,
            tx,
        })
    }
}

impl Renderable for SkillTreeView {
    fn render(&self, area: Rect, buffer: &mut Buffer) {
        self.editor.render(area, buffer);
    }
    fn desired_height(&self, _width: u16) -> u16 {
        26
    }
}

impl BottomPaneView for SkillTreeView {
    fn handle_key_event(&mut self, key: KeyEvent) {
        self.editor.key(key);
        if let Some(prompt) = self.editor.prompt.take() {
            self.tx.send(AppEvent::SkillTreeApply { prompt });
        }
    }
    fn is_complete(&self) -> bool {
        self.editor.closed
    }
    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.editor.closed = true;
        CancellationEvent::Handled
    }
    fn prefer_esc_to_handle_key_event(&self) -> bool {
        true
    }
    fn handle_paste(&mut self, text: String) -> bool {
        self.editor.paste(&text);
        true
    }
}
