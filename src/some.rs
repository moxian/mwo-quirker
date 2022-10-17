// mod parse_data;

#[allow(unused_imports)]
use log::{error, info, warn};
use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::mwo_types::{Affiliation, Engine, Equipment, Variant, Weapon};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum TriState {
    Maybe,
    Yes,
    No,
}
impl std::default::Default for TriState {
    fn default() -> Self {
        return TriState::Maybe;
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct FitStatus {
    pub fits: TriState,
}

#[derive(Debug)]
pub(crate) struct Loadout {
    weapons: Vec<(Weapon, i32)>,
    want_ecm: bool,
    min_speed: f32,
    min_heatsinks: i32,
    want_armor: String,
    ammo_t: f32,
}

fn speed_with_engine(eng: &Engine, mech: &Variant) -> f32 {
    (eng.rating as f32) / (mech.max_tons as f32) * 16.2
}

fn is_fits(eqdb: &Equipment, mech: &Variant, loadout: &Loadout) -> TriState {
    let (want_weap, want_amt) = match loadout.weapons.as_slice() {
        [(w, a)] => (w, *a),
        _ => panic!("can't deal with multiweapon thing"),
    };
    // == First check if we maybe definitely CAN'T fit the thing
    // Faction
    if mech.affiliation != want_weap.faction {
        return TriState::No;
    }

    // Tonnage
    {
        let mut mech_free_tons = mech.max_tons as f32;
        mech_free_tons -= mech.max_tons as f32 * 0.05; // 5% for endo-steel structure;

        mech_free_tons -= loadout.ammo_t;

        // Armor, choose lightest
        match loadout.want_armor.as_str() {
            "none" => {}
            "full" => {
                let full_armor_points = mech
                    .components
                    .iter()
                    .map(|(name, c)| match name.as_str() {
                        "head" => 18,
                        _ => c.hp * 2,
                    })
                    .sum::<i32>();
                let mut full_armor_weight = full_armor_points as f32 * 0.03125;
                match mech.affiliation {
                    Affiliation::InnerSphere => full_armor_weight /= 1.12,
                    Affiliation::Clan => full_armor_weight /= 1.2,
                };
                mech_free_tons -= full_armor_weight
            }
            _ => panic!("{:?}", loadout),
        }

        // get lightest engine
        let possible_engines = eqdb
            .engines
            .iter()
            .filter(|eng| eng.factions.contains(&mech.affiliation))
            .filter(|eng| eng.rating >= mech.engine_min && eng.rating <= mech.engine_max)
            .filter(|eng| speed_with_engine(eng, mech) >= loadout.min_speed)
            .collect::<Vec<_>>();
        let lightest_engine_w_min_hs = possible_engines.iter().min_by_key(|eng| {
            let free_heatsinks = eng.heatsinks.min(10);
            let need_extra_hs = (10 - free_heatsinks).max(0);
            ordered_float::OrderedFloat(eng.weight + need_extra_hs as f32)
        });
        let lightest_engine_w_min_hs = match lightest_engine_w_min_hs {
            Some(e) => e,
            None => return TriState::No,
        };
        mech_free_tons -= lightest_engine_w_min_hs.weight;
        // nonfree heatsinks here
        let need_extra_hs = loadout.min_heatsinks - lightest_engine_w_min_hs.heatsinks.min(10);
        mech_free_tons -= need_extra_hs as f32;

        let equip_tons = want_weap.tons * want_amt as f32;
        if mech_free_tons < equip_tons {
            return TriState::No;
        }
    }

    // Hardpoints
    {
        let mut amt = 0;
        for (_, comp) in &mech.components {
            let hps = *comp
                .hardpoint_count
                .get(&want_weap.hardpoint_kind)
                .unwrap_or(&0);
            let slots = comp.effective_slots;
            amt += std::cmp::min(hps, slots / want_weap.slots);
        }
        if amt < want_amt {
            return TriState::No;
        }
    }

    // Wrap up
    return TriState::Yes;
}

pub(crate) fn get_fitting_mechs(
    equipment: &Equipment,
    mech_variants: &[Variant],
    mech_map: &HashMap<String, Variant>,
    chosen_weapons: &[(impl AsRef<str>, i32, f32)],
    min_speed: f32,
    armor: &str,
) -> BTreeMap<String, FitStatus> {
    let chosen_weapon_name = chosen_weapons[0].0.as_ref();
    let chosen_weapon_amt = chosen_weapons[0].1;
    let chosen_weapon_ammo_t = chosen_weapons[0].2;
    let want_weap = equipment
        .weapons
        .iter()
        .find(|w| w.name == chosen_weapon_name)
        .unwrap();
    let mut mechs_can_mount: BTreeMap<String, FitStatus> = BTreeMap::new();

    let want_loadout = Loadout {
        weapons: vec![(want_weap.clone(), chosen_weapon_amt)],
        min_speed,
        want_ecm: false,
        min_heatsinks: 10,
        want_armor: armor.into(),
        ammo_t: chosen_weapon_ammo_t,
    };
    for m in mech_variants {
        let fits = is_fits(equipment, m, &want_loadout);
        mechs_can_mount.insert(m.variant_name.clone(), FitStatus { fits });
    }

    let mut result: BTreeMap<String, FitStatus> = Default::default();
    for (variant, fit_stat) in &mechs_can_mount {
        result.insert(variant.to_string(), fit_stat.clone());
    }

    result
}

pub(crate) fn get_relevant_quirks(
    equipment: &Equipment,
    mech: &Variant,
    chosen_weapons: &[(impl AsRef<str>, i32, f32)],
) -> BTreeMap<String, BTreeMap<String, f32>> {
    let mut result: BTreeMap<String, BTreeMap<String, f32>> = BTreeMap::new();
    for (weap_name, _, _) in chosen_weapons {
        let weap_name = weap_name.as_ref();
        let want_weap = equipment
            .weapons
            .iter()
            .find(|w| w.name == weap_name)
            .unwrap();
        for (quirk_name, quirk_value) in &mech.quirks {
            let mut is_good_quirk = false;
            let mut quirk_kind = "".to_string();
            let mut quirk_math = "";
            for hp_alias in &want_weap.hardpoint_aliases {
                let tmp = hp_alias.to_lowercase() + "_";
                for prefix in &[tmp.as_str(), "all_"] {
                    if quirk_name.starts_with(prefix) {
                        let tail = &quirk_name[prefix.len()..];
                        let t2 = tail.split("_").collect::<Vec<_>>();
                        match t2.as_slice() {
                            [kind, math] => {
                                is_good_quirk = true;
                                quirk_kind = kind.to_string();
                                quirk_math = math;
                            }
                            _ => {
                                panic!("Quirk {:?} is supposedly related to hardpoint alias {:?} but idk how", quirk_name, hp_alias)
                            }
                        };
                    }
                }
            }

            if quirk_kind == "cooldown" && want_weap.cooldown == 0.0 {
                is_good_quirk = false;
            } else if quirk_kind == "velocity" && want_weap.speed == 0 {
                is_good_quirk = false;
            }

            if !is_good_quirk {
                continue;
            }

            let mech_entry = result.entry(weap_name.to_string()).or_default();
            let value = if quirk_math == "multiplier" {
                (quirk_value * 100.0).round()
            } else {
                *quirk_value
            };
            let q_entry = mech_entry.entry(quirk_kind).or_default();
            *q_entry += value;
        }
    }
    result
}

// #[allow(unused_variables, unreachable_code)]
// pub(crate) fn stuffs(
//     equipment: &Equipment,
//     mech_variants: &[Variant],
//     mech_map: &HashMap<String, Variant>,
//     chosen_weapons: &[(impl AsRef<str>, i32, f32)],
//     min_speed: f32,
//     armor: &str,
// ) -> BTreeMap<String, (FitStatus, BTreeMap<String, f32>)> {
//     let generic_weapon_quirks = vec![
//         "all_cooldown_multiplier",
//         "all_heat_multiplier",
//         "all_range_multiplier",
//         "all_velocity_multiplier",
//     ];

//     let chosen_weapon_name = chosen_weapons[0].0.as_ref();
//     let chosen_weapon_amt = chosen_weapons[0].1;
//     let chosen_weapon_ammo_t = chosen_weapons[0].2;
//     let want_weap = equipment
//         .weapons
//         .iter()
//         .find(|w| w.name == chosen_weapon_name)
//         .unwrap();
//     let mut mechs_can_mount: BTreeMap<String, FitStatus> = BTreeMap::new();

//     let want_loadout = Loadout {
//         weapons: vec![(want_weap.clone(), chosen_weapon_amt)],
//         min_speed,
//         want_ecm: false,
//         min_heatsinks: 10,
//         want_armor: armor.into(),
//     };
//     for m in mech_variants {
//         let fits = is_fits(equipment, m, &want_loadout);
//         mechs_can_mount.insert(m.variant_name.clone(), FitStatus { fits });
//     }

//     let mut result: BTreeMap<String, (FitStatus, BTreeMap<String, f32>)> = Default::default();
//     for (variant, amt) in &mechs_can_mount {
//         result.insert(variant.to_string(), (amt.clone(), Default::default()));
//     }

//     for mech in mech_variants {
//         for (quirk_name, quirk_value) in &mech.quirks {
//             if !mechs_can_mount.contains_key(&mech.variant_name) {
//                 continue;
//             }

//             let mut is_good_quirk = false;
//             let mut quirk_kind = "".to_string();
//             let mut quirk_math = "";
//             for hp_alias in &want_weap.hardpoint_aliases {
//                 let tmp = hp_alias.to_lowercase() + "_";
//                 for prefix in &[tmp.as_str(), "all_"] {
//                     if quirk_name.starts_with(prefix) {
//                         let tail = &quirk_name[prefix.len()..];
//                         let t2 = tail.split("_").collect::<Vec<_>>();
//                         match t2.as_slice() {
//                             [kind, math] => {
//                                 is_good_quirk = true;
//                                 quirk_kind = kind.to_string();
//                                 quirk_math = math;
//                             }
//                             _ => {
//                                 panic!("Quirk {:?} is supposedly related to hardpoint alias {:?} but idk how", quirk_name, hp_alias)
//                             }
//                         };
//                     }
//                 }
//             }

//             if quirk_kind == "cooldown" && want_weap.cooldown == 0.0 {
//                 is_good_quirk = false;
//             } else if quirk_kind == "velocity" && want_weap.speed == 0 {
//                 is_good_quirk = false;
//             }

//             if !is_good_quirk {
//                 continue;
//             }

//             let mech_entry = result.entry(mech.variant_name.to_string()).or_default();
//             let value = if quirk_math == "multiplier" {
//                 (quirk_value * 100.0).round()
//             } else {
//                 *quirk_value
//             };
//             let q_entry = mech_entry.1.entry(quirk_kind).or_default();
//             *q_entry += value;
//         }
//     }

//     return result;
// }
