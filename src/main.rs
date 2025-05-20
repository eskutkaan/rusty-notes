use eframe::egui::{self, CentralPanel, Context, Layout, SidePanel, TextEdit, TopBottomPanel, Visuals};
use std::fs;
use std::path::{Path, PathBuf};

struct Note {
    title: String,
    content: String,
    path: PathBuf,
}

struct AppState {
    notes: Vec<Note>,
    open_tabs: Vec<usize>,
    current_tab: Option<usize>,
    search_query: String,
    notes_dir: PathBuf,
    editing_title: Option<usize>,
    editing_title_buffer: String,
    dark_mode: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let notes_dir = std::env::current_dir().unwrap().join("notes");
        let _ = fs::create_dir_all(&notes_dir);
        let mut notes = vec![];

        if let Ok(entries) = fs::read_dir(&notes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "md") {
                    let content = fs::read_to_string(&path).unwrap_or_default();
                    let title = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                    notes.push(Note { title, content, path });
                }
            }
        }
        notes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

        Self {
            notes,
            open_tabs: vec![],
            current_tab: None,
            search_query: String::new(),
            notes_dir,
            editing_title: None,
            editing_title_buffer: String::new(),
            dark_mode: true,
        }
    }
}

impl AppState {
    fn create_note(&mut self) {
        let title = format!("Note_{}.md", self.notes.len() + 1);
        let safe_title = title.replace(|c: char| !c.is_alphanumeric(), "_");
        let path = self.notes_dir.join(&safe_title);
        if fs::write(&path, "").is_ok() {
            let note = Note {
                title: safe_title.trim_end_matches(".md").to_string(),
                content: String::new(),
                path,
            };
            self.notes.push(note);
            self.notes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        }
    }

    fn delete_note(&mut self, i: usize) {
        let _ = fs::remove_file(&self.notes[i].path);
        self.notes.remove(i);
        self.open_tabs.retain(|&x| x != i);
        for tab in self.open_tabs.iter_mut() {
            if *tab > i {
                *tab -= 1;
            }
        }
        if let Some(current) = self.current_tab {
            if current == i {
                self.current_tab = self.open_tabs.last().copied();
            } else if current > i {
                self.current_tab = Some(current - 1);
            }
        }
        if self.notes.is_empty() {
            self.current_tab = None;
        }
    }

    fn rename_note(&mut self, idx: usize, new_title: &str) {
        if let Some(note) = self.notes.get_mut(idx) {
            let safe_title = new_title.replace(|c: char| !c.is_alphanumeric(), "_");
            let new_path = self.notes_dir.join(format!("{}.md", safe_title));
            if fs::rename(&note.path, &new_path).is_ok() {
                note.title = safe_title;
                note.path = new_path;
                self.notes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            }
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Apply theme
        ctx.set_visuals(if self.dark_mode {
            Visuals::dark()
        } else {
            Visuals::light()
        });

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("+ New Note").clicked() {
                    self.create_note();
                }
                ui.text_edit_singleline(&mut self.search_query);
                if ui.button(if self.dark_mode { "🌞 Light" } else { "🌙 Dark" }).clicked() {
                    self.dark_mode = !self.dark_mode;
                }
            });
        });

        SidePanel::left("side_panel")
            .resizable(true)
            .default_width(180.0)
            .min_width(120.0)
            .show(ctx, |ui| {
                let mut note_to_remove: Option<usize> = None;
                for (i, note) in self.notes.iter().enumerate() {
                    let query = self.search_query.to_lowercase();
                    if !query.is_empty()
                        && !(note.title.to_lowercase().contains(&query)
                            || note.content.to_lowercase().contains(&query))
                    {
                        continue;
                    }

                    ui.horizontal(|ui| {
                        if ui.button(&note.title).on_hover_text("Open note").clicked() {
                            if !self.open_tabs.contains(&i) {
                                self.open_tabs.push(i);
                            }
                            self.current_tab = Some(i);
                        }
                        if ui.button("🗑").on_hover_text("Delete note").clicked() {
                            note_to_remove = Some(i);
                        }
                    });
                }

                if let Some(i) = note_to_remove {
                    self.delete_note(i);
                }
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(eframe::egui::Align::Min), |ui| {
                ui.horizontal_wrapped(|ui| {
                    let mut tab_to_close: Option<usize> = None;
                    for &tab_idx in &self.open_tabs {
                        let note = &self.notes[tab_idx];
                        let selected = self.current_tab == Some(tab_idx);

                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, &note.title).clicked() {
                                self.current_tab = Some(tab_idx);
                            }
                            if ui.button("❌").on_hover_text("Close tab").clicked() {
                                tab_to_close = Some(tab_idx);
                            }
                        });
                    }

                    if let Some(idx) = tab_to_close {
                        self.open_tabs.retain(|&x| x != idx);
                        if self.current_tab == Some(idx) {
                            self.current_tab = self.open_tabs.last().copied();
                        }
                    }
                });

                ui.separator();

                if let Some(idx) = self.current_tab {
                    // Clone title outside borrow
                    let title = self.notes[idx].title.clone();

                    if self.editing_title == Some(idx) {
                        let mut new_title = self.editing_title_buffer.clone();
                        ui.horizontal(|ui| {
                            let title_edit = ui.text_edit_singleline(&mut new_title);

                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let ok_clicked = ui.button("OK").clicked();

                            if title_edit.lost_focus() || enter_pressed || ok_clicked {
                                self.rename_note(idx, &new_title);
                                self.editing_title = None;
                            } else {
                                self.editing_title_buffer = new_title;
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.heading(&title);
                            if ui.button("✏️ Rename").clicked() {
                                self.editing_title = Some(idx);
                                self.editing_title_buffer = title;
                            }
                        });
                    }

                    // Mutable borrow after rename UI
                    let note = &mut self.notes[idx];
                    let response = ui.add_sized(ui.available_size(), TextEdit::multiline(&mut note.content));
                    if response.changed() {
                        let _ = fs::write(&note.path, &note.content);
                    }
                } else {
                    ui.label("No note open. Create a new note or open an existing one.");
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "rusty-notes",
        options,
        Box::new(|_cc| Box::new(AppState::default())),
    )
}

