use std::collections::BTreeMap;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Weapon {
    pub name: String,
    pub hardpoint_aliases: Vec<String>,
    pub faction: Affiliation,
    pub slots: i32,
    pub tons: f32,
    pub id: i32,
    pub cooldown: f32,
    pub speed: i32,
}

#[derive(
    serde::Serialize, serde::Deserialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum Affiliation {
    InnerSphere,
    Clan,
}
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructureType {
    Std,
    Endo,
}
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ArmorType {
    Std,
    Ferro,
    Stealth,
    LightFerro,
}
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HSType {
    Single,
    Double,
}
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum HardpointKind {
    Ballistic,
    Energy,
    Missile,
    AMS,
}
impl HardpointKind {
    #[allow(dead_code)]
    pub fn to_int(&self) -> i32 {
        match self {
            HardpointKind::Ballistic => 0,
            HardpointKind::Energy => 1,
            HardpointKind::Missile => 2,
            HardpointKind::AMS => 4,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Specialness {
    Normal,
    Champion,
    Hero,
    Special,
    Founder,
    Phoenix,
    Sarah,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Variant {
    pub chassis: String,
    pub variant_name: String,
    pub display_name: String,
    pub specialness: Specialness,
    pub affiliation: Affiliation,
    pub max_tons: i32,
    pub base_tons: f32,
    pub max_jj: i32,
    pub engine_min: i32,
    pub engine_max: i32,

    pub components: BTreeMap<String, Component>,
    pub quirks: Vec<(String, f32)>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Component {
    pub base_slots: i32,
    pub effective_slots: i32,
    pub hp: i32,
    pub internal_ids: Vec<i32>,
    // kind -> count
    pub hardpoint_count: BTreeMap<u8, i32>,
    pub can_equip_ecm: bool,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct MechdataCombined2 {
    pub weapons: Vec<Weapon>,
    pub mech_variants: Vec<Variant>,
}
