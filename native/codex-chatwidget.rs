use super::ChatWidget;
use crate::skilltree_view::SkillTreeView;

impl ChatWidget {
    pub(crate) fn open_skilltree(&mut self, task: String) {
        match SkillTreeView::open(self.config.cwd.as_path(), task, self.app_event_tx.clone()) {
            Ok(view) => self.bottom_pane.show_view(Box::new(view)),
            Err(error) => self.add_error_message(error),
        }
    }
}
