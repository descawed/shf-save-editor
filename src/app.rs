use std::fs::File;
use std::path::PathBuf;

use binrw::BinReaderExt;
use binrw::BinWriterExt;
use eframe::egui;
use egui::{RichText, ViewportCommand};

use crate::save::*;
use crate::uobject::Stringable;

const BINARY_DATA_CUTOFF: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListAction {
    None,
    Delete(usize),
    Insert(usize),
}

impl ListAction {
    fn update(&mut self, action: Self) {
        if self == &Self::None {
            *self = action;
        }
    }
}

impl Default for ListAction {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Default)]
pub struct AppState {
    save_path: Option<PathBuf>,
    save: Option<SaveGame>,
    error_message: Option<String>,
}

impl AppState {
    fn error_modal(&mut self, ctx: &egui::Context) {
        let Some(ref error_message) = self.error_message else {
            return;
        };

        let response = egui::Modal::new(egui::Id::new("Error Modal")).show(ctx, |ui| {
            ui.label(RichText::new("Error").strong());
            ui.separator();
            ui.vertical_centered(|ui| {
                ui.label(error_message);
                ui.button("OK").clicked()
            }).inner
        });

        if response.should_close() || response.inner {
            self.error_message = None;
        }
    }

    fn load_save(&mut self, save_path: PathBuf) -> anyhow::Result<()> {
        let mut file = File::open(&save_path)?;
        let save: SaveGame = file.read_le()?;
        self.save_path = Some(save_path);
        self.save = Some(save);
        Ok(())
    }

    fn open_save(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Silent Hill f save", &["sav"])
            .pick_file()
        {
            if let Err(err) = self.load_save(path) {
                self.error_message = Some(format!("Failed to load save: {err}"));
            }
        }
    }

    fn save_as(&mut self) {
        let Some(ref save) = self.save else { return; };

        let mut dialog = rfd::FileDialog::new()
            .add_filter("Silent Hill f save", &["sav"]);

        if let Some(path) = &self.save_path {
            if let Some(parent) = path.parent() {
                dialog = dialog.set_directory(parent);
            }
        }

        if let Some(path) = dialog.save_file() {
            let result: anyhow::Result<()> = (|| {
                let mut file = File::create(&path)?;
                file.write_le(save)?;
                Ok(())
            })();

            if let Err(err) = result {
                self.error_message = Some(format!("Failed to save: {err}"));
            } else {
                self.save_path = Some(path);
            }
        }
    }

    fn typed_input<T: Stringable + ?Sized>(ui: &mut egui::Ui, label: &str, value: &mut T) {
        ui.horizontal(|ui| {
            ui.label(format!("{label}: "));
            let mut string = value.to_string();
            if ui.text_edit_singleline(&mut string).changed() {
                value.try_set_from_str(&string);
            }
        });
    }

    fn text_input(ui: &mut egui::Ui, label: &str, value: &mut FString) {
        ui.horizontal(|ui| {
            ui.label(format!("{label}: "));
            ui.text_edit_singleline(value.as_mut());
        });
    }

    fn show_header(&mut self, ui: &mut egui::Ui) {
        let Some(save) = &mut self.save else {
            return;
        };

        Self::typed_input(ui, "Save Game Version", &mut save.header.save_game_version);
        egui::CollapsingHeader::new("Package Version")
            .default_open(true)
            .show(ui, |ui| {
                Self::typed_input(ui, "UE4", &mut save.header.package_version.0);
                Self::typed_input(ui, "UE5", &mut save.header.package_version.1);
            });
        egui::CollapsingHeader::new("Engine Version")
            .default_open(true)
            .show(ui, |ui| {
                Self::typed_input(ui, "Major", &mut save.header.engine_version.major);
                Self::typed_input(ui, "Minor", &mut save.header.engine_version.minor);
                Self::typed_input(ui, "Patch", &mut save.header.engine_version.patch);
                Self::typed_input(ui, "Build", &mut save.header.engine_version.build);
                Self::text_input(ui, "Build ID", &mut save.header.engine_version.build_id);
            });
    }

    fn show_custom_format(&mut self, ui: &mut egui::Ui) {
        let Some(save) = &mut self.save else {
            return;
        };

        Self::typed_input(ui, "Version", &mut save.custom_format_data.version);

        let num_entries = save.custom_format_data.entries.len();
        egui::CollapsingHeader::new(format!("Entries ({num_entries})"))
            .show(ui, |ui| {
                for (i, entry) in save.custom_format_data.entries.iter_mut().enumerate() {
                    egui::CollapsingHeader::new(i.to_string())
                        .show(ui, |ui| {
                            Self::typed_input(ui, "GUID", &mut entry.guid);
                            Self::typed_input(ui, "Value", &mut entry.value);
                        });
                }
            });
    }

    fn show_type(ui: &mut egui::Ui, property_type: &mut PropertyType) {
        Self::text_input(ui, "Name", &mut property_type.name);

        let num_tags = property_type.tags.len();
        egui::CollapsingHeader::new(format!("Tags ({num_tags})"))
            .show(ui, |ui| {
                for (i, tag) in property_type.tags.iter_mut().enumerate() {
                    egui::CollapsingHeader::new(i.to_string())
                        .default_open(true)
                        .show(ui, |ui| {
                            Self::typed_input(ui, "Kind", &mut tag.kind);
                            Self::text_input(ui, "Value", &mut tag.value);
                        });
                }
            });

        // FIXME: how to initialize the inner type if the user changes the type from one that has no
        //  inner type to one that does?
        for inner_type in &mut property_type.inner_types {
            egui::CollapsingHeader::new(format!("Inner Type: {}", inner_type.name))
                .show(ui, |ui| {
                    Self::show_type(ui, inner_type);
                });
        }
    }

    fn show_binary_data(ui: &mut egui::Ui, label: &str, data: &[u8]) {
        let mut desc = format!("{label}: ");
        for (i, b) in data.iter().enumerate() {
            if i >= BINARY_DATA_CUTOFF {
                desc.push_str(&format!("... ({})", data.len()));
                break;
            }
            desc.push_str(&format!("{b:02X} "));
        }
        ui.label(desc);
    }

    fn show_list_context_menu(ui: &mut egui::Ui, index: usize) -> ListAction {
        ui.menu_button("☰", |ui| {
            if ui.button("Insert above").clicked() {
                return ListAction::Insert(index);
            }
            if ui.button("Insert below").clicked() {
                return ListAction::Insert(index + 1);
            }
            ui.separator();
            if ui.button("Delete").clicked() {
                return ListAction::Delete(index);
            }
            ListAction::None
        }).inner.unwrap_or_default()
    }

    fn show_property_value(ui: &mut egui::Ui, label: &str, property_value: &mut PropertyValue, flags: Option<&mut u8>, property_type: &PropertyType) {
        match property_value {
            PropertyValue::StrProperty(s) | PropertyValue::NameProperty(s) | PropertyValue::EnumProperty(s) | PropertyValue::ObjectProperty(s) => {
                Self::text_input(ui, label, s);
            }
            PropertyValue::BoolProperty(b) => {
                if let Some(value) = b {
                    ui.checkbox(value, label);
                } else {
                    let flags = flags.expect("flags should not be None if the BoolProperty value is also None");
                    let mut value = *flags & 0x10 != 0;
                    ui.checkbox(&mut value, label);
                    if value {
                        *flags |= 0x10;
                    } else {
                        *flags &= !0x10;
                    }
                }
            }
            PropertyValue::ByteProperty(b) => {
                Self::typed_input(ui, label, b);
            }
            PropertyValue::IntProperty(i) => {
                Self::typed_input(ui, label, i);
            }
            PropertyValue::FloatProperty(f) => {
                Self::typed_input(ui, label, f);
            }
            PropertyValue::DoubleProperty(d) => {
                Self::typed_input(ui, label, d);
            }
            PropertyValue::TextProperty { flags, data } => {
                egui::CollapsingHeader::new(label)
                    .show(ui, |ui| {
                        let mut int_flags = flags.bits();
                        Self::typed_input(ui, "Flags", &mut int_flags);
                        *flags = TextFlags::from_bits(int_flags).unwrap();
                        // TODO: implement selector for TextData type
                        match data {
                            TextData::None { values } => {
                                let num_values = values.len();
                                egui::CollapsingHeader::new(format!("Values ({num_values})"))
                                    .show(ui, |ui| {
                                        for (i, value) in values.iter_mut().enumerate() {
                                            Self::text_input(ui, &i.to_string(), value);
                                        }
                                    });
                            }
                            TextData::Base { namespace, key, source_string } => {
                                Self::text_input(ui, "Namespace", namespace);
                                Self::text_input(ui, "Key", key);
                                Self::text_input(ui, "Source String", source_string);
                            }
                            TextData::AsDateTime { ticks, date_style, time_style, time_zone, culture_name } => {
                                Self::typed_input(ui, "Ticks", ticks);
                                Self::typed_input(ui, "Date Style", date_style);
                                Self::typed_input(ui, "Time Style", time_style);
                                Self::text_input(ui, "Time Zone", time_zone);
                                Self::text_input(ui, "Culture Name", culture_name);
                            }
                            TextData::StringTableEntry { table, key } => {
                                Self::text_input(ui, "Table", table);
                                Self::text_input(ui, "Key", key);
                            }
                        }
                    });
            }
            PropertyValue::StructProperty(props) => {
                Self::show_properties(ui, label, props);
            }
            PropertyValue::CustomStructProperty(custom_struct) => {
                egui::CollapsingHeader::new(label)
                    .default_open(true)
                    .show(ui, |ui| {
                        Self::typed_input(ui, "Flags", &mut custom_struct.flags);
                        Self::show_properties(ui, "Properties", &mut custom_struct.properties);
                        Self::show_binary_data(ui, "Extra", &custom_struct.extra);
                    });
            }
            PropertyValue::CoreUObjectStructProperty(object) => {
                egui::CollapsingHeader::new(label)
                    .default_open(true)
                    .show(ui, |ui| {
                        for (name, field) in object.fields_mut() {
                            Self::typed_input(ui, name, field);
                        }
                    });
            }
            PropertyValue::ArrayProperty { values } => {
                let num_values = values.len();
                if num_values == 1 && let Some(PropertyValue::UnknownProperty(data)) = values.first() {
                    Self::show_binary_data(ui, label, data);
                    return;
                }

                let element_type = property_type.element_type();
                egui::CollapsingHeader::new(format!("{label} ({num_values})"))
                    .show(ui, |ui| {
                        let mut action = ListAction::None;
                        for (i, value) in values.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                action.update(Self::show_list_context_menu(ui, i));
                                Self::show_property_value(ui, &i.to_string(), value, None, &element_type);
                            });
                        }

                        let flags = match flags {
                            Some(flags) => *flags,
                            None => 0,
                        };

                        match action {
                            ListAction::Insert(index) => {
                                values.insert(index, element_type.make_default_value(flags));
                            }
                            ListAction::Delete(index) => {
                                values.remove(index);
                            }
                            ListAction::None => (),
                        }

                        if values.is_empty() && ui.button("Insert").clicked() {
                            values.push(element_type.make_default_value(flags));
                        }
                    });
            }
            PropertyValue::MapProperty { removed_count, values } => {
                let num_values = values.len();
                egui::CollapsingHeader::new(format!("{label} ({num_values})"))
                    .show(ui, |ui| {
                        Self::typed_input(ui, "Removed", removed_count);

                        let mut action = ListAction::None;
                        let key_type = property_type.element_type();
                        let Some(value_type) = property_type.inner_types.last() else { return; };
                        for (i, value) in values.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                action.update(Self::show_list_context_menu(ui, i));
                                egui::CollapsingHeader::new(i.to_string())
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        Self::show_property_value(ui, "Key", &mut value.0, None, &key_type);
                                        Self::show_property_value(ui, "Value", &mut value.1, None, &value_type);
                                    });
                            });
                        }

                        let flags = match flags {
                            Some(flags) => *flags,
                            None => 0,
                        };

                        match action {
                            ListAction::Insert(index) => {
                                let key = key_type.make_default_value(flags);
                                let value = value_type.make_default_value(flags);
                                values.insert(index, (key, value));
                            }
                            ListAction::Delete(index) => {
                                values.remove(index);
                            }
                            ListAction::None => (),
                        }

                        if values.is_empty() && ui.button("Insert").clicked() {
                            let key = key_type.make_default_value(flags);
                            let value = value_type.make_default_value(flags);
                            values.push((key, value));
                        }
                    });
            }
            PropertyValue::UnknownProperty(data) => {
                Self::show_binary_data(ui, label, data);
            }
        }
    }

    fn show_property(ui: &mut egui::Ui, property: &mut Property) {
        Self::text_input(ui, "Name", &mut property.name);

        let Some(property) = &mut property.body else {
            return;
        };

        egui::CollapsingHeader::new(format!("Type: {}", property.property_type.describe()))
            .show(ui, |ui| {
                Self::show_type(ui, &mut property.property_type);
            });
        Self::typed_input(ui, "Flags", &mut property.flags);

        Self::show_property_value(ui, "Value", &mut property.value, Some(&mut property.flags), &property.property_type);
    }

    fn show_properties(ui: &mut egui::Ui, label: &str, properties: &mut Vec<Property>) {
        let num_properties = properties.len();
        egui::CollapsingHeader::new(format!("{label} ({num_properties})"))
            .show(ui, |ui| {
                let mut delete_index = None;
                for (i, property) in properties.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!property.is_none(), egui::Button::new("🗑")).clicked() {
                            delete_index = Some(i);
                        }
                        egui::CollapsingHeader::new(format!("{}: {}", i, property.name))
                            .show(ui, |ui| {
                                Self::show_property(ui, property);
                            });
                    });
                }

                if let Some(index) = delete_index {
                    properties.remove(index);
                }
            });
    }

    fn show_save_game(&mut self, ui: &mut egui::Ui) {
        let Some(save) = &mut self.save else {
            return;
        };

        Self::text_input(ui, "Type", &mut save.save_data.type_name);
        Self::typed_input(ui, "Flags", &mut save.save_data.flags);
        Self::show_properties(ui, "Properties", &mut save.save_data.properties);
        Self::typed_input(ui, "Extra", &mut save.save_data.extra);
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open .sav...").clicked() {
                        ui.close();
                        self.open_save();
                    }
                    let can_save = self.save.is_some();
                    if ui.add_enabled(can_save, egui::Button::new("Save as..."))
                        .clicked()
                    {
                        ui.close();
                        self.save_as();
                    }
                    if ui.button("Exit").clicked() {
                        ui.close();
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                });
            });
        });

        // Optional left tree panel when a file is loaded
        if self.save.is_some() {
            egui::CentralPanel::default()
                .show(ctx, |ui| {
                    // Placeholder tree view using collapsing headers
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            egui::CollapsingHeader::new("Header")
                                .show(ui, |ui| self.show_header(ui));

                            egui::CollapsingHeader::new("Custom Format")
                                .show(ui, |ui| self.show_custom_format(ui));

                            egui::CollapsingHeader::new("Save Game")
                                .show(ui, |ui| self.show_save_game(ui));
                        });
                });
        } else {
            // Main content
            egui::CentralPanel::default().show(ctx, |ui| {
                match &self.save_path {
                    None => {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.heading("Silent Hill f Save Editor");
                            ui.label("Open a .sav file to begin.");
                            ui.add_space(10.0);
                            if ui.button("Open .sav...").clicked() {
                                self.open_save();
                            }
                        });
                    }
                    Some(path) => {
                        // Right side: file info / placeholder content panel
                        ui.vertical_centered(|ui| {
                            ui.heading("File Loaded");
                            ui.monospace(path.display().to_string());
                            ui.label("Tree contents to be implemented.");
                        });
                    }
                }
            });
        }

        self.error_modal(ctx);
    }
}
