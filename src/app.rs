use std::collections::HashSet;
use std::fs::File;
use std::path::PathBuf;

use anyhow::Result;
use binrw::BinReaderExt;
use binrw::BinWriterExt;
use eframe::{egui, Storage};
use egui::{KeyboardShortcut, Modifiers, Key, RichText, SliderClamping, ViewportCommand};

use crate::game::*;
use crate::save::*;
use crate::uobject::Stringable;

const BINARY_DATA_CUTOFF: usize = 10;

const MIN_UI_SCALE: f32 = 0.5;
const MAX_UI_SCALE: f32 = 2.0;

const SETTINGS_KEY: &str = "shf_settings";

#[derive(serde::Serialize, serde::Deserialize)]
struct Settings {
    default_pixels_per_point: Option<f32>,
    ui_scale: f32,
    last_directory: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_pixels_per_point: None,
            ui_scale: 1.0,
            last_directory: None,
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppTab {
    Simple,
    Advanced,
}

impl AppTab {
    const fn list() -> [Self; 2] {
        [Self::Simple, Self::Advanced]
    }

    const fn name(&self) -> &'static str {
        match self {
            Self::Simple => "Simple",
            Self::Advanced => "Advanced",
        }
    }
}

impl Default for AppTab {
    fn default() -> Self {
        Self::Simple
    }
}

pub struct AppState {
    save_path: Option<PathBuf>,
    last_directory: Option<PathBuf>,
    save: Option<SaveGame>,
    error_message: Option<String>,
    tab: AppTab,
    default_pixels_per_point: Option<f32>,
    ui_scale: f32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            save_path: None,
            last_directory: Self::get_default_save_directory(),
            save: None,
            error_message: None,
            tab: AppTab::default(),
            default_pixels_per_point: None,
            ui_scale: 1.0,
        }
    }
}

impl AppState {
    pub fn load_app(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();

        if let Some(storage) = cc.storage {
            if let Some(settings) = eframe::get_value::<Settings>(storage, SETTINGS_KEY) {
                app.default_pixels_per_point = settings.default_pixels_per_point;
                app.ui_scale = settings.ui_scale;
                if let Some(last_directory) = settings.last_directory {
                    app.last_directory = Some(last_directory);
                }
            }
        }

        app
    }

    fn get_default_save_directory() -> Option<PathBuf> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")?;
        let mut path = PathBuf::from(local_app_data);
        path.push("SHf");
        path.push("Saved");
        path.push("SaveGames");

        if !path.exists() {
            return None;
        }

        let mut subdirectories = Vec::new();
        if let Ok(entries) = path.read_dir() {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        subdirectories.push(entry.path());
                    }
                }
            }
        }

        if subdirectories.len() == 1 {
            Some(subdirectories.remove(0))
        } else {
            Some(path)
        }
    }

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

    fn load_save(&mut self, save_path: PathBuf) -> Result<()> {
        let mut file = File::open(&save_path)?;
        let save: SaveGame = file.read_le()?;
        self.save_path = Some(save_path);
        self.save = Some(save);
        Ok(())
    }

    fn open_save(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Silent Hill f save", &["sav"]);

        if let Some(path) = &self.last_directory {
            dialog = dialog.set_directory(path);
        }

        if let Some(path) = dialog.pick_file() {
            if let Some(parent) = path.parent() {
                self.last_directory = Some(parent.to_path_buf());
            }

            if let Err(err) = self.load_save(path) {
                self.error_message = Some(format!("Failed to load save: {err}"));
            }
        }
    }

    fn save_to(&mut self, path: PathBuf) {
        let Some(ref save) = self.save else {
            self.save_path = Some(path);
            return;
        };

        let result: Result<()> = (|| {
            let mut file = File::create(&path)?;
            file.write_le(save)?;
            Ok(())
        })();

        if let Err(err) = result {
            self.error_message = Some(format!("Failed to save: {err}"));
        }

        self.save_path = Some(path);
    }

    fn save(&mut self) {
        let Some(save_path) = self.save_path.take() else { return; };
        self.save_to(save_path);
    }

    fn save_as(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Silent Hill f save", &["sav"]);

        if let Some(path) = &self.save_path {
            if let Some(parent) = path.parent() {
                dialog = dialog.set_directory(parent);
            }
        } else if let Some(path) = &self.last_directory {
            dialog = dialog.set_directory(path);
        }

        if let Some(path) = dialog.save_file() {
            if let Some(parent) = path.parent() {
                self.last_directory = Some(parent.to_path_buf());
            }

            self.save_to(path);
        }
    }

    fn typed_input<T: Stringable + ?Sized>(ui: &mut egui::Ui, label: &str, value: &mut T) -> bool {
        ui.horizontal(|ui| {
            if !label.is_empty() {
                ui.label(format!("{label}: "));
            }
            let mut string = value.to_string();
            if ui.text_edit_singleline(&mut string).changed() {
                value.try_set_from_str(&string);
                true
            } else {
                false
            }
        }).inner
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
                    .id_salt(label)
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

    fn show_type_menu(ui: &mut egui::Ui, selected_type: &mut Option<&'static str>) -> bool {
        let mut selected = false;
        for type_name in &SCALAR_TYPE_NAMES {
            if ui.button(*type_name).clicked() {
                *selected_type = Some(*type_name);
                ui.close();
                selected = true;
            }
        }

        selected
    }

    fn show_properties(ui: &mut egui::Ui, label: &str, properties: &mut Vec<Property>) {
        let num_properties = properties.len();
        egui::CollapsingHeader::new(format!("{label} ({num_properties})"))
            .id_salt(label)
            .show(ui, |ui| {
                let mut action = ListAction::None;
                let mut selected_type = None;
                for (i, property) in properties.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.menu_button("☰", |ui| {
                            ui.menu_button("Insert above", |ui| {
                                if Self::show_type_menu(ui, &mut selected_type) {
                                    action = ListAction::Insert(i);
                                }
                            });

                            ui.menu_button("Insert below", |ui| {
                                if Self::show_type_menu(ui, &mut selected_type) {
                                    action = ListAction::Insert(i + 1);
                                }
                            });

                            if action != ListAction::None && selected_type.is_some() {
                                ui.close();
                            }

                            ui.separator();

                            if ui.add_enabled(!property.is_none(), egui::Button::new("Delete")).clicked() {
                                action = ListAction::Delete(i);
                                ui.close();
                            }
                        });
                        egui::CollapsingHeader::new(format!("{}: {}", i, property.name))
                            .id_salt(i.to_string())
                            .show(ui, |ui| {
                                Self::show_property(ui, property);
                            });
                    });
                }

                match action {
                    ListAction::Insert(index) => {
                        let Some(selected_type) = selected_type else { return; };
                        properties.insert(index, Property::new_scalar(&format!("Field{index}"), PropertyValue::default_for_type(selected_type)));
                    }
                    ListAction::Delete(index) => {
                        properties.remove(index);
                    },
                    ListAction::None => (),
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

    fn show_advanced_view(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Header")
            .show(ui, |ui| self.show_header(ui));

        egui::CollapsingHeader::new("Custom Format")
            .show(ui, |ui| self.show_custom_format(ui));

        egui::CollapsingHeader::new("Save Game")
            .show(ui, |ui| self.show_save_game(ui));
    }

    fn show_upgrade_level_selector(ui: &mut egui::Ui, player_stats: &mut impl Indexable, level_key: &str, buy_key: &str) -> (bool, i32) {
        ui.horizontal(|ui| {
            let Some(current_level) = player_stats.get_key_mut(level_key) else {
                ui.colored_label(egui::Color32::RED, "Missing upgrade level");
                return (false, 0);
            };
            ui.label("Upgrade level: ");

            let mut selected_level = None;
            for level in 0..=MAX_UPGRADE_LEVEL {
                if ui.selectable_label(*current_level == level, level.to_string()).clicked() {
                    selected_level = Some(level);
                }
            }

            let Some(selected_level) = selected_level else {
                return (false, match current_level {
                    PropertyValue::IntProperty(level) => *level,
                    _ => 0,
                });
            };

            *current_level = PropertyValue::IntProperty(selected_level);
            if let Some(buy_level) = player_stats.get_key_mut(buy_key) {
                *buy_level = PropertyValue::IntProperty(selected_level);
            }

            (true, selected_level)
        }).inner
    }

    fn show_stat_slider(ui: &mut egui::Ui, label: &str, stat_value: Option<&mut PropertyValue>) -> (bool, f32) {
        let Some(PropertyValue::FloatProperty(stat_value)) = stat_value else {
            ui.colored_label(egui::Color32::RED, "Error: Missing or invalid stat value");
            return (false, 0.0);
        };

        // never clamp so the user can play around with unusual values if they want to
        let response = ui.add(egui::Slider::new(stat_value, 0.0..=1.0).text(label).clamping(SliderClamping::Never));
        (response.changed(), *stat_value)
    }

    fn show_player_stats(ui: &mut egui::Ui, player_stats: &mut impl Indexable, health: &mut f32) {
        ui.heading("Health");

        let health_changed = Self::typed_input(ui, "Current", health);
        let (ratio_changed, ratio) = Self::show_stat_slider(ui, "Ratio", player_stats.get_key_mut("HealthRatio"));
        let (upgrade_level_changed, upgrade_level) = Self::show_upgrade_level_selector(ui, player_stats, "MaxHealthLevel", "BuyHealthLevel");

        if health_changed && let Some(PropertyValue::FloatProperty(health_ratio)) = player_stats.get_key_mut("HealthRatio") {
            *health_ratio = *health / (BASE_HEALTH + upgrade_level as f32 * HEALTH_PER_UPGRADE);
        } else if ratio_changed || upgrade_level_changed {
            *health = ratio * (BASE_HEALTH + upgrade_level as f32 * HEALTH_PER_UPGRADE);
        }

        ui.separator();

        ui.heading("Stamina");
        Self::show_stat_slider(ui, "Ratio", player_stats.get_key_mut("StaminaRatio"));
        Self::show_upgrade_level_selector(ui, player_stats, "MaxStaminaLevel", "BuyStaminaLevel");
        ui.separator();

        ui.heading("Sanity");
        Self::show_stat_slider(ui, "Ratio", player_stats.get_key_mut("SanityRatio"));
        Self::show_stat_slider(ui, "Current Max Ratio", player_stats.get_key_mut("CurrentMaxSanityRatio"));
        Self::show_upgrade_level_selector(ui, player_stats, "MaxSanityLevel", "BuySanityLevel");
        ui.separator();

        let Some(PropertyValue::IntProperty(faith_value)) = player_stats.get_key_mut("FaithValue") else {
            ui.colored_label(egui::Color32::RED, "Error: Missing faith value");
            return;
        };
        ui.heading("Faith");
        Self::typed_input(ui, "", faith_value);
    }

    fn show_inventory_delete(ui: &mut egui::Ui, index: usize, min_index: usize, delete_index: &mut Option<usize>) {
        let can_delete = index >= min_index;
        if ui.add_enabled(can_delete, egui::Button::new("🗑")).clicked() {
            *delete_index = Some(index);
        }
    }

    fn show_item_dropdown<T: Item + 'static>(ui: &mut egui::Ui, salt: &str, id_index: &mut i32, item: Option<&T>) {
        let dropdown = egui::ComboBox::from_id_salt(salt);
        let dropdown = match item {
            Some(item) => dropdown.selected_text(item.name()),
            None => dropdown.selected_text(format!("Unknown {}", *id_index)),
        };
        dropdown.show_ui(ui, |ui| {
            let none = T::none();
            ui.selectable_value(id_index, none.id_index(), none.name());
            for item in T::all() {
                ui.selectable_value(id_index, item.id_index(), item.name());
            }
        });
    }

    fn show_weapons(ui: &mut egui::Ui, inventory: &mut impl Indexable, world: &str) {
        ui.heading(format!("{world} Weapons"));

        let equip_key = format!("{world}EquippedWeaponIndex");
        let mut equip_index = match inventory.get_key(&equip_key) {
            Some(PropertyValue::IntProperty(equip_index)) => *equip_index,
            _ => -1,
        };
        let mut set_equip_index = false;

        let Some(PropertyValue::ArrayProperty { values, .. }) = inventory.get_key_mut(&format!("{world}Weapons")) else {
            ui.colored_label(egui::Color32::RED, "Error: missing or invalid weapons");
            return;
        };

        let mut delete_index = None;
        for (i, inventory_weapon) in values.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                Self::show_inventory_delete(ui, i, MIN_WEAPONS, &mut delete_index);

                let max_durability = match inventory_weapon.get_key_mut("IDIndex") {
                    Some(PropertyValue::IntProperty(id_index)) => {
                        let weapon = get_weapon_from_id(*id_index);

                        ui.label("Weapon");
                        Self::show_item_dropdown(ui, &format!("{world} weapon {i}"), id_index, weapon);

                        // grab the weapon definition again in case it changed
                        match get_weapon_from_id(*id_index) {
                            Some(weapon) => weapon.max_durability,
                            None => DEFAULT_MAX_WEAPON_DURABILITY,
                        }
                    }
                    _ => {
                        ui.colored_label(egui::Color32::RED, "Error: missing or invalid weapon ID index");
                        DEFAULT_MAX_WEAPON_DURABILITY
                    }
                };

                match inventory_weapon.get_key_mut("Durability") {
                    Some(PropertyValue::FloatProperty(durability)) => {
                        ui.label("Durability");
                        ui.add(egui::Slider::new(durability, 0.0..=max_durability).clamping(SliderClamping::Never));
                    }
                    _ => {
                        ui.colored_label(egui::Color32::RED, "Error: missing or invalid durability");
                    }
                }

                let is_equipped = i as i32 == equip_index;
                if ui.radio(is_equipped, "Equipped").clicked() {
                    equip_index = i as i32;
                    set_equip_index = true;
                }
            });
        }

        if let Some(index) = delete_index {
            values.remove(index);
        }

        if values.len() < MAX_WEAPONS && ui.button("Add weapon").clicked() {
            values.push(
                PropertyValue::StructProperty(vec![
                    Property::new_scalar("Durability", PropertyValue::FloatProperty(0.0)),
                    Property::new_scalar("IDIndex", PropertyValue::IntProperty(NO_WEAPON.id_index)),
                    Property::new_none(),
                ])
            );
        }

        // I *think* the equipped and target indexes are always set to the same value in practice, but I don't know for sure,
        // so we shouldn't change things unless the user explicitly requested a change
        if set_equip_index {
            if let Some(PropertyValue::IntProperty(equip_index_value)) = inventory.get_key_mut(&equip_key) {
                *equip_index_value = equip_index;
            }
            if let Some(PropertyValue::IntProperty(target_index_value)) = inventory.get_key_mut(&format!("{world}TargetWeaponIndex")) {
                *target_index_value = equip_index;
            }
        }
    }

    fn show_consumables(ui: &mut egui::Ui, inventory: &mut impl Indexable) {
        ui.heading("Consumables");

        let Some(PropertyValue::ArrayProperty { values, .. }) = inventory.get_key_mut("Consumables") else {
            ui.colored_label(egui::Color32::RED, "Error: missing or invalid consumables");
            return;
        };

        let mut delete_index = None;
        for (i, inventory_consumable) in values.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                Self::show_inventory_delete(ui, i, MIN_CONSUMABLE_ITEMS, &mut delete_index);

                let max_quantity = match inventory_consumable.get_key_mut("IDIndex") {
                    Some(PropertyValue::IntProperty(id_index)) => {
                        let consumable = get_consumable_item_from_id(*id_index);

                        ui.label("Item");
                        Self::show_item_dropdown(ui, &format!("consumable {i}"), id_index, consumable);

                        match consumable {
                            Some(consumable) => consumable.max_stack,
                            None => DEFAULT_MAX_CONSUMABLE_ITEM_STACK,
                        }
                    }
                    _ => {
                        ui.colored_label(egui::Color32::RED, "Error: missing or invalid consumable ID index");
                        DEFAULT_MAX_CONSUMABLE_ITEM_STACK
                    }
                };

                match inventory_consumable.get_key_mut("Quantity") {
                    Some(PropertyValue::IntProperty(quantity)) => {
                        Self::typed_input(ui, "Quantity", quantity);
                        ui.label(format!(" / {max_quantity}"));
                    }
                    _ => {
                        ui.colored_label(egui::Color32::RED, "Error: missing or invalid quantity");
                    }
                }
            });
        }

        if let Some(index) = delete_index {
            values.remove(index);
        }

        if values.len() < MAX_CONSUMABLE_ITEMS && ui.button("Add consumable").clicked() {
            values.push(
                PropertyValue::StructProperty(vec![
                    Property::new_scalar("Quantity", PropertyValue::IntProperty(0)),
                    Property::new_scalar("IDIndex", PropertyValue::IntProperty(NO_CONSUMABLE_ITEM.id_index)),
                    Property::new_none(),
                ])
            );
        }
    }

    fn show_named_items(ui: &mut egui::Ui, item_flags: &mut [PropertyValue], item_names: &[&str]) {
        if ui.button("Add all").clicked() {
            for item_flag in item_flags.iter_mut() {
                *item_flag = PropertyValue::BoolProperty(Some(true));
            }
        }

        for (i, item_flag) in item_flags.iter_mut().enumerate() {
            let PropertyValue::BoolProperty(Some(item_flag)) = item_flag else {
                ui.colored_label(egui::Color32::RED, "Error: missing or invalid item flag");
                continue;
            };

            match item_names.get(i) {
                Some(item_name) => ui.checkbox(item_flag, *item_name),
                None => ui.checkbox(item_flag, format!("Unknown {i}")),
            };
        }
    }

    fn show_key_items(ui: &mut egui::Ui, inventory: &mut impl Indexable) {
        ui.heading("Key Items");

        let Some(PropertyValue::ArrayProperty { values, .. }) = inventory.get_key_mut("KeyItems") else {
            ui.colored_label(egui::Color32::RED, "Error: missing or invalid key items");
            return;
        };

        Self::show_named_items(ui, values, &KEY_ITEM_NAMES);
    }

    fn show_letters(ui: &mut egui::Ui, inventory: &mut impl Indexable) {
        ui.heading("Letters");

        let Some(PropertyValue::ArrayProperty { values, .. }) = inventory.get_key_mut("Letters") else {
            ui.colored_label(egui::Color32::RED, "Error: missing or invalid letters");
            return;
        };

        Self::show_named_items(ui, values, &LETTER_NAMES);
    }

    fn set_omamori_dropdown_text(dropdown: egui::ComboBox, id_index: i32) -> egui::ComboBox {
        match OMAMORI_NAMES.get(id_index as usize) {
            Some(name) => dropdown.selected_text(*name),
            None if id_index == -1 => dropdown.selected_text("None"),
            None => dropdown.selected_text(format!("Unknown {id_index}")),
        }
    }

    fn show_omamori(ui: &mut egui::Ui, inventory: &mut impl Indexable) {
        ui.heading("Omamori");

        ui.label("Obtained");
        let mut obtained = HashSet::new();
        match inventory.get_key_mut("Omamories") {
            Some(PropertyValue::ArrayProperty { values, .. }) => {
                for value in values.iter() {
                    if let Some(PropertyValue::IntProperty(id_index)) = value.get_key("IDIndex") && *id_index >= 0 {
                        obtained.insert(*id_index);
                    }
                }

                if ui.button("Add all").clicked() {
                    for i in 0..OMAMORI_NAMES.len() {
                        if !obtained.contains(&(i as i32)) {
                            values.push(
                                PropertyValue::StructProperty(vec![
                                    Property::new_scalar("IDIndex", PropertyValue::IntProperty(i as i32)),
                                    Property::new_none(),
                                ])
                            );
                        }
                    }
                }

                let mut delete_index = None;

                for (i, value) in values.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        Self::show_inventory_delete(ui, i, 0, &mut delete_index);

                        let Some(PropertyValue::IntProperty(id_index)) = value.get_key_mut("IDIndex") else {
                            ui.colored_label(egui::Color32::RED, "Error: missing or invalid omamori index");
                            return;
                        };

                        let dropdown = egui::ComboBox::from_id_salt(format!("obtained omamori {i}"));
                        let dropdown = Self::set_omamori_dropdown_text(dropdown, *id_index);
                        dropdown.show_ui(ui, |ui| {
                            ui.selectable_value(id_index, -1, "None");
                            for (i, name) in OMAMORI_NAMES.iter().enumerate() {
                                let i = i as i32;
                                // only show this omamori + ones that we don't already have
                                if i == *id_index || !obtained.contains(&i) {
                                    ui.selectable_value(id_index, i, *name);
                                }
                            }
                        });
                    });
                }

                if let Some(index) = delete_index {
                    values.remove(index);
                }

                if obtained.len() < OMAMORI_NAMES.len() && ui.button("Add omamori").clicked() {
                    values.push(
                        PropertyValue::StructProperty(vec![
                            Property::new_scalar("IDIndex", PropertyValue::IntProperty(-1)),
                            Property::new_none(),
                        ])
                    );
                }
            }
            _ => {
                ui.colored_label(egui::Color32::RED, "Error: missing or invalid omamori inventory");
            }
        }

        ui.label("Equipped");
        match inventory.get_key_mut("EquippedOmamories") {
            Some(PropertyValue::ArrayProperty { values, .. }) => {
                let mut delete_index = None;

                for (i, value) in values.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        Self::show_inventory_delete(ui, i, MIN_OMAMORI_SLOTS, &mut delete_index);

                        let PropertyValue::IntProperty(id_index) = value else {
                            ui.colored_label(egui::Color32::RED, "Error: missing or invalid omamori index");
                            return;
                        };

                        let dropdown = egui::ComboBox::from_id_salt(format!("equipped omamori {i}"));
                        let dropdown = Self::set_omamori_dropdown_text(dropdown, *id_index);
                        dropdown.show_ui(ui, |ui| {
                            ui.selectable_value(id_index, -1, "None");
                            for (i, name) in OMAMORI_NAMES.iter().enumerate() {
                                let i = i as i32;
                                // only show this omamori + ones that we have
                                if i == *id_index || obtained.contains(&i) {
                                    ui.selectable_value(id_index, i, *name);
                                }
                            }
                        });
                    });
                }

                if let Some(index) = delete_index {
                    values.remove(index);
                }

                if values.len() < MAX_OMAMORI_SLOTS && ui.button("Add omamori slot").clicked() {
                    values.push(PropertyValue::IntProperty(-1));
                }
            }
            _ => {
                ui.colored_label(egui::Color32::RED, "Error: missing or invalid equipped omamori inventory");
            }
        }
    }

    fn show_inventory(ui: &mut egui::Ui, inventory: &mut impl Indexable) {
        Self::show_weapons(ui, inventory, "Fog");
        ui.separator();
        Self::show_weapons(ui, inventory, "Dark");
        ui.separator();
        Self::show_consumables(ui, inventory);
        ui.separator();
        Self::show_omamori(ui, inventory);
        ui.separator();
        Self::show_key_items(ui, inventory);
        ui.separator();
        Self::show_letters(ui, inventory);
    }

    fn show_difficulty<T: DifficultyLevel>(ui: &mut egui::Ui, name: &str, save: &mut impl Indexable, property_name: &str) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label(name);

            let level_property = save.get_key_mut(property_name);
            let mut selected_level = match level_property {
                Some(PropertyValue::EnumProperty(level)) => {
                    let level = level.as_str();
                    match T::from_str(level) {
                        Ok(level) => level,
                        Err(_) => {
                            ui.colored_label(egui::Color32::RED, format!("Error: invalid difficulty level {level}"));
                            return Ok(());
                        }
                    }
                }
                Some(_) => {
                    ui.colored_label(egui::Color32::RED, "Error: invalid difficulty level type");
                    return Ok(());
                }
                _ => T::default(),
            };

            egui::ComboBox::from_id_salt(name)
                .selected_text(selected_level.name())
                .show_ui(ui, |ui| {
                    for level in T::all() {
                        ui.selectable_value(&mut selected_level, *level, level.name());
                    }
                });

            match level_property {
                Some(PropertyValue::EnumProperty(level)) => {
                    let level = level.as_mut();
                    level.clear();
                    level.push_str(selected_level.as_str());
                }
                // at this point we know level_property, if present, is a valid EnumProperty, otherwise
                // we would have bailed above
                Some(_) => unreachable!(),
                None if selected_level != T::default() => {
                    save.add_property(
                        Property::new_enum(property_name, T::namespace(), T::type_name(), selected_level.as_str())
                    )?;
                }
                // we don't need to do anything if the level is the default value
                None => (),
            }

            Ok(())
        }).inner
    }

    fn show_difficulties(ui: &mut egui::Ui, save: &mut impl Indexable) -> Result<()> {
        ui.heading("Difficulty");

        Self::show_difficulty::<ActionLevel>(ui, "Action", save, "ActionLevel")?;
        Self::show_difficulty::<RiddleLevel>(ui, "Puzzles", save, "RiddleLevel")
    }

    fn show_simple_view(&mut self, ui: &mut egui::Ui) {
        let Some(save) = &mut self.save else {
            return;
        };

        if save.save_data.type_name == SYSTEM_SAVE_TYPE {
            ui.label("The Simple view only supports gameplay saves, not system saves. Use the Advanced view to edit the system save.");
            return;
        }

        if save.save_data.type_name != SAVE_GAME_TYPE {
            let type_name = save.save_data.type_name.as_str();
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("Warning: unrecognized save type {type_name}. Some information may be missing or incorrect. It may be safer to use the Advanced view instead."),
            );
        }

        if let Err(e) = Self::show_difficulties(ui, &mut save.save_data) {
            self.error_message = Some(format!("Failed to set difficulty: {e}"));
        }
        ui.separator();

        let mut health = match prop!(&save.save_data, ["HinakoRecord"]["Health"]) {
            Some(PropertyValue::FloatProperty(health)) => *health,
            _ => 0.0,
        };

        let Some(player_state_record) = save.save_data.get_key_mut("PlayerStateRecord") else {
            ui.colored_label(egui::Color32::RED, "Error: missing PlayerStateRecord");
            return;
        };

        match player_state_record.get_key_mut("Data") {
            Some(data) => Self::show_player_stats(ui, data, &mut health),
            None => {
                ui.colored_label(egui::Color32::RED, "Error: missing Data property in PlayerStateRecord");
            }
        }

        ui.separator();

        let Some(PropertyValue::ArrayProperty { values, .. }) = player_state_record.get_key_mut("ComponentRecords") else {
            ui.colored_label(egui::Color32::RED, "Error: missing or invalid ComponentRecords property in PlayerStateRecord");
            return;
        };

        let mut found_inventory = false;
        for component_record in values {
            if let Some(class) = component_record.get_key_mut("Class") {
                if class == PLAYER_INVENTORY_COMPONENT_CLASS && let Some(data) = component_record.get_key_mut("Data") {
                    Self::show_inventory(ui, data);
                    found_inventory = true;
                    break;
                }
            }
        }

        if !found_inventory {
            ui.colored_label(egui::Color32::RED, "Error: missing inventory component record");
        }

        // if the health was updated above, save it
        if let Some(PropertyValue::FloatProperty(health_property)) = prop_mut!(&mut save.save_data, ["HinakoRecord"]["Health"]) {
            *health_property = health;
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.default_pixels_per_point.is_none() {
            self.default_pixels_per_point = Some(ctx.pixels_per_point());
        }

        let open_shortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
        if ctx.input_mut(|i| i.consume_shortcut(&open_shortcut)) {
            self.open_save();
        }

        let save_shortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
        if ctx.input_mut(|i| i.consume_shortcut(&save_shortcut)) && self.save.is_some() {
            self.save();
        }

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(egui::Button::new("Open...").shortcut_text(ctx.format_shortcut(&open_shortcut)))
                        .clicked()
                    {
                        ui.close();
                        self.open_save();
                    }

                    ui.separator();

                    let can_save = self.save.is_some();

                    if ui
                        .add_enabled(
                            can_save,
                            egui::Button::new("Save").shortcut_text(ctx.format_shortcut(&save_shortcut)),
                        )
                        .clicked()
                    {
                        ui.close();
                        self.save();
                    }

                    if ui.add_enabled(can_save, egui::Button::new("Save as..."))
                        .clicked()
                    {
                        ui.close();
                        self.save_as();
                    }

                    ui.separator();

                    if ui.button("Exit").clicked() {
                        ui.close();
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            if ui.add(egui::Slider::new(&mut self.ui_scale, MIN_UI_SCALE..=MAX_UI_SCALE).text("UI Scale")).changed() {
                ctx.set_pixels_per_point(self.ui_scale * self.default_pixels_per_point.unwrap());
            }
        });

        if self.save.is_some() {
            egui::CentralPanel::default()
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for tab in AppTab::list() {
                            if ui.selectable_label(self.tab == tab, tab.name()).clicked() {
                                self.tab = tab;
                            }
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            match self.tab {
                                AppTab::Simple => self.show_simple_view(ui),
                                AppTab::Advanced => self.show_advanced_view(ui),
                            }
                        });
                });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.heading("Silent Hill f Save Editor");
                    ui.label("Open a .sav file to begin.");
                    ui.add_space(10.0);
                    if ui
                        .add(egui::Button::new("Open .sav...").shortcut_text(ctx.format_shortcut(&open_shortcut)))
                        .clicked()
                    {
                        self.open_save();
                    }
                });
            });
        }

        self.error_modal(ctx);
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        let settings = Settings {
            default_pixels_per_point: self.default_pixels_per_point,
            ui_scale: self.ui_scale,
            last_directory: self.last_directory.clone(),
        };
        eframe::set_value(storage, SETTINGS_KEY, &settings);
    }
}
