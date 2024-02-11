#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use eframe::egui;
use egui_extras::TableBuilder;
use time::Duration;
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
    selected_entry: Option<usize>,
    #[serde(skip)]
    renaming_entry: Option<usize>,
    #[serde(skip)]
    renaming_string: String,

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

            selected_entry: None,
            renaming_entry: None,
            renaming_string: String::new(),

            previous_path: Vec::new(),
            forward_path: Vec::new(),

            current_dir_items: Vec::new()
        }
    }
}

impl eframe::App for ExplorerApp {
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.main_app(ctx);
    }
}

impl ExplorerApp {
    fn change_dir(&mut self, new_path: PathBuf) {
        self.selected_entry = None;
        self.previous_path.push(self.current_path.clone());

        self.current_path = new_path;
        self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

        self.update_dir_entries();
    }

    fn previous_dir(&mut self) {
        if let Some(target_path) = self.previous_path.pop() {
            self.forward_path.push(self.current_path.clone());
            self.current_path = target_path;
            self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

            self.selected_entry = None;
            self.update_dir_entries();
        }
    }

    fn forward_dir(&mut self) {
        if let Some(target_path) = self.forward_path.pop() {
            self.previous_path.push(self.current_path.clone());
            self.current_path = target_path;
            self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

            self.selected_entry = None;
            self.update_dir_entries();
        }
    }

    fn previous_level(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.previous_path.push(self.current_path.clone());
            self.current_path = parent.to_path_buf();
            self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();

            self.selected_entry = None;
            self.update_dir_entries();
        }
    }

    fn refresh_dir(&mut self) {
        self.selected_entry = None;
        self.update_dir_entries();
    }

    fn main_app(&mut self, ctx: &egui::Context) {
        if self.current_path_str.is_empty() {
            self.current_path_str = self.current_path.to_str().unwrap_or_default().to_string();
        }

        egui::TopBottomPanel::top("current_path").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!self.previous_path.is_empty(), |ui| {
                    if ui.small_button("⏴").clicked() {
                        self.previous_dir();
                    }
                });

                ui.add_enabled_ui(!self.forward_path.is_empty(), |ui| {
                    if ui.small_button("⏵").clicked() {
                        self.forward_dir();
                    }
                });

                ui.add_enabled_ui(self.current_path.parent().is_some(), |ui| {
                    if ui.small_button("⏶").clicked() {
                        self.previous_level();
                    }
                });

                ui.separator();

                if ui.small_button("↻").clicked() {
                    self.refresh_dir();
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

                if path_text.lost_focus() && ui.input(| i | i.key_down(egui::Key::Enter)) {
                    self.change_dir(PathBuf::from(&self.current_path_str));
                }

                ui.visuals_mut().override_text_color = None;
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                self.fill_files_table(ui);
            });
        });
    }

    fn fill_files_table(&mut self, ui: &mut egui::Ui) {
        let text_size = egui::TextStyle::Body.resolve(ui.style()).size + 10.0;
        let mut new_path = None;

        TableBuilder::new(ui)
            .column(egui_extras::Column::initial(300.0))
            .column(egui_extras::Column::initial(100.0))
            .column(egui_extras::Column::initial(80.0))
            .column(egui_extras::Column::initial(100.0))
            .column(egui_extras::Column::initial(100.0))
            .column(egui_extras::Column::initial(100.0))
            .column(egui_extras::Column::remainder())
            .resizable(true)
            .striped(true)
            .header(20.0, | mut header | {
                header.col(| ui | {
                    ui.strong("Name");
                });

                header.col(| ui | {
                    ui.strong("Type");
                });

                header.col(| ui | {
                    ui.strong("Size");
                });

                header.col(| ui | {
                    ui.strong("Creation Time");
                });

                header.col(| ui | {
                    ui.strong("Last Accessed");
                });

                header.col(| ui | {
                    ui.strong("Last Modified");
                });

                header.col(| ui | {
                    ui.strong("Permissions");
                });
            })
            .body(| body | {
                body.rows(text_size, self.current_dir_items.len(), | mut row | {
                    let row_idx = row.index();

                    if let Some(entry) = self.current_dir_items.get(row_idx) {
                        let (entry_name, entry_type) = match entry._type {
                            EntryType::File => {
                                let file_type = {
                                    if let Ok(t) = file_format::FileFormat::from_file(&entry.path) {
                                        t.media_type().to_string()
                                    }
                                    else {
                                        "File".to_string()
                                    }
                                };
            
                                (format!("🗋 {}", entry.name), file_type)
                            }
                            EntryType::Folder => (format!("🗁 {}", entry.name), "Folder".to_string()),
                            EntryType::Symlink => (format!("🔗 {}", entry.name), "Symlink".to_string())
                        };

                        row.col(| ui | {
                            let renaming = {
                                if let Some(target) = self.renaming_entry.as_ref() {
                                    row_idx == *target
                                }
                                else {
                                    false
                                }
                            };

                            if renaming {
                                // Red highlight for the text if there is a file with the same name.
                                if entry.name != self.renaming_string {
                                    if let Some(path) = entry.path.parent() {
                                        if path.join(PathBuf::from(&self.renaming_string)).exists() {
                                            ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(255, 0, 0));
                                        }
                                    }
                                }

                                let entry_label = {
                                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), | ui | {
                                        ui.text_edit_singleline(&mut self.renaming_string)
                                    }).response
                                };

                                if entry_label.lost_focus() {
                                    // User committed the changes.
                                    if ui.input(| i | i.key_pressed(egui::Key::Enter)) {
                                        // Check if an entry with the same name already exists.
                                        if let Some(parent) = entry.path.parent() {
                                            let new_entry = parent.join(PathBuf::from(&self.renaming_string));
                                        
                                            // There's already an entry on this directory with that name, don't rename.
                                            if !new_entry.exists() {
                                                if let Err(e) = std::fs::rename(&entry.path, new_entry) {
                                                    println!("{}", e);
                                                }   
                                            }
                                        }
                                    }

                                    // Forcing a refresh for the current dir.
                                    new_path = Some(self.current_path.clone());
                    
                                    self.renaming_entry = None;
                                    self.renaming_string = String::new();
                                }
                                else {
                                    entry_label.request_focus();
                                }

                                ui.visuals_mut().override_text_color = None;
                            }
                            else {
                                let is_selected = {
                                    if let Some(selection) = self.selected_entry.as_ref() {
                                        *selection == row_idx
                                    }
                                    else {
                                        false
                                    }
                                };
                                
                                let entry_label = {
                                    ui.push_id(&entry.name, | ui | {
                                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), | ui | {
                                            ui.selectable_label(is_selected, entry_name)
                                        }).inner
                                    }).inner
                                };
                
                                if entry_label.double_clicked() {
                                    if entry._type == EntryType::File {
                                        open::that_in_background(&entry.path);
                                    }
                                    else {
                                        new_path = Some(entry.path.clone());
                                    }
                    
                                    self.selected_entry = Some(row_idx);
                                }
                                else if entry_label.clicked() {
                                    self.selected_entry = Some(row_idx);
                                }
                
                                entry_label.context_menu(| ui | {
                                    if ui.selectable_label(false, "Open").clicked() {
                                        if entry.path.exists() {
                                            if entry._type == EntryType::File {
                                                open::that_in_background(&entry.path);
                                            }
                                            else {
                                                new_path = Some(entry.path.clone());
                                            }
                                        }
                
                                        ui.close_menu();
                                    }

                                    if entry._type == EntryType::Folder {
                                        if ui.selectable_label(false, "Open in new window").clicked() {
                                            ui.close_menu();
                                            
                                            let vp_id = egui::ViewportId::from_hash_of(&entry.path);
                                            let vp_builder = egui::ViewportBuilder::default()
                                                .with_title("explorer-rs")
                                            ;

                                            let entry_path = entry.path.clone();

                                            ui.ctx().show_viewport_deferred(vp_id, vp_builder, move | ctx, _ | {
                                                let entry_path = &entry_path;
                                                let mut new_state = ExplorerApp::default();
                                                new_state.change_dir(entry_path.to_path_buf());

                                                new_state.main_app(ctx);
                                            });
                                        }
                                    }
                
                                    ui.separator();
                
                                    // TODO.
                                    ui.add_enabled_ui(false, |ui| {
                                        if ui.selectable_label(false, "Cut").clicked() {
                                            ui.close_menu();
                                        }
                                    });
                
                                    // TODO.
                                    ui.add_enabled_ui(false, |ui| {
                                        if ui.selectable_label(false, "Copy").clicked() {
                                            ui.close_menu();
                                        }
                                    });
                
                                    ui.separator();
                
                                    if ui.selectable_label(false, "Rename").clicked() {
                                        self.renaming_entry = Some(row_idx);
                                        self.renaming_string = entry.name.clone();
                
                                        ui.close_menu();
                                    }
                
                                    // TODO: This could use a confirmation prompt.
                                    if ui.selectable_label(false, "Remove").clicked() {
                                        if entry._type == EntryType::Folder {
                                            if let Err(e) = std::fs::remove_dir_all(&entry.path) {
                                                println!("{}", e);
                                            }
                                        }
                                        else if let Err(e) = std::fs::remove_file(&entry.path) {
                                            println!("{}", e);
                                        }
                
                                        new_path = Some(self.current_path.clone());
                                        ui.close_menu();
                                    }
                                });
                            }
                        });

                        row.col(| ui | {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), | ui | {
                                ui.label(entry_type);
                            });
                        });

                        row.col(| ui | {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), | ui | {
                                ui.label(ExplorerApp::size_to_string(entry.length)); 
                            });
                        });

                        row.col(| ui | {
                            if let Some(creation_time) = entry.last_modification.as_ref() {
                                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), | ui | {
                                    ui.label(&ExplorerApp::duration_to_string(creation_time));
                                });
                            }
                        });

                        row.col(| ui | {
                            if let Some(last_accessed) = entry.last_accessed.as_ref() {
                                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), | ui | {
                                    ui.label(&ExplorerApp::duration_to_string(last_accessed));
                                });
                            }
                        });

                        row.col(| ui | {
                            if let Some(last_modified) = entry.last_modified.as_ref() {
                                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), | ui | {
                                    ui.label(&ExplorerApp::duration_to_string(last_modified));
                                });
                            }
                        });

                        row.col(| ui | {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), | ui | {
                                ui.label(&entry.permissions); 
                            });
                        });
                    }
                });
            })
        ;

        if let Some(new_path) = new_path {
            self.change_dir(new_path);
        }
    }

    pub fn update_dir_entries(&mut self) {
        if let Ok(entries) = std::fs::read_dir(&self.current_path) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries.flatten() {
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
    let mut app = ExplorerApp::default();
    let native_options = eframe::NativeOptions::default();

    app.update_dir_entries();

    eframe::run_native("explorer-rs", native_options, Box::new(|_| Box::new(app)));
}
