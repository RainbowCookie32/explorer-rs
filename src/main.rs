use std::path::PathBuf;

use time::Duration;
use eframe::{egui, epi};
use serde::{Deserialize, Serialize};

#[derive(PartialEq)]
enum EntryType {
    File,
    Folder,
    Symlink
}

struct EntryInfo {
    _type: EntryType,

    name: String,
    path: PathBuf,
    extension: String,
    length: usize,
    permissions: String,

    last_modified: Option<Duration>,
    last_accessed: Option<Duration>,
    last_modification: Option<Duration>
}


#[derive(Deserialize, Serialize)]
struct ExplorerApp {
    initial_path: PathBuf,
    current_path: PathBuf,

    #[serde(skip)]
    current_path_str: String,
    #[serde(skip)]
    editing_current_path: bool,

    #[serde(skip)]
    context_menu_target: usize,
    #[serde(skip)]
    context_menu_response: Option<egui::Response>,

    #[serde(skip)]
    selected_entry: Option<usize>,

    #[serde(skip)]
    previous_path: Vec<PathBuf>,
    #[serde(skip)]
    forward_path: Vec<PathBuf>,

    #[serde(skip)]
    current_dir_items: Vec<EntryInfo>
}

impl Default for ExplorerApp {
    fn default() -> Self {
        let initial_path = dirs::home_dir().expect("Failed to get home path");
        let current_path = initial_path.clone();

        let current_path_str = current_path.to_str().unwrap_or_default().to_string();

        ExplorerApp {
            initial_path,
            current_path,

            current_path_str,
            editing_current_path: false,

            context_menu_target: 0,
            context_menu_response: None,

            selected_entry: None,

            previous_path: Vec::new(),
            forward_path: Vec::new(),

            current_dir_items: Vec::new()
        }
    }
}

impl epi::App for ExplorerApp {
    fn name(&self) -> &str {
        "explorer-rs"
    }

    fn setup(&mut self, _ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>, storage: Option<&dyn epi::Storage>) {
        if let Some(storage) = storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }

        self.update_dir_entries();
    }

    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        let mut refresh_files = false;

        if self.current_path_str.is_empty() {
            self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();
        }

        egui::TopBottomPanel::top("current_path").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!self.previous_path.is_empty(), |ui| {
                    if ui.small_button("⏴").clicked() {
                        if let Some(target_path) = self.previous_path.pop() {
                            self.forward_path.push(self.current_path.clone());
                            self.current_path = target_path;
                            self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

                            refresh_files = true;
                        }
                    }
                });

                ui.add_enabled_ui(!self.forward_path.is_empty(), |ui| {
                    if ui.small_button("⏵").clicked() {
                        if let Some(target_path) = self.forward_path.pop() {
                            self.previous_path.push(self.current_path.clone());
                            self.current_path = target_path;
                            self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

                            refresh_files = true;
                        }
                    }
                });

                ui.add_enabled_ui(self.current_path.parent().is_some(), |ui| {
                    if ui.small_button("⏶").clicked() {
                        if let Some(parent) = self.current_path.parent() {
                            self.current_path = parent.to_path_buf();
                            self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

                            refresh_files = true;
                        }
                    }
                });

                ui.separator();

                if ui.small_button("↻").clicked() {
                    refresh_files = true;
                }

                if self.editing_current_path {
                    if PathBuf::from(&self.current_path_str).exists() {
                        ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(0, 255, 0));
                    }
                    else {
                        ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(255, 0, 0));
                    }
                }

                let path_text = ui.text_edit_singleline(&mut self.current_path_str);
                
                self.editing_current_path = path_text.has_focus();

                if path_text.lost_focus() && ui.input().key_down(egui::Key::Enter) {
                    self.previous_path.push(self.current_path.clone());
                    self.current_path = PathBuf::from(&self.current_path_str);
                    self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

                    refresh_files = true;
                }

                ui.visuals_mut().override_text_color = None;
            });

            egui::Grid::new("header_grid").min_col_width(110.0).show(ui, |ui| {
                // FIXME: This barely works. I need a proper solution for a header.
                ui.label("Name");
                ui.label("Type");
                ui.label("Size");
                ui.label("Creation Time");
                ui.label("Last Accessed");
                ui.label("Last Modified");
                ui.label("Permissions");

                ui.end_row();
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                egui::Grid::new("entries_grid").min_col_width(90.0).show(ui, |ui| {
                    for (idx, entry) in self.current_dir_items.iter().enumerate() {
                        let (label_name, label_type) = match entry._type {
                            EntryType::File => {
                                let file_type = {
                                    if entry.extension.is_empty() {
                                        "File".to_string()
                                    }
                                    else {
                                        format!("{} file", entry.extension)
                                    }
                                };

                                (format!("🗋 {}", entry.name), file_type)
                            }
                            EntryType::Folder => (format!("🗁 {}", entry.name), "Folder".to_string()),
                            EntryType::Symlink => (format!("🔗 {}", entry.name), "Symlink".to_string())
                        };

                        let selected = {
                            if let Some(selection) = self.selected_entry.as_ref() {
                                *selection == idx
                            }
                            else {
                                false
                            }
                        };

                        let label_size = ExplorerApp::size_to_string(entry.length);
                        let entry_label = ui.selectable_label(selected, label_name);

                        if entry_label.double_clicked() {
                            if entry._type == EntryType::File {
                                open::that_in_background(&entry.path);
                            }
                            else {
                                self.previous_path.push(self.current_path.clone());
                                self.current_path = entry.path.clone();
                                self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

                                refresh_files = true;
                            }

                            self.selected_entry = Some(idx);
                        }
                        else if entry_label.clicked() {
                            self.selected_entry = Some(idx);
                        }
                        else if entry_label.secondary_clicked() {
                            self.context_menu_target = idx;
                            self.context_menu_response = Some(entry_label);
                            
                            self.selected_entry = Some(idx);
                        }

                        ui.label(label_type);
                        ui.label(label_size);

                        if let Some(creation_time) = entry.last_modification.as_ref() {
                            ui.label(&ExplorerApp::duration_to_string(creation_time));
                        }

                        if let Some(last_accessed) = entry.last_accessed.as_ref() {
                            ui.label(&ExplorerApp::duration_to_string(last_accessed));
                        }

                        if let Some(last_modified) = entry.last_modified.as_ref() {
                            ui.label(&ExplorerApp::duration_to_string(last_modified));
                        }

                        ui.label(&entry.permissions);
                        ui.end_row();
                    }
                });

                let mut close_popup = false;

                if let Some(target) = self.context_menu_response.as_ref() {
                    let menu_id = ui.make_persistent_id("context_menu");
    
                    ui.memory().open_popup(menu_id);

                    egui::containers::popup_below_widget(ui, menu_id, target, |ui| {
                        ui.set_min_width(150.0);

                        if ui.selectable_label(false, "Open").clicked() {
                            if let Some(entry) = self.current_dir_items.get(self.context_menu_target) {
                                if entry._type == EntryType::File {
                                    open::that_in_background(&entry.path);
                                }
                                else {
                                    self.previous_path.push(self.current_path.clone());
                                    self.current_path = entry.path.clone();
                                    self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

                                    refresh_files = true;
                                }
                            }

                            close_popup = true;
                        }

                        ui.separator();

                        // TODO.
                        ui.add_enabled_ui(false, |ui| {
                            if ui.selectable_label(false, "Cut").clicked() {
                                close_popup = true;
                            }
                        });

                        // TODO.
                        ui.add_enabled_ui(false, |ui| {
                            if ui.selectable_label(false, "Copy").clicked() {
                                close_popup = true;
                            }
                        });

                        ui.separator();

                        // TODO.
                        ui.add_enabled_ui(false, |ui| {
                            if ui.selectable_label(false, "Rename").clicked() {
                                close_popup = true;
                            }
                        });

                        // TODO: This could use a confirmation prompt.
                        if ui.selectable_label(false, "Remove").clicked() {
                            close_popup = true;

                            if let Some(entry) = self.current_dir_items.get(self.context_menu_target) {
                                if entry._type == EntryType::Folder {
                                    if let Err(e) = std::fs::remove_dir_all(&entry.path) {
                                        println!("{}", e.to_string());
                                    }
                                }
                                else {
                                    if let Err(e) = std::fs::remove_file(&entry.path) {
                                        println!("{}", e.to_string());
                                    }
                                }

                                refresh_files = true;
                            }
                        }
                    });
                }

                if close_popup {
                    ui.memory().close_popup();
                            
                    self.context_menu_target = 0;
                    self.context_menu_response = None;
                }
            });

            
        });

        if refresh_files {
            self.selected_entry = None;
            self.update_dir_entries();
        }
    }
}

impl ExplorerApp {
    pub fn update_dir_entries(&mut self) {
        if let Ok(entries) = std::fs::read_dir(&self.current_path) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        let entry_type = {
                            if metadata.is_file() {
                                EntryType::File
                            }
                            else if metadata.is_dir() {
                                EntryType::Folder
                            }
                            else {
                                EntryType::Symlink
                            }
                        };

                        let entry_name = entry.file_name().into_string().unwrap_or_default();
                        let entry_path = entry.path();
                        let entry_extension = entry.path().extension().unwrap_or_default().to_str().unwrap_or_default().to_string();
                        let entry_length = metadata.len() as usize;
                        let entry_permissions = if metadata.permissions().readonly() { "r".to_string() } else { "rw".to_string() };

                        let last_modified = {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(modified) = modified.elapsed() {
                                    Duration::try_from(modified).ok()
                                }
                                else {
                                    None
                                }
                            }
                            else {
                                None
                            }
                        };

                        let last_accessed = {
                            if let Ok(accessed) = metadata.accessed() {
                                if let Ok(accessed) = accessed.elapsed() {
                                    Duration::try_from(accessed).ok()
                                }
                                else {
                                    None
                                }
                            }
                            else {
                                None
                            }
                        };

                        let creation_time = {
                            if let Ok(created) = metadata.created() {
                                if let Ok(created) = created.elapsed() {
                                    Duration::try_from(created).ok()
                                }
                                else {
                                    None
                                }
                            }
                            else {
                                None
                            }
                        };

                        let dir_entry = EntryInfo {
                            _type: entry_type,

                            name: entry_name,
                            path: entry_path,
                            extension: entry_extension,
                            length: entry_length,
                            permissions: entry_permissions,

                            last_modified,
                            last_accessed,
                            last_modification: creation_time
                        };

                        if metadata.is_dir() {
                            dirs.push(dir_entry);
                        }
                        else {
                            files.push(dir_entry);
                        }
                    }
                }
            }

            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            let mut entries = Vec::new();

            entries.append(&mut dirs);
            entries.append(&mut files);

            self.current_dir_items = entries;
        }
    }

    pub fn size_to_string(bytes: usize) -> String {
        bytesize::to_string(bytes as u64, false)
    }

    pub fn duration_to_string(duration: &Duration) -> String {
        if duration.whole_weeks() >= 1 {
            format!("{} weeks ago", duration.whole_weeks())
        }
        else if duration.whole_days() >= 1 {
            format!("{} days ago", duration.whole_days())
        }
        else if duration.whole_hours() >= 1 {
            format!("{} hours ago", duration.whole_days())
        }
        else if duration.whole_minutes() >= 1 {
            format!("{} minutes ago", duration.whole_minutes())
        }
        else {
            format!("{} seconds ago", duration.whole_seconds())
        }
    }
}

fn main() {
    let app = ExplorerApp::default();
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(Box::new(app), native_options);
}
