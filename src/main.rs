use eframe::egui::{self, CentralPanel, Context, Key, Layout, RichText, 
    ScrollArea, SidePanel, TextEdit, TextStyle, TopBottomPanel, Visuals};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

struct Note {
    title: String,
    content: String,
    path: PathBuf,
    unsaved_changes: bool,
    last_saved: Instant,
}

struct ConfirmationDialog {
    open: bool,
    title: String,
    message: String,
    action_type: DialogAction,
    target_index: Option<usize>,
}

#[derive(Clone)]
enum DialogAction {
    DeleteNote,
    CloseUnsavedTab,
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
    show_preview: bool,
    confirmation_dialog: ConfirmationDialog,
    autosave_interval: Duration,
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
                    notes.push(Note { 
                        title, 
                        content, 
                        path,
                        unsaved_changes: false,
                        last_saved: Instant::now(),
                    });
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
            show_preview: false,
            confirmation_dialog: ConfirmationDialog {
                open: false,
                title: String::new(),
                message: String::new(),
                action_type: DialogAction::DeleteNote,
                target_index: None,
            },
            autosave_interval: Duration::from_secs(30),
        }
    }
}

impl AppState {
    fn create_note(&mut self) {
        let title = format!("Note_{}", self.notes.len() + 1);
        let safe_title = title.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
        let path = self.notes_dir.join(format!("{}.md", safe_title));
        if fs::write(&path, "").is_ok() {
            let note = Note {
                title: safe_title.trim_end_matches(".md").to_string(),
                content: String::new(),
                path,
                unsaved_changes: false,
                last_saved: Instant::now(),
            };
            self.notes.push(note);
            let _idx = self.notes.len() - 1;
            self.notes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            
            // Find the index after sorting
            let new_idx = self.notes.iter().position(|n| n.title == safe_title).unwrap_or(0);
            self.open_tabs.push(new_idx);
            self.current_tab = Some(new_idx);
        }
    }

    fn delete_note(&mut self, i: usize) {
        let _ = fs::remove_file(&self.notes[i].path);
        self.notes.remove(i);
        
        // Update open tabs
        self.open_tabs.retain(|&x| x != i);
        for tab in self.open_tabs.iter_mut() {
            if *tab > i {
                *tab -= 1;
            }
        }
        
        // Update current tab
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
        if new_title.is_empty() {
            return;
        }
        
        if let Some(note) = self.notes.get_mut(idx) {
            let safe_title = new_title.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
            let new_path = self.notes_dir.join(format!("{}.md", safe_title));
            
            // Skip if the title hasn't changed
            if note.title == safe_title {
                return;
            }
            
            // Make a reference to the old path to compare later
            let old_path = note.path.clone();
            
            if fs::rename(&note.path, &new_path).is_ok() {
                note.title = safe_title;
                note.path = new_path;
                note.unsaved_changes = true;
                
                // Store the current index for this note
                let current_idx = idx;
                
                // Re-sort notes
                self.notes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
                
                // Find the new index of the renamed note
                let new_idx = self.notes.iter().position(|n| n.path == old_path).unwrap_or(current_idx);
                
                // Update open tabs indices
                for i in 0..self.open_tabs.len() {
                    if self.open_tabs[i] == current_idx {
                        self.open_tabs[i] = new_idx;
                    }
                }
                
                // Update current tab
                if let Some(tab_idx) = self.current_tab {
                    if tab_idx == current_idx {
                        self.current_tab = Some(new_idx);
                    }
                }
            }
        }
    }
    
    fn find_note_by_path(&self, path: &Path) -> Option<usize> {
        self.notes.iter().position(|note| note.path == path)
    }
    
    fn save_current_note(&mut self) -> bool {
        if let Some(idx) = self.current_tab {
            let note = &mut self.notes[idx];
            if note.unsaved_changes {
                if fs::write(&note.path, &note.content).is_ok() {
                    note.unsaved_changes = false;
                    note.last_saved = Instant::now();
                    return true;
                }
            }
        }
        false
    }
    
    fn autosave_notes(&mut self) {
        let now = Instant::now();
        for (_i, note) in self.notes.iter_mut().enumerate() {
            if note.unsaved_changes && now.duration_since(note.last_saved) >= self.autosave_interval {
                if fs::write(&note.path, &note.content).is_ok() {
                    note.unsaved_changes = false;
                    note.last_saved = now;
                }
            }
        }
    }
    
    fn count_words_and_chars(&self, idx: usize) -> (usize, usize) {
        if let Some(note) = self.notes.get(idx) {
            let chars = note.content.chars().count();
            let words = note.content.split_whitespace().count();
            (words, chars)
        } else {
            (0, 0)
        }
    }
    
    fn render_markdown_to_html(&self, markdown: &str) -> String {
        // Simple markdown rendering without using pulldown_cmark
        let mut html_output = String::new();
        
        // Process line by line for basic markdown support
        for line in markdown.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                html_output.push_str("<p></p>\n");
            } else if trimmed.starts_with("# ") {
                html_output.push_str(&format!("<h1>{}</h1>\n", &trimmed[2..]));
            } else if trimmed.starts_with("## ") {
                html_output.push_str(&format!("<h2>{}</h2>\n", &trimmed[3..]));
            } else if trimmed.starts_with("### ") {
                html_output.push_str(&format!("<h3>{}</h3>\n", &trimmed[4..]));
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                html_output.push_str(&format!("<li>{}</li>\n", &trimmed[2..]));
            } else if trimmed.starts_with("> ") {
                html_output.push_str(&format!("<blockquote>{}</blockquote>\n", &trimmed[2..]));
            } else if trimmed.starts_with("```") {
                html_output.push_str("<pre><code>\n");
            } else if trimmed.ends_with("```") {
                html_output.push_str("</code></pre>\n");
            } else {
                html_output.push_str(&format!("<p>{}</p>\n", trimmed));
            }
        }
        
        html_output
    }
    
    fn show_confirmation_dialog(&mut self, ctx: &Context) -> Option<DialogAction> {
        if !self.confirmation_dialog.open {
            return None;
        }
        
        let mut action = None;
        
        egui::Window::new(&self.confirmation_dialog.title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(&self.confirmation_dialog.message);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.confirmation_dialog.open = false;
                    }
                    
                    let confirm_text = match self.confirmation_dialog.action_type {
                        DialogAction::DeleteNote => "Delete",
                        DialogAction::CloseUnsavedTab => "Close without saving",
                    };
                    
                    if ui.button(confirm_text).clicked() {
                        action = Some(self.confirmation_dialog.action_type.clone());
                        self.confirmation_dialog.open = false;
                    }
                });
            });
            
        action
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Process keyboard shortcuts
        let ctrl = ctx.input(|i| i.modifiers.ctrl);
        let _shift = ctx.input(|i| i.modifiers.shift);
        
        // Keyboard shortcuts
        if ctrl {
            if ctx.input(|i| i.key_pressed(Key::N)) {
                // Ctrl+N: New note
                self.create_note();
            } else if ctx.input(|i| i.key_pressed(Key::S)) {
                // Ctrl+S: Save current note
                self.save_current_note();
            } else if ctx.input(|i| i.key_pressed(Key::P)) {
                // Ctrl+P: Toggle preview
                self.show_preview = !self.show_preview;
            } else if ctx.input(|i| i.key_pressed(Key::W)) {
                // Ctrl+W: Close current tab
                if let Some(idx) = self.current_tab {
                    let note = &self.notes[idx];
                    if note.unsaved_changes {
                        // Show confirmation dialog
                        self.confirmation_dialog = ConfirmationDialog {
                            open: true,
                            title: "Unsaved Changes".to_string(),
                            message: format!("The note \"{}\" has unsaved changes. Close without saving?", note.title),
                            action_type: DialogAction::CloseUnsavedTab,
                            target_index: Some(idx),
                        };
                    } else {
                        self.open_tabs.retain(|&x| x != idx);
                        self.current_tab = self.open_tabs.last().copied();
                    }
                }
            }
        }
        
        // Apply theme
        ctx.set_visuals(if self.dark_mode {
            Visuals::dark()
        } else {
            Visuals::light()
        });
        
        // Process any dialog actions
        if let Some(action) = self.show_confirmation_dialog(ctx) {
            match action {
                DialogAction::DeleteNote => {
                    if let Some(idx) = self.confirmation_dialog.target_index {
                        self.delete_note(idx);
                    }
                },
                DialogAction::CloseUnsavedTab => {
                    if let Some(idx) = self.confirmation_dialog.target_index {
                        self.open_tabs.retain(|&x| x != idx);
                        self.current_tab = self.open_tabs.last().copied();
                    }
                }
            }
        }
        
        // Periodic autosave check
        self.autosave_notes();

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Remove the redundant "New Note" button and keep only the icon button
                if ui.button("📄").on_hover_text("New Note (Ctrl+N)").clicked() {
                    self.create_note();
                }
                
                if ui.button("💾").on_hover_text("Save (Ctrl+S)").clicked() {
                    self.save_current_note();
                }
                
                ui.separator();
                
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);
                
                ui.separator();
                
                if ui.button(if self.dark_mode { "🌞 Light" } else { "🌙 Dark" }).clicked() {
                    self.dark_mode = !self.dark_mode;
                }
                
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(_idx) = self.current_tab {
                        if ui.button(if self.show_preview { "✏️ Edit" } else { "👁️ Preview" })
                            .on_hover_text("Toggle Preview (Ctrl+P)")
                            .clicked() 
                        {
                            self.show_preview = !self.show_preview;
                        }
                    }
                });
            });
        });

        SidePanel::left("side_panel")
            .resizable(true)
            .default_width(180.0)
            .min_width(120.0)
            .show(ctx, |ui| {
                ui.heading("Notes");
                ui.separator();
                
                let filtered_notes: Vec<_> = self.notes.iter().enumerate()
                    .filter(|(_, note)| {
                        let query = self.search_query.to_lowercase();
                        query.is_empty() || 
                            note.title.to_lowercase().contains(&query) || 
                            note.content.to_lowercase().contains(&query)
                    })
                    .collect();
                
                ScrollArea::vertical().show(ui, |ui| {
                    for &(i, note) in &filtered_notes {
                        ui.horizontal(|ui| {
                            let mut title_text = note.title.clone();
                            if note.unsaved_changes {
                                title_text.push('*');
                            }
                            
                            // Highlight open notes
                            let is_open = self.open_tabs.contains(&i);
                            let is_current = self.current_tab == Some(i);
                            
                            let text = if is_current {
                                RichText::new(&title_text).strong()
                            } else if is_open {
                                RichText::new(&title_text).italics()
                            } else {
                                RichText::new(&title_text)
                            };
                            
                            if ui.button(text).on_hover_text("Open note").clicked() {
                                if !self.open_tabs.contains(&i) {
                                    self.open_tabs.push(i);
                                }
                                self.current_tab = Some(i);
                            }
                            
                            if ui.button("🗑").on_hover_text("Delete note").clicked() {
                                self.confirmation_dialog = ConfirmationDialog {
                                    open: true,
                                    title: "Confirm Deletion".to_string(),
                                    message: format!("Are you sure you want to delete \"{}\"?", note.title),
                                    action_type: DialogAction::DeleteNote,
                                    target_index: Some(i),
                                };
                            }
                        });
                    }
                    
                    if filtered_notes.is_empty() {
                        ui.label("No notes match your search.");
                    }
                });
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(eframe::egui::Align::Min), |ui| {
                // Tab bar
                ui.horizontal_wrapped(|ui| {
                    let mut tab_to_close: Option<usize> = None;
                    
                    for &tab_idx in &self.open_tabs {
                        let note = &self.notes[tab_idx];
                        let selected = self.current_tab == Some(tab_idx);
                        
                        ui.horizontal(|ui| {
                            let mut title_text = note.title.clone();
                            if note.unsaved_changes {
                                title_text.push('*');
                            }
                            
                            let text = if selected {
                                RichText::new(title_text).strong()
                            } else {
                                RichText::new(title_text)
                            };
                            
                            if ui.selectable_label(selected, text).clicked() {
                                self.current_tab = Some(tab_idx);
                            }
                            
                            if ui.button("❌").on_hover_text("Close tab (Ctrl+W)").clicked() {
                                let note = &self.notes[tab_idx];
                                if note.unsaved_changes {
                                    // Show confirmation dialog
                                    self.confirmation_dialog = ConfirmationDialog {
                                        open: true,
                                        title: "Unsaved Changes".to_string(),
                                        message: format!("The note \"{}\" has unsaved changes. Close without saving?", note.title),
                                        action_type: DialogAction::CloseUnsavedTab,
                                        target_index: Some(tab_idx),
                                    };
                                } else {
                                    tab_to_close = Some(tab_idx);
                                }
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
                    // Note title area
                    let title = self.notes[idx].title.clone();

                    if self.editing_title == Some(idx) {
                        // Title editing mode
                        let mut new_title = self.editing_title_buffer.clone();
                        ui.horizontal(|ui| {
                            let _title_edit = ui.text_edit_singleline(&mut new_title);
                            self.editing_title_buffer = new_title.clone();  // Update the buffer with changes

                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let ok_clicked = ui.button("OK").clicked();
                            let cancel_clicked = ui.button("Cancel").clicked();

                            if enter_pressed || ok_clicked {
                                let new_title = self.editing_title_buffer.clone();
        self.rename_note(idx, &new_title);
                                self.editing_title = None;
                            } else if cancel_clicked {
                                self.editing_title = None;
                            }
                        });
                    } else {
                        // Normal title display
                        ui.horizontal(|ui| {
                            ui.heading(&title);
                            if ui.button("✏️ Rename").clicked() {
                                self.editing_title = Some(idx);
                                self.editing_title_buffer = title;
                            }
                        });
                    }

                    // Note content area with preview
                    if self.show_preview {
                        // Make a copy of the content for preview
                        let content_copy = self.notes[idx].content.clone();
                        let html_content = self.render_markdown_to_html(&content_copy);
                        
                        ScrollArea::vertical().show(ui, |ui| {
                            ui.add_space(5.0);
                            ui.label(RichText::new("Preview Mode").italics());
                            ui.separator();
                            
                            // Basic HTML rendering with Label
                            for line in html_content.lines() {
                                let clean_line = line.trim();
                                if !clean_line.is_empty() {
                                    if clean_line.starts_with("<h1>") {
                                        let text = clean_line.replace("<h1>", "").replace("</h1>", "");
                                        ui.heading(text);
                                    } else if clean_line.starts_with("<h2>") {
                                        let text = clean_line.replace("<h2>", "").replace("</h2>", "");
                                        ui.heading(text);
                                    } else if clean_line.starts_with("<h3>") {
                                        let text = clean_line.replace("<h3>", "").replace("</h3>", "");
                                        ui.heading(text);
                                    } else if clean_line.starts_with("<p>") {
                                        let text = clean_line.replace("<p>", "").replace("</p>", "");
                                        ui.label(text);
                                    } else if clean_line.starts_with("<ul>") || 
                                              clean_line.starts_with("</ul>") || 
                                              clean_line.starts_with("<ol>") || 
                                              clean_line.starts_with("</ol>") {
                                        // Skip list container tags
                                        continue;
                                    } else if clean_line.starts_with("<li>") {
                                        let text = clean_line.replace("<li>", "• ").replace("</li>", "");
                                        ui.label(text);
                                    } else if clean_line.starts_with("<blockquote>") {
                                        let text = clean_line.replace("<blockquote>", "").replace("</blockquote>", "");
                                        ui.label(RichText::new(text).italics());
                                    } else if clean_line.starts_with("<pre>") || 
                                              clean_line.starts_with("<code>") || 
                                              clean_line.starts_with("</pre>") || 
                                              clean_line.starts_with("</code>") {
                                        // Handle code blocks
                                        let text = clean_line
                                            .replace("<pre>", "")
                                            .replace("</pre>", "")
                                            .replace("<code>", "")
                                            .replace("</code>", "");
                                        if !text.is_empty() {
                                            ui.monospace(text);
                                        }
                                    } else {
                                        // Default rendering for other elements
                                        ui.label(clean_line);
                                    }
                                } else {
                                    ui.add_space(5.0);
                                }
                            }
                        });
                    } else {
                        // Edit mode
                        let available_size = ui.available_size();
                        let editor_size = egui::Vec2::new(
                            available_size.x,
                            available_size.y - 20.0  // Reserve space for status bar
                        );
                        
                        let mut content = self.notes[idx].content.clone();
                        let response = ui.add_sized(
                            editor_size,
                            TextEdit::multiline(&mut content)
                                .font(TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                        );
                        
                        if response.changed() {
                            self.notes[idx].unsaved_changes = true;
                            self.notes[idx].content = content;
                        }
                    }
                    
                    // Status bar
                    TopBottomPanel::bottom("status_bar").show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Get a copy of the note info for the status bar
                            let unsaved = self.notes[idx].unsaved_changes;
                            let (words, chars) = self.count_words_and_chars(idx);
                            
                            ui.label(format!("Words: {}, Characters: {}", words, chars));
                            
                            ui.with_layout(Layout::right_to_left(egui::Align::RIGHT), |ui| {
                                if unsaved {
                                    ui.label(RichText::new("Unsaved changes").italics());
                                } else {
                                    ui.label(RichText::new("Saved").italics());
                                }
                            });
                        });
                    });
                    
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("No note open");
                        ui.label("Create a new note or open an existing one");
                        ui.add_space(10.0);
                        if ui.button("Create New Note").clicked() {
                            self.create_note();
                        }
                    });
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    // Note: No need for the pulldown_cmark dependency as we're using our own markdown renderer
    
    let options = eframe::NativeOptions {
        // Since the API changed, use default options
        ..Default::default()
    };
    
    eframe::run_native(
        "rusty-notes",
        options,
        Box::new(|_cc| Box::new(AppState::default())),
    )
}
