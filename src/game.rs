pub const MAX_UPGRADE_LEVEL: i32 = 6;

pub const PLAYER_INVENTORY_COMPONENT_CLASS: &str = "/Script/GameNoce.NocePlayerInventoryComponent";

pub trait Item: Sized {
    fn none() -> &'static Self;

    fn all() -> &'static [Self];

    fn id_index(&self) -> i32;

    fn name(&self) -> &'static str;
}

const fn get_item_from_id<T: Item>(id: i32, no_item: &'static T, items: &'static [T]) -> Option<&'static T> {
    if id == -1 {
        Some(no_item)
    } else if id < 0 {
        None
    } else {
        let id = id as usize;
        if id < items.len() {
            Some(&items[id])
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Weapon {
    pub id_index: i32,
    pub name: &'static str,
    pub max_durability: f32,
}

impl Weapon {
    pub const fn new(id_index: i32, name: &'static str, max_durability: f32) -> Self {
        Self { id_index, name, max_durability }
    }
}

impl Item for Weapon {
    fn none() -> &'static Self {
        &NO_WEAPON
    }

    fn all() -> &'static [Self] {
        &WEAPONS
    }

    fn id_index(&self) -> i32 {
        self.id_index
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

pub const DEFAULT_MAX_WEAPON_DURABILITY: f32 = 1000.0;
pub const NO_WEAPON: Weapon = Weapon::new(-1, "None", DEFAULT_MAX_WEAPON_DURABILITY);
pub const WEAPONS: [Weapon; 15] = [
    Weapon::new(0, "Steel Pipe", 450.0),
    Weapon::new(1, "Kitchen Knife", 240.0),
    Weapon::new(2, "Sickle", 360.0),
    Weapon::new(3, "Lantern", 1000.0),
    Weapon::new(4, "Fox Arm", 1000.0),
    Weapon::new(5, "Naginata", 1000.0),
    Weapon::new(6, "Axe", 360.0),
    Weapon::new(7, "Baseball Bat", 450.0),
    Weapon::new(8, "Sacred Sword", 540.0),
    Weapon::new(9, "Sacred Sword (purified)", 540.0),
    Weapon::new(10, "Crowbar", 540.0),
    Weapon::new(11, "Sledgehammer", 280.0),
    Weapon::new(12, "Kaiken", 1000.0),
    Weapon::new(13, "Steel Pipe (ending 1)", 1000.0),
    Weapon::new(14, "PP-8001", 800.0),
];

pub const fn get_weapon_from_id(id: i32) -> Option<&'static Weapon> {
    get_item_from_id(id, &NO_WEAPON, &WEAPONS)
}

pub const MIN_WEAPONS: usize = 3;
pub const MAX_WEAPONS: usize = 5;

#[derive(Debug, Clone)]
pub struct ConsumableItem {
    pub id_index: i32,
    pub name: &'static str,
    pub max_stack: i32,
}

impl ConsumableItem {
    pub const fn new(id_index: i32, name: &'static str, max_stack: i32) -> Self {
        Self { id_index, name, max_stack }
    }
}

impl Item for ConsumableItem {
    fn none() -> &'static Self {
        &NO_CONSUMABLE_ITEM
    }

    fn all() -> &'static [Self] {
        &CONSUMABLE_ITEMS
    }

    fn id_index(&self) -> i32 {
        self.id_index
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

pub const DEFAULT_MAX_CONSUMABLE_ITEM_STACK: i32 = 99;
pub const NO_CONSUMABLE_ITEM: ConsumableItem = ConsumableItem::new(-1, "None", DEFAULT_MAX_CONSUMABLE_ITEM_STACK);
pub const CONSUMABLE_ITEMS: [ConsumableItem; 16] = [
    ConsumableItem::new(0, "Red Capsules", 99),
    ConsumableItem::new(1, "Red Capsules (ending 1)", 99),
    ConsumableItem::new(2, "Bandage", 3),
    ConsumableItem::new(3, "First Aid Kit", 1),
    ConsumableItem::new(4, "Ramune", 3),
    ConsumableItem::new(5, "Divine Water", 3),
    ConsumableItem::new(6, "Kudzu Tea", 1),
    ConsumableItem::new(7, "Higashi", 3),
    ConsumableItem::new(8, "Inari Sushi", 1),
    ConsumableItem::new(9, "Chocolate", 3),
    ConsumableItem::new(10, "Yokan", 3),
    ConsumableItem::new(11, "Arare", 5),
    ConsumableItem::new(12, "Shriveled Abura-age", 1),
    ConsumableItem::new(13, "Dried Carcass", 1),
    ConsumableItem::new(14, "Antique Comb", 1),
    ConsumableItem::new(15, "Toolkit", 3),
];

pub const fn get_consumable_item_from_id(id: i32) -> Option<&'static ConsumableItem> {
    get_item_from_id(id, &NO_CONSUMABLE_ITEM, &CONSUMABLE_ITEMS)
}

pub const MIN_CONSUMABLE_ITEMS: usize = 8;
pub const MAX_CONSUMABLE_ITEMS: usize = 14;

pub const OMAMORI_NAMES: [&str; 41] = [
    "Weasel",
    "Serpent",
    "Cat",
    "Clam",
    "Hawk",
    "Owl",
    "Pine",
    "Bamboo",
    "Plum",
    "Kudzu Leaf",
    "Boar",
    "Rabbit",
    "Horse",
    "Crow",
    "Hound",
    "Swallow",
    "Bear",
    "Wolf",
    "Spider",
    "Swordfish",
    "Goat",
    "Turtle",
    "Elephant",
    "Beetle",
    "Butterfly",
    "Suzuran",
    "Dolphin",
    "Camel",
    "Bull",
    "Otter",
    "Willow",
    "Cuckoo",
    "Shrew Mole",
    "Tanuki",
    "Crab",
    "Sakura",
    "Whale",
    "Mantis",
    "Daisy",
    "Blessed Hand Mirror",
    "Peony",
];

pub const KEY_ITEM_NAMES: [&str; 88] = [
    "Capsule Case",
    "Hotei-sama Sitting Cross-legged",
    "Back Door Key",
    "Old Annex Main Gate Key",
    "Inari Sculpture",
    "Stone Orb",
    "Storage Room Key",
    "Stone Key",
    "Drawer Key",
    "The White Rabbit Emblem Key",
    "The Tri Scale Emblem Key",
    "Rightward-facing Fox Crest",
    "Leftward-facing Fox Crest",
    "Forward-facing Fox Crest",
    "Brainiac Hero Comic.",
    "Storehouse Key",
    "Bloody Key",
    "Black Bird and Sword Plate",
    "White Bird and Fox Plate",
    "Scales Plate",
    "Key to Sakuko's Mailbox",
    "Hairpin",
    "Key to the Back Mountains",
    "Picture Frame",
    "Key Cabinet Key",
    "Main Building 2F Stair Key",
    "Second Floor Classroom Generic Key",
    "Black Sparrow Crest",
    "White Sparrow Crest",
    "Fox Mask Crest",
    "Yellowed Calendar",
    "Block of Wax",
    "Fletching Key",
    "Pine Tree Emblem",
    "Propane Tube",
    "Desk Drawer Key",
    "Metal Gate Key",
    "Student's Prank Drawing",
    "Calendar",
    "Shoulder Bag",
    "Packing Tips",
    "Dirty Drawstring bag",
    "Embroidered Drawstring Bag",
    "Furoshiki",
    "School Bag",
    "Front Door Keys To Shu's House",
    "Faded Bride Doll",
    "Rusted Flask",
    "Broken Japanese Geta Sandal",
    "Cracked Hibachi Brazier",
    "Dad's Old Kitchen Knife",
    "Treasure Hunting Game Key",
    "Brooch",
    "Drawing of a Young Shimizu Hinako",
    "Raygun",
    "Balcony Key",
    "Talon Lampshade",
    "Restraint Lampshade",
    "A Yellowed Calendar",
    "Locker Room Key",
    "Combination Lock Dial",
    "Research Journal Photo 1",
    "Research Journal Photo 2",
    "Research Journal Photo 3",
    "Research Journal Photo 4",
    "Research Journal Photo 5",
    "Ema [1]",
    "Ema [2]",
    "Ema [3]",
    "Ema [4]",
    "Ema [5]",
    "Ema [6]",
    "Ema [7]",
    "Ema [8]",
    "Ema [9]",
    "Ema [10]",
    "Ema [11]",
    "Ema [12]",
    "Ema [13]",
    "Ema [14]",
    "Ema [15]",
    "Ema [16]",
    "Ema [17]",
    "Ema [18]",
    "Ema [19]",
    "Ema [20]",
    "Ema [21]",
    "Ema [22]",
];

pub const LETTER_NAMES: [&str; 229] = [
    "Note from Shu",
    "Strict Mother's Letter [1]",
    "Strict Mother's Letter [2]",
    "Strict Mother's Letter [3]",
    "Strict Mother's Letter [4]",
    "Strict Mother's Letter [5]",
    "Family Physician's Log [1]",
    "Family Physician's Log [2]",
    "Family Physician's Log [3]",
    "Family Physician's Log [4]",
    "Family Physician's Log [5]",
    "Housemaid's Note [1]",
    "Housemaid's Note [2]",
    "Housemaid's Note [3]",
    "Housemaid's Note [4]",
    "Bundle of Letters from the Drawer",
    "Fragrant Letter",
    "Letter from a Woman",
    "Letter with a Pressed Flower",
    "Letter from a Man",
    "Letter from a Boy",
    "Research Notes on \"Enmi-jugonroku\"",
    "Research Notes on \"Kakura-makakura\"",
    "Research Notes on \"Exotic Herbology\"",
    "Research Notes on Hakkokusou",
    "Clinical Trial [1]",
    "Clinical Trial [2]",
    "Clinical Trial [3]",
    "Prescription for My Partner",
    "Letter from Sakuko's Mother",
    "Mysterious Note",
    "Pages from an Observation Log [1]",
    "Letter from Rinko to Hinako [1]",
    "Letter from Rinko to Hinako [2]",
    "Sakuko's Parent-Teacher Journal",
    "Class Observation Log [1]",
    "Class Observation Log [2]",
    "Sakuko's Diary [1]",
    "Sakuko's Diary [2]",
    "Sakuko's Diary [3]",
    "Scrap of Paper [1]",
    "Scrap of Paper [2]",
    "Blood-Stained Letter",
    "Note for the Vice Principal",
    "Note to Teacher",
    "Student Note",
    "Note between Lovers [1]",
    "Note between Lovers [2]",
    "Note from a Male Student [1]",
    "Note from a Male Student [2]",
    "Cryptic Note",
    "Unopened Envelope [1]",
    "Unopened Envelope [2]",
    "Unopened Envelope [3]",
    "Unopened Envelope [4]",
    "Asakura's Textbook [1]",
    "Ancient Note",
    "Solemn Letter [1]",
    "Solemn Letter [2]",
    "Solemn Letter [3]",
    "Solemn Letter [4]",
    "Ornate Scroll [1]",
    "Ornate Scroll [2]",
    "Ornate Scroll [3]",
    "Ornate Scroll [4]",
    "Ornate Scroll [5]",
    "Ornate Scroll [6]",
    "Origami of Rumors [1]",
    "Origami of Rumors [2]",
    "Origami of Rumors [3]",
    "Origami of Grievances [1]",
    "A Farmer's Story",
    "Letter to Home",
    "Origami of Grievances [2]",
    "Origami of Grievances [3]",
    "Card from Rinko [1]",
    "Card from Rinko [2]",
    "Card from Rinko [3]",
    "Card from Rinko [4]",
    "Card from Rinko [5]",
    "Folded Letter [1]",
    "Folded Letter [2]",
    "Folded Letter [3]",
    "Folded Letter [4]",
    "Folded Letter [5]",
    "Flash Cards [1]",
    "Flash Cards [2]",
    "Flash Cards [3]",
    "Flash Cards [4]",
    "Flash Cards [5]",
    "Asakura's Textbook [2]",
    "Asakura's Textbook [3]",
    "Teacher Journal",
    "Pages from an Observation Log [2]",
    "Pages from an Observation Log [3]",
    "Letter of Concern",
    "Letter from Rinko to Hinako [3]",
    "Rinko's Childhood Diary [1]",
    "Rinko's Childhood Diary [2]",
    "Balled-Up Notebook Paper",
    "Fox Origami [1]",
    "Fox Origami [2]",
    "Fox Origami [3]",
    "Class Observation Log [3]",
    "Scrap of Paper [3]",
    "Sakuko's Diary [4]",
    "Sakuko's Diary [5]",
    "Sakuko's Diary [6]",
    "Sakuko's Diary [7]",
    "Sakuko's Diary [8]",
    "Rinko's Diary [1]",
    "Rinko's Diary [2]",
    "Rinko's Diary [3]",
    "Rinko's Diary [4]",
    "Rinko's Diary [5]",
    "Rinko's Secret Journal [1]",
    "Rinko's Secret Journal [2]",
    "Rinko's Secret Journal [3]",
    "Rinko's Secret Journal [4]",
    "Rinko's Secret Journal [5]",
    "A Perspective on Hierogamy [1]",
    "Scroll of Welcome [1]",
    "Scroll of Welcome [2]",
    "Scroll of Welcome [3]",
    "Scroll of Welcome [4]",
    "Scroll of Welcome [5]",
    "Scroll of Welcome [6]",
    "Scroll of Pity [1]",
    "Scroll of Pity [2]",
    "Scroll of Pity [3]",
    "Scroll of Pity [4]",
    "Scroll of Pity [5]",
    "Scroll of Pity [6]",
    "Scroll of Discipline [1]",
    "Scroll of Discipline [2]",
    "Scroll of Discipline [3]",
    "Scroll of Discipline [4]",
    "Scroll of Discipline [5]",
    "Scroll of Discipline [6]",
    "A Perspective on Hierogamy [2]",
    "A Perspective on Hierogamy [3]",
    "Diary of a Determined Boy",
    "Letter from a Determined Youth",
    "Diary of an Infatuated Boy",
    "Letter from an Infatuated Youth",
    "Strange Note [1]",
    "Strange Note [2]",
    "Strange Note [3]",
    "Strange Note [4]",
    "Strange Note [5]",
    "Strange Note [6]",
    "Regarding the \"Legend of the Sacred Sword\" [1]",
    "Regarding the \"Legend of the Sacred Sword\" [2]",
    "Regarding the \"Legend of the Sacred Sword\" [3]",
    "Regarding the \"Legend of the Sacred Sword\" [4]",
    "Regarding the \"Legend of the Sacred Sword\" [5]",
    "Regarding the \"Legend of the Sacred Sword\" [6]",
    "Letter to Kumiko",
    "A Boy's Diary",
    "Treasure Hunt Game Note [1]",
    "Treasure Hunt Game Note [2]",
    "Treasure Hunt Game Note [3]",
    "Treasure Hunt Game Note [4]",
    "Shu's Childhood Diary",
    "School Founder's Anecdote [1]",
    "School Founder's Anecdote [2]",
    "Letter to My Partner",
    "Movie Review of \"The Great Space Invasion\"",
    "Mandate of the Clan [1]",
    "Mandate of the Clan [2]",
    "Mandate of the Clan [3]",
    "Mandate of the Clan [4]",
    "Unfinished Letter",
    "Note of Caution",
    "Diary of Revenge [1]",
    "Diary of Revenge [2]",
    "Diary of Revenge [3]",
    "Diary of Revenge [4]",
    "Diary of Revenge [5]",
    "Diary of Revenge [6]",
    "School Paper Cutout",
    "Sennensugi Shrine Flyer",
    "Letter from the Hospital",
    "Local Doctor's Note [1]",
    "Local Doctor's Note [2]",
    "Local Doctor's Note [3]",
    "Local Doctor's Note [4]",
    "Local Doctor's Note [5]",
    "Local Doctor's Note [6]",
    "Local Doctor's Note [7]",
    "Autopsy Report: Undetermined Cause of Death",
    "Old Medical Record [1]",
    "Old Medical Record [2]",
    "Origami of Grievances [4]",
    "Hinako's Diary [1]",
    "Hinako's Diary [2]",
    "Hinako's Diary [3]",
    "Hinako's Diary [4]",
    "Errand Request",
    "Page from Mom's Diary",
    "Certificate of Full Payment",
    "Note from Hinako",
    "Excerpt from \"The Encyclopedia of Occult Phenomena\"",
    "Crushed Note",
    "Letter to Santa",
    "Washing Machine Flyer",
    "Letter from My Uncle",
    "Married Couple's Letter",
    "Project from Art Class",
    "Mom's Bag of Prescriptions",
    "Letter from Sakuko to Hinako",
    "Origami of Fox Prayers",
    "Letter from Kotoyuki",
    "Messy Scrawls",
    "A Warning",
    "Article: Serious Issues in Ebisugaoka [1]",
    "Article: Serious Issues in Ebisugaoka [2]",
    "Article: Child Drowns in Waterway [1]",
    "Article: Child Drowns in Waterway [2]",
    "Article: Child Drowns in Waterway [3]",
    "Article: Child Drowns in Waterway [4]",
    "Article: Child Drowns in Waterway [5]",
    "Crumpled Letter [1]",
    "Crumpled Letter [2]",
    "Sheet of Fine Paper [1]",
    "Sheet of Fine Paper [2]",
    "Tattered Paper [1]",
    "Tattered Paper [2]",
    "Tattered Paper [3]",
];