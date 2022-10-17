use std::collections::BTreeMap;


#[derive(serde::Deserialize, serde::Serialize)]
pub struct MechdataCombined {
    pub mech_variants: Vec<Variant>,
    pub equipment: Equipment,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Equipment {
    pub weapons: Vec<Weapon>,
    pub engines: Vec<Engine>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Weapon {
    pub name: String,
    pub hardpoint_kind: HardpointKind,
    pub hardpoint_aliases: Vec<String>,
    pub faction: Affiliation,
    pub slots: i32,
    pub tons: f32,
    pub id: i32,
    pub cooldown: f32,
    pub speed: i32,
    pub ammo_type: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug)]
pub struct Engine {
    pub id: i32,
    pub name: String,
    pub rating: i32,
    pub heatsinks: i32,
    pub weight: f32,
    pub side_slots: i32,
    pub factions: Vec<Affiliation>,
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
#[derive(
    Clone, Copy, Debug, PartialOrd, Ord, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
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
    #[allow(dead_code)]
    pub fn from_int(x: i32) -> Self {
        match x {
            0 => HardpointKind::Ballistic,
            1 => HardpointKind::Energy,
            2 => HardpointKind::Missile,
            4 => HardpointKind::AMS,
            _ => panic!("not a hardpoint kind {:?}", x),
        }
    }
}
impl std::str::FromStr for HardpointKind {
    type Err = String; // using Result<_, String> is bad, but eh
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Ballistic" => HardpointKind::Ballistic,
            "Energy" => HardpointKind::Energy,
            "Missile" => HardpointKind::Missile,
            "AMS" => HardpointKind::AMS,
            _ => return Err(format!("Unknown hardpoint kind {:?}", s)),
        })
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
    pub hardpoint_count: BTreeMap<HardpointKind, i32>,
    pub can_equip_ecm: bool,
    pub has_doors: bool,
}
