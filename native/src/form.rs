use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[derive(Clone)]
pub(crate) enum Target {
    Class(Option<String>),
    Tree(String, Option<String>),
    NewSkill(String, String),
    Metadata(String),
    Instructions(String),
    Source,
    Budget,
    Task,
}

pub(crate) struct Field {
    pub label: &'static str,
    pub value: String,
    pub cursor: usize,
    pub multiline: bool,
}

pub(crate) struct Form {
    pub title: &'static str,
    pub target: Target,
    pub fields: Vec<Field>,
    pub active: usize,
}

impl Form {
    pub fn new(title: &'static str, target: Target, values: Vec<(&'static str, String)>) -> Self {
        Self {
            title,
            target,
            active: 0,
            fields: values
                .into_iter()
                .map(|(label, value)| Field {
                    label,
                    cursor: value.len(),
                    value,
                    multiline: label == "Instructions",
                })
                .collect(),
        }
    }

    pub fn paste(&mut self, text: &str) {
        let field = &mut self.fields[self.active];
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let text = if field.multiline {
            text
        } else {
            text.replace('\n', " ")
        };
        field.value.insert_str(field.cursor, &text);
        field.cursor += text.len();
    }

    pub fn key(&mut self, key: KeyEvent) {
        let field = &mut self.fields[self.active];
        match key.code {
            KeyCode::Tab => self.active = (self.active + 1) % self.fields.len(),
            KeyCode::BackTab => {
                self.active = (self.active + self.fields.len() - 1) % self.fields.len()
            }
            KeyCode::Enter if !field.multiline => {
                self.active = (self.active + 1) % self.fields.len()
            }
            KeyCode::Enter => self.paste("\n"),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                field.value.clear();
                field.cursor = 0;
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.paste(&c.to_string())
            }
            KeyCode::Backspace if field.cursor > 0 => {
                let previous = field.value[..field.cursor]
                    .char_indices()
                    .last()
                    .map_or(0, |(i, _)| i);
                field.value.replace_range(previous..field.cursor, "");
                field.cursor = previous;
            }
            KeyCode::Delete if field.cursor < field.value.len() => {
                let next = field.value[field.cursor..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8);
                field
                    .value
                    .replace_range(field.cursor..field.cursor + next, "");
            }
            KeyCode::Left => {
                field.cursor = field.value[..field.cursor]
                    .char_indices()
                    .last()
                    .map_or(0, |(i, _)| i)
            }
            KeyCode::Right => {
                field.cursor += field.value[field.cursor..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8)
            }
            KeyCode::Home => {
                field.cursor = field.value[..field.cursor].rfind('\n').map_or(0, |i| i + 1)
            }
            KeyCode::End => {
                field.cursor += field.value[field.cursor..]
                    .find('\n')
                    .unwrap_or(field.value.len() - field.cursor)
            }
            KeyCode::Up | KeyCode::Down if field.multiline => {
                let line_start = field.value[..field.cursor].rfind('\n').map_or(0, |i| i + 1);
                let column = field.value[line_start..field.cursor].chars().count();
                let target = if key.code == KeyCode::Up {
                    if line_start == 0 {
                        return;
                    }
                    let end = line_start - 1;
                    let start = field.value[..end].rfind('\n').map_or(0, |i| i + 1);
                    start..end
                } else {
                    let Some(newline) = field.value[field.cursor..].find('\n') else {
                        return;
                    };
                    let start = field.cursor + newline + 1;
                    let end = start
                        + field.value[start..]
                            .find('\n')
                            .unwrap_or(field.value.len() - start);
                    start..end
                };
                field.cursor = target.start
                    + field.value[target.clone()]
                        .char_indices()
                        .nth(column)
                        .map_or(target.len(), |(i, _)| i);
            }
            _ => {}
        }
    }
}
