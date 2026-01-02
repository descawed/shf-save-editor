use std::fs::File;
use std::path::PathBuf;

use binrw::BinReaderExt;
use binrw::BinWriterExt;
use eframe::egui;
use egui::{KeyboardShortcut, Modifiers, Key, RichText, SliderClamping, ViewportCommand};

use crate::game::*;
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

#[derive(Default)]
pub struct AppState {
    save_path: Option<PathBuf>,
    save: Option<SaveGame>,
    error_message: Option<String>,
    tab: AppTab,
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

    fn save_to(&mut self, path: PathBuf) {
        let Some(ref save) = self.save else {
            self.save_path = Some(path);
            return;
        };

        let result: anyhow::Result<()> = (|| {
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
        }

        if let Some(path) = dialog.save_file() {
            self.save_to(path);
        }
    }

    fn typed_input<T: Stringable + ?Sized>(ui: &mut egui::Ui, label: &str, value: &mut T) {
        ui.horizontal(|ui| {
            if !label.is_empty() {
                ui.label(format!("{label}: "));
            }
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

    fn show_advanced_view(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Header")
            .show(ui, |ui| self.show_header(ui));

        egui::CollapsingHeader::new("Custom Format")
            .show(ui, |ui| self.show_custom_format(ui));

        egui::CollapsingHeader::new("Save Game")
            .show(ui, |ui| self.show_save_game(ui));
    }

    fn show_upgrade_level_selector(ui: &mut egui::Ui, player_stats: &mut impl Indexable, level_key: &str, buy_key: &str) {
        ui.horizontal(|ui| {
            let Some(current_level) = player_stats.get_key_mut(level_key) else {
                ui.colored_label(egui::Color32::RED, "Missing upgrade level");
                return;
            };
            ui.label("Upgrade level: ");

            let mut selected_level = None;
            for level in 0..=MAX_UPGRADE_LEVEL {
                if ui.selectable_label(*current_level == level, level.to_string()).clicked() {
                    selected_level = Some(level);
                }
            }

            let Some(selected_level) = selected_level else {
                return;
            };

            *current_level = PropertyValue::IntProperty(selected_level);
            if let Some(buy_level) = player_stats.get_key_mut(buy_key) {
                *buy_level = PropertyValue::IntProperty(selected_level);
            }
        });
    }

    fn show_stat_slider(ui: &mut egui::Ui, label: &str, stat_value: Option<&mut PropertyValue>) {
        let Some(PropertyValue::FloatProperty(stat_value)) = stat_value else {
            ui.colored_label(egui::Color32::RED, "Error: Missing or invalid stat value");
            return;
        };

        // never clamp so the user can play around with unusual values if they want to
        ui.add(egui::Slider::new(stat_value, 0.0..=1.0).text(label).clamping(SliderClamping::Never));
    }

    fn show_player_stats(ui: &mut egui::Ui, player_stats: &mut impl Indexable) {
        ui.heading("Health");
        Self::show_stat_slider(ui, "Ratio", player_stats.get_key_mut("HealthRatio"));
        Self::show_upgrade_level_selector(ui, player_stats, "MaxHealthLevel", "BuyHealthLevel");
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

    fn show_inventory(ui: &mut egui::Ui, inventory: &mut impl Indexable) {
        Self::show_weapons(ui, inventory, "Fog");
        ui.separator();
        Self::show_weapons(ui, inventory, "Dark");
        ui.separator();
        Self::show_consumables(ui, inventory);
    }

    fn show_simple_view(&mut self, ui: &mut egui::Ui) {
        let Some(save) = &mut self.save else {
            return;
        };

        let Some(player_state_record) = save.save_data.get_key_mut("PlayerStateRecord") else {
            ui.colored_label(egui::Color32::RED, "Error: missing PlayerStateRecord");
            return;
        };

        match player_state_record.get_key_mut("Data") {
            Some(data) => Self::show_player_stats(ui, data),
            None => {
                ui.colored_label(egui::Color32::RED, "Error: missing Data property in PlayerStateRecord");
            }
        }

        ui.separator();

        let Some(PropertyValue::ArrayProperty { values, .. }) = player_state_record.get_key_mut("ComponentRecords") else {
            ui.colored_label(egui::Color32::RED, "Error: missing or invalid ComponentRecords property in PlayerStateRecord");
            return;
        };

        for component_record in values {
            if let Some(class) = component_record.get_key_mut("Class") {
                if class == PLAYER_INVENTORY_COMPONENT_CLASS && let Some(data) = component_record.get_key_mut("Data") {
                    Self::show_inventory(ui, data);
                    return;
                }
            }
        }

        ui.colored_label(egui::Color32::RED, "Error: missing inventory component record");
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                        .add(egui::Button::new("Open .sav...").shortcut_text(ctx.format_shortcut(&open_shortcut)))
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

        if self.save.is_some() {
            egui::CentralPanel::default()
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for tab in AppTab::list() {
                                    if ui.selectable_label(self.tab == tab, tab.name()).clicked() {
                                        self.tab = tab;
                                    }
                                }
                            });

                            ui.separator();

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
}
