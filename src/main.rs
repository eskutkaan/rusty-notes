use eframe::egui::{CentralPanel, Context, TextEdit, TopBottomPanel, SidePanel, Layout};
use std::fs;
use std::path::PathBuf;

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
}

impl Default for AppState {
    fn default() -> Self {
        let notes_dir = std::env::current_dir().unwrap().join("notes");
        fs::create_dir_all(&notes_dir).unwrap();
        let mut notes = vec![];
        for entry in fs::read_dir(&notes_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "md") {
                let content = fs::read_to_string(&path).unwrap_or_default();
                let title = path.file_stem().unwrap().to_string_lossy().into_owned();
                notes.push(Note { title, content, path });
            }
        }
        Self {
            notes,
            open_tabs: vec![],
            current_tab: None,
            search_query: String::new(),
            notes_dir,
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Top panel with new‑note button and search bar
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("+ New Note").clicked() {
                    let title = format!("Note_{}.md", self.notes.len() + 1);
                    let path = self.notes_dir.join(&title);
                    fs::write(&path, "").unwrap();
                    let note = Note {
                        title: title.trim_end_matches(".md").to_string(),
                        content: String::new(),
                        path,
                    };
                    self.notes.push(note);
                }

                ui.text_edit_singleline(&mut self.search_query);
            });
        });

        // Resizable left sidebar (width not persisted)
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
                        if ui.button(&note.title).clicked() {
                            if !self.open_tabs.contains(&i) {
                                self.open_tabs.push(i);
                            }
                            self.current_tab = Some(i);
                        }
                        if ui.button("🗑").clicked() {
                            note_to_remove = Some(i);
                        }
                    });
                }

                if let Some(i) = note_to_remove {
                    let _ = fs::remove_file(&self.notes[i].path);
                    self.notes.remove(i);

                    // Remove the deleted note from open_tabs and adjust other indices
                    self.open_tabs.retain(|&x| x != i);
                    for tab in self.open_tabs.iter_mut() {
                        if *tab > i {
                            *tab -= 1;
                        }
                    }

                    // Fix current_tab after removal
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
            });

        // Central panel with tab bar and full-area editor
        CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(eframe::egui::Align::Min), |ui| {
                ui.horizontal_wrapped(|ui| {
                    for &tab_idx in &self.open_tabs {
                        let note = &self.notes[tab_idx];
                        let selected = self.current_tab == Some(tab_idx);
                        if ui.selectable_label(selected, &note.title).clicked() {
                            self.current_tab = Some(tab_idx);
                        }
                    }
                });

                ui.separator();

                if let Some(idx) = self.current_tab {
                    if let Some(note) = self.notes.get_mut(idx) {
                        let response = ui.add_sized(ui.available_size(), TextEdit::multiline(&mut note.content));
                        if response.changed() {
                            let _ = fs::write(&note.path, &note.content);
                        }
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
