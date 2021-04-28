mod mwo_types;
mod pak_archive;

use itertools::Itertools;
use std::io::Read;

use mwo_types::{Affiliation, Component, Specialness, Variant, Weapon};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
type MyMap<K, V> = std::collections::BTreeMap<K, V>;

fn main() {
    let game_path = Path::new(&std::env::args().nth(1).expect("pls supply path/to/MechWarrior Online/ ")).join("Game");

    let weapons = parse_weapons(&game_path);
    let internals = parse_internals(&game_path);
    let variants = parse_mechs(&game_path, &internals);

    let combined = crate::mwo_types::MechdataCombined2 {
        mech_variants: variants,
        weapons: weapons.clone(),
    };
    let to_write = vec![
        (
            "static/mechdata_combined.min.json",
            serde_json::to_string(&combined).unwrap(),
        ),
        (
            "static/mechdata_combined.pretty.json",
            serde_json::to_string_pretty(&combined).unwrap(),
        ),
    ];
    for (file, content) in to_write {
        std::io::Write::write_all(
            &mut std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(file)
                .unwrap(),
            content.as_bytes(),
        )
        .unwrap();
    }
}

pub(crate) fn parse_weapons(game_path: impl AsRef<Path>) -> Vec<Weapon> {
    let arch_path = game_path.as_ref().join(r"GameData.pak");
    let mut archive =
        pak_archive::PakArchive::new(std::fs::File::open(arch_path).unwrap()).unwrap();
    let wep_contents =
        String::from_utf8(archive.unpack(r"Libs/Items/Weapons/Weapons.xml")).unwrap();
    let doc = roxmltree::Document::parse(&wep_contents).unwrap();

    let weap_list_elem = doc.root().children().next().unwrap();
    assert_eq!(weap_list_elem.tag_name().name(), "WeaponList");

    let mut weapons = vec![];
    for w in weap_list_elem.children() {
        if !w.is_element() {
            continue;
        }
        // println!("{:?}", w);
        assert_eq!(w.tag_name().name(), "Weapon");

        let stats = if let Some(f) = w.attribute("InheritFrom") {
            // println!("inheriting");
            weap_list_elem
                .children()
                .find(|w| w.attribute("id") == Some(f))
                .unwrap()
        } else {
            w
        }
        .children()
        .find(|c| c.tag_name().name() == "WeaponStats")
        .unwrap();

        let weap = Weapon {
            id: w.attribute("id").unwrap().parse().unwrap(),
            name: w.attribute("name").unwrap().into(),
            hardpoint_aliases: w
                .attribute("HardpointAliases")
                .unwrap()
                .split(",")
                .map(|x| x.to_string())
                .collect(),
            faction: match w.attribute("faction").unwrap() {
                "Clan" => Affiliation::Clan,
                "InnerSphere" => Affiliation::InnerSphere,
                other => panic!("{:?}", other),
            },
            slots: stats.attribute("slots").unwrap().parse().unwrap(),
            tons: stats.attribute("tons").unwrap().parse().unwrap(),
            cooldown: stats.attribute("cooldown").unwrap().parse().unwrap(),
            speed: stats.attribute("speed").unwrap().parse().unwrap(),
        };
        if ["DropShipLargePulseLaser", "FakeMachineGun"].contains(&weap.name.as_str()) {
            continue;
        }
        weapons.push(weap);
    }

    weapons
}

struct Internal {
    id: i32,
    slots: i32,
}

fn parse_internals(game_path: impl AsRef<Path>) -> Vec<Internal> {
    let arch_path = game_path.as_ref().join(r"GameData.pak");
    let mut archive =
        pak_archive::PakArchive::new(std::fs::File::open(arch_path).unwrap()).unwrap();
    let int_contents =
        String::from_utf8(archive.unpack(r"Libs/Items/Modules/Internals.xml")).unwrap();
    let doc = roxmltree::Document::parse(&int_contents).unwrap();

    let int_list_elem = doc.root().children().next().unwrap();
    assert_eq!(int_list_elem.tag_name().name(), "ModuleList");

    let mut internals = vec![];
    for int in int_list_elem.children().filter(|x| x.is_element()) {
        assert_eq!(int.tag_name().name(), "Internal");
        let module_stats = int
            .children()
            .filter(|x| x.is_element() && x.tag_name().name() == "ModuleStats")
            .exactly_one()
            .unwrap();
        internals.push(Internal {
            id: int.attribute("id").unwrap().parse().unwrap(),
            slots: module_stats.attribute("slots").unwrap().parse().unwrap(),
        })
    }

    internals
}

struct MechListElement {
    // id: i32,
    faction: Affiliation,
    chassis: String,
    variant: String,
}

fn parse_mechs(game_path: impl AsRef<Path>, internals: &[Internal]) -> Vec<Variant> {
    let game_path = game_path.as_ref();
    let gamedata_pak_path = game_path.join(r"GameData.pak");
    let mut archive =
        pak_archive::PakArchive::new(std::fs::File::open(gamedata_pak_path).unwrap()).unwrap();
    let mech_list_contents =
        String::from_utf8(archive.unpack(r"Libs/Items/Mechs/Mechs.xml")).unwrap();

    let mut mech_list = vec![];
    let doc = roxmltree::Document::parse(&mech_list_contents).unwrap();
    let root = doc.root();

    let ml: roxmltree::Node = root
        .children()
        .filter(|x| x.is_element())
        .exactly_one()
        .unwrap();
    for mech in ml.children().filter(|x| x.is_element()) {
        assert_eq!(mech.tag_name().name(), "Mech");
        mech_list.push(MechListElement {
            // id: mech.attribute("id").unwrap().parse().unwrap(),
            faction: match mech.attribute("faction").unwrap() {
                "InnerSphere" => Affiliation::InnerSphere,
                "Clan" => Affiliation::Clan,
                _ => panic!(),
            },
            chassis: mech.attribute("chassis").unwrap().parse().unwrap(),
            variant: mech.attribute("name").unwrap().parse().unwrap(),
        })
    }
    let chassis_set = mech_list
        .iter()
        .map(|x| x.chassis.to_string())
        .collect::<std::collections::BTreeSet<_>>();
    let mut variants = vec![];
    for chassis in chassis_set {
        // println!("{:?}", chassis);
        variants.extend(parse_mech_chassis(
            game_path, &chassis, internals, &mech_list,
        ));
    }

    {
        // sanity check
        let vars_declared: BTreeSet<_> = mech_list
            .iter()
            .map(|x| x.variant.to_string().to_lowercase())
            .collect();
        let vars_found: BTreeSet<_> = variants
            .iter()
            .map(|x| x.variant_name.to_string().to_lowercase())
            .collect();
        assert_eq!(vars_found, vars_declared);
    }
    variants
}

fn parse_mech_chassis(
    game_path: impl AsRef<Path>,
    chassis: &str,
    internals: &[Internal],
    mechlist: &Vec<MechListElement>,
) -> Vec<Variant> {
    let game_path = game_path.as_ref();
    let pak_path = game_path.join(format!("mechs/{}.pak", chassis));
    let mut pak_contents = vec![];
    std::fs::File::open(pak_path)
        .unwrap()
        .read_to_end(&mut pak_contents)
        .unwrap();

    // let arch = rpak::PakArchive::from_bytes(&pak_contents).unwrap();
    // let files = pak_archive::unpak(std::io::Cursor::new(pak_contents)).unwrap();
    let mut archive = pak_archive::PakArchive::new(std::io::Cursor::new(pak_contents)).unwrap();
    let mut hardpoints_xml = None;
    let mut variants_xmls = vec![];
    for filename in archive.file_list() {
        if filename.ends_with("-hardpoints.xml") {
            hardpoints_xml = Some(String::from_utf8(archive.unpack(&filename)).unwrap());
            continue;
        }
        if filename.ends_with(".mdf") {
            variants_xmls.push((
                filename.to_string(),
                String::from_utf8(archive.unpack(&filename)).unwrap(),
            ));
        }
    }
    let hardpoints_xml = hardpoints_xml.unwrap();

    let hardpoints = parse_hardpoints_def(&hardpoints_xml);
    // println!("{:?}", hardpoints);
    let mut variants = vec![];
    for (filename, variant) in variants_xmls {
        // println!("hi - {:?}", filename);
        let variant_name = Path::new(&filename)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .split(".")
            .next()
            .unwrap();
        let var = parse_mech_variant(variant_name, &variant, &hardpoints, internals, mechlist);
        // println!("{:?}", var);
        variants.push(var)
    }
    variants
}

#[derive(Debug)]
struct HardpointDefs {
    // hardpoint id -> slot count
    slot_count: MyMap<i32, i32>,
}

fn parse_hardpoints_def(contents: &str) -> HardpointDefs {
    let doc = roxmltree::Document::parse(contents).unwrap();
    let root = doc.root();
    let hardpoints_elem: roxmltree::Node = root
        .children()
        .filter(|x| x.is_element())
        .exactly_one()
        .unwrap();
    assert_eq!(hardpoints_elem.tag_name().name(), "Hardpoints");

    let mut result: MyMap<i32, i32> = Default::default();
    for hp in hardpoints_elem.children().filter(|x| x.is_element()) {
        match hp.tag_name().name() {
            "Hardpoint" => {}
            "WeaponDoorSet" => {
                // TODO: idk
                continue;
            }
            _ => panic!(),
        }
        assert_eq!(hp.tag_name().name(), "Hardpoint");
        let id: i32 = hp.attribute("id").unwrap().parse().unwrap();
        let mut amt = 0;
        for slot in hp.children().filter(|x| x.is_element()) {
            assert_eq!(slot.tag_name().name(), "WeaponSlot");
            amt += 1
        }
        result.insert(id, amt);
    }

    HardpointDefs { slot_count: result }
}

fn parse_mech_variant(
    variant_name: &str,
    variant_content: &str,
    hardpoint_defs: &HardpointDefs,
    internals: &[Internal],
    mechlist: &[MechListElement],
) -> Variant {
    let doc = roxmltree::Document::parse(variant_content).unwrap();
    let root = doc.root();
    let mech_def_elem: roxmltree::Node = root
        .children()
        .filter(|x| x.is_element())
        .exactly_one()
        .unwrap();
    assert_eq!(mech_def_elem.tag_name().name(), "MechDefinition");

    struct VariantData {
        display_name: String,
        max_tons: i32,
        base_tons: f32,
        max_jj: i32,
        // ecm: bool,
        engine_min: i32,
        engine_max: i32,
        specialness: Specialness,
    }

    let melem = mech_def_elem
        .children()
        .filter(|x| x.is_element() && x.tag_name().name() == "Mech")
        .exactly_one()
        .unwrap();
    let variant_data = VariantData {
        display_name: melem.attribute("Variant").expect("no variant").to_string(),
        max_tons: melem.attribute("MaxTons").unwrap().parse().unwrap(),
        base_tons: melem.attribute("BaseTons").unwrap().parse().unwrap(),
        max_jj: melem.attribute("MaxJumpJets").unwrap().parse().unwrap(),
        engine_min: melem.attribute("MinEngineRating").unwrap().parse().unwrap(),
        engine_max: melem.attribute("MaxEngineRating").unwrap().parse().unwrap(),
        specialness: melem
            .attribute("VariantType")
            .map(|typ| match typ {
                "Champion" => Specialness::Champion,
                "Hero" => Specialness::Hero,
                "Founder" => Specialness::Special,
                "Special" => Specialness::Special,
                "Phoenix" => Specialness::Special,
                "Sarah" => Specialness::Special,
                x => panic!("VariantType {:?} is idk", x),
            })
            .unwrap_or(Specialness::Normal),
    };

    let mut components = MyMap::<String, Component>::new();
    let complist_elem: roxmltree::Node = mech_def_elem
        .children()
        .filter(|x| x.is_element() && x.tag_name().name() == "ComponentList")
        .exactly_one()
        .unwrap();
    for comp_elem in complist_elem.children().filter(|x| x.is_element()) {
        assert_eq!(comp_elem.tag_name().name(), "Component");
        let comp_name = comp_elem.attribute("Name").unwrap().to_string();
        let hardpoint_count: BTreeMap<_, _> = comp_elem
            .children()
            .filter(|x| x.tag_name().name() == "Hardpoint")
            .map(|hp| {
                (
                    hp.attribute("ID").unwrap().parse::<i32>().unwrap(),
                    hp.attribute("Type").unwrap().parse::<u8>().unwrap(),
                )
            })
            .map(|(id, typ)| (typ, hardpoint_defs.slot_count[&id]))
            .collect();
        let base_slots: i32 = comp_elem.attribute("Slots").unwrap().parse().unwrap();
        let internal_ids: Vec<i32> = comp_elem
            .children()
            .filter(|x| x.tag_name().name() == "Internal")
            .map(|internal| {
                internal
                    .attribute("ItemID")
                    .unwrap()
                    .parse::<i32>()
                    .unwrap()
            })
            .collect();
        let effective_slots: i32 = base_slots
            - internal_ids
                .iter()
                .map(|iid| internals.iter().find(|int| int.id == *iid).unwrap().slots)
                .sum::<i32>();
        let comp = Component {
            base_slots,
            effective_slots,
            hp: comp_elem.attribute("HP").unwrap().parse().unwrap(),
            internal_ids,
            hardpoint_count,
            can_equip_ecm: comp_elem
                .attribute("CanEquipECM")
                .map(|x| x.parse::<i32>().unwrap() != 0)
                .unwrap_or(false),
        };
        if comp_name.ends_with("_rear") {
            assert_eq!(comp.base_slots, 0);
            assert_eq!(comp.hp, 0);
            assert!(comp.internal_ids.is_empty());
            assert!(comp.hardpoint_count.is_empty());
            continue;
        }
        components.insert(comp_name, comp);
    }
    let mut quirk_list_tmp = mech_def_elem
        .children()
        .filter(|x| x.is_element() && x.tag_name().name() == "QuirkList");
    let (quirk_list_tmp1, quirk_list_tmp2) = (quirk_list_tmp.next(), quirk_list_tmp.next());
    assert_eq!(quirk_list_tmp2, None);
    let quirk_list: Vec<(String, f32)> = if let Some(ql) = quirk_list_tmp1 {
        ql.children()
            .filter(|x| x.is_element())
            .map(|q: roxmltree::Node| {
                assert_eq!(q.tag_name().name(), "Quirk");
                (
                    q.attribute("name").unwrap().to_string(),
                    q.attribute("value").unwrap().parse::<f32>().unwrap(),
                )
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    let mechlist_item = mechlist.iter().find(|x| x.variant == variant_name).unwrap();

    Variant {
        chassis: mechlist_item.chassis.clone(),
        variant_name: variant_name.to_string(),
        display_name: variant_data.display_name,
        specialness: variant_data.specialness,
        affiliation: mechlist_item.faction.clone(),
        base_tons: variant_data.base_tons,
        max_tons: variant_data.max_tons,
        engine_min: variant_data.engine_min,
        engine_max: variant_data.engine_max,
        max_jj: variant_data.max_jj,

        components,
        quirks: quirk_list,
    }
}
