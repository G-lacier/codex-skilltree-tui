use super::editor::Editor;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

fn list(
    area: Rect,
    buffer: &mut Buffer,
    title: &str,
    items: Vec<String>,
    selected: usize,
    active: bool,
) {
    let block = Block::default().borders(Borders::ALL).title(if active {
        title.to_string().cyan().bold()
    } else {
        title.to_string().dim()
    });
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.is_empty() {
        return;
    }
    let start = selected.saturating_sub(usize::from(inner.height).saturating_sub(1));
    let lines: Vec<Line<'_>> = if items.is_empty() {
        vec!["No skills in this tree".dim().into()]
    } else {
        items
            .into_iter()
            .enumerate()
            .skip(start)
            .take(usize::from(inner.height))
            .map(|(i, text)| {
                if i == selected && active {
                    text.cyan().reversed().into()
                } else if i == selected {
                    text.bold().into()
                } else {
                    text.into()
                }
            })
            .collect()
    };
    Paragraph::new(lines).render(inner, buffer);
}

fn wrapped(text: &str, width: u16) -> Vec<Line<'static>> {
    text.lines()
        .flat_map(|line| {
            textwrap::wrap(line, usize::from(width.max(1)))
                .into_iter()
                .map(|s| Line::from(s.into_owned()))
                .collect::<Vec<_>>()
        })
        .collect()
}

impl Editor {
    pub fn render(&self, area: Rect, buffer: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::TOP)
            .title(" Skill trees ".cyan().bold());
        let inner = block.inner(area);
        block.render(area, buffer);
        if inner.width < 12 || inner.height < 3 {
            return;
        }
        if let Some(form) = &self.form {
            Paragraph::new(form.title.bold())
                .render(Rect::new(inner.x, inner.y, inner.width, 1), buffer);
            let mut y = inner.y + 2;
            for (i, field) in form.fields.iter().enumerate() {
                if y + 3 > inner.bottom() {
                    break;
                }
                let active = form.active == i;
                let label = if active {
                    field.label.cyan().bold()
                } else {
                    field.label.dim()
                };
                Paragraph::new(Line::from(label))
                    .render(Rect::new(inner.x, y, inner.width, 1), buffer);
                y += 1;
                let mut value = field.value.clone();
                if active {
                    value.insert(field.cursor, '│');
                }
                let height = if field.multiline {
                    inner.bottom().saturating_sub(y + 3)
                } else {
                    2
                };
                let lines = wrapped(&value, inner.width);
                let before_cursor = wrapped(&field.value[..field.cursor], inner.width).len();
                let start = if active {
                    before_cursor.saturating_sub(usize::from(height).max(1))
                } else {
                    0
                };
                Paragraph::new(lines.into_iter().skip(start).collect::<Vec<_>>())
                    .render(Rect::new(inner.x, y, inner.width, height), buffer);
                y += height + 1;
            }
            Paragraph::new(wrapped(&self.status, inner.width)).render(
                Rect::new(inner.x, inner.bottom().saturating_sub(3), inner.width, 2),
                buffer,
            );
            Paragraph::new("Tab fields · Ctrl+S save · Ctrl+U clear field · Esc cancel".dim())
                .render(
                    Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
                    buffer,
                );
            return;
        }
        let count = self
            .catalog
            .expand(&self.selected)
            .map_or(0, |skills| skills.len());
        let estimate = if self.selected.is_empty() {
            0
        } else {
            self.catalog.estimate(&self.selected).unwrap_or(0)
        };
        let task = if self.task.is_empty() {
            "Choose skills, then write your task in the composer"
        } else {
            &self.task
        };
        let header = vec![
            Line::from(task.to_string()),
            Line::from(vec![
                format!(
                    "{} selected · {count} with prerequisites · ~{estimate}/{} tokens",
                    self.selected.len(),
                    self.budget
                )
                .cyan(),
                "   t task · r suggest · b budget".dim(),
            ]),
        ];
        Paragraph::new(header).render(Rect::new(inner.x, inner.y, inner.width, 2), buffer);
        let footer_rows = if inner.height >= 17 { 8 } else { 5 };
        let list_area = Rect::new(
            inner.x,
            inner.y + 2,
            inner.width,
            inner.height.saturating_sub(2 + footer_rows),
        );
        let class = &self.catalog.config.classes[self.class];
        let classes = self
            .catalog
            .config
            .classes
            .iter()
            .map(|c| c.name.clone())
            .collect();
        let trees = class.trees.iter().map(|t| t.name.clone()).collect();
        let visible = self.visible();
        let skills = visible
            .iter()
            .map(|i| {
                let skill = &self.catalog.skills[*i];
                format!(
                    "[{}] {}",
                    if self.selected.contains(&skill.id) {
                        '+'
                    } else {
                        ' '
                    },
                    skill.name
                )
            })
            .collect();
        let titles = ["Classes", "Trees", "Skills"];
        let positions = [self.class, self.tree, self.skill];
        if inner.width >= 76 {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(30),
                    Constraint::Percentage(40),
                ])
                .split(list_area);
            for (i, items) in [classes, trees, skills].into_iter().enumerate() {
                list(
                    columns[i],
                    buffer,
                    titles[i],
                    items,
                    positions[i],
                    self.focus == i,
                );
            }
        } else {
            for (i, items) in [classes, trees, skills].into_iter().enumerate() {
                if self.focus == i {
                    list(list_area, buffer, titles[i], items, positions[i], true);
                }
            }
        }
        let detail_y = list_area.bottom();
        let search = if self.searching {
            format!("Search all skills: {}│  Enter to browse", self.query)
        } else if !self.query.is_empty() {
            format!("Search: {} · / to change · Esc then clears", self.query)
        } else {
            format!("{} / {}", class.name, class.trees[self.tree].name)
        };
        let mut details = vec![Line::from(search.cyan())];
        if let Some(index) = visible.get(self.skill) {
            let skill = &self.catalog.skills[*index];
            details.extend(
                wrapped(
                    &format!("{} · {}", skill.id, skill.description),
                    inner.width,
                )
                .into_iter()
                .take(2),
            );
            if footer_rows >= 8 {
                details.push(Line::from(
                    format!(
                        "Tags: {} | Requires: {}",
                        skill.tags.join(", "),
                        skill.requires.join(", ")
                    )
                    .dim(),
                ));
            }
        }
        if footer_rows >= 8 {
            details.extend(wrapped(&self.status, inner.width).into_iter().take(2));
        }
        Paragraph::new(details).render(
            Rect::new(
                inner.x,
                detail_y,
                inner.width,
                footer_rows.saturating_sub(2),
            ),
            buffer,
        );
        let hints = if inner.width >= 84 {
            vec![Line::from("Tab/←→ pane · ↑↓ browse · Space select · Ctrl+L load into chat · / search".dim()),
                Line::from("n new · e edit · i instructions · a assign · x clear · s source · F5 refresh · Esc close".dim())]
        } else {
            vec![
                Line::from("Tab pane · ↑↓ browse · Space select · Ctrl+L load".dim()),
                Line::from("n new · e edit · i body · a assign · / search · Esc close".dim()),
            ]
        };
        Paragraph::new(hints).render(
            Rect::new(inner.x, inner.bottom().saturating_sub(2), inner.width, 2),
            buffer,
        );
    }
}
