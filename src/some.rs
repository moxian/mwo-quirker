// mod parse_data;

#[allow(unused_imports)]
use log::{error, info, warn};
use std::collections::BTreeMap;
use std::collections::HashMap;
// urls: https://mech.nav-alpha.com/php/fetch_quirks.php

use crate::mwo_types::{HardpointKind, Variant, Weapon};

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

#[allow(unused_variables, unreachable_code)]
pub(crate) fn stuffs(
    weapons: &[Weapon],
    mech_variants: &[Variant],
    mech_map: &HashMap<String, Variant>,
    chosen_weapon: &str,
    chosen_weapon_amt: i32,
) -> BTreeMap<String, (FitStatus, BTreeMap<String, f32>)> {
    let generic_weapon_quirks = vec![
        "all_cooldown_multiplier",
        "all_heat_multiplier",
        "all_range_multiplier",
        "all_velocity_multiplier",
    ];

    let wanted_thing = chosen_weapon;
    let want_weap = weapons.iter().find(|w| w.name == wanted_thing).unwrap();
    let mut mechs_can_mount: BTreeMap<String, FitStatus> = BTreeMap::new();
    let hardpoint_kind = match want_weap.hardpoint_aliases[0].as_str() {
        "Energy" => HardpointKind::Energy,
        "Ballistic" => HardpointKind::Ballistic,
        "Missile" => HardpointKind::Missile,
        "AntiMissileSystem" => HardpointKind::AMS,
        other => panic!("unknown kind {:?}", other),
    };
    let equip_tons = want_weap.tons * chosen_weapon_amt as f32;

    for m in mech_variants {
        // == First check if we maybe definitely CAN'T fit the thing
        // Faction
        let mut faction_ok = true;
        if m.affiliation != want_weap.faction {
            faction_ok = false;
        }

        // Tonnage
        let mut tons_ok = true;
        let mut mech_free_tons = m.max_tons as f32;
        mech_free_tons -= m.max_tons as f32 * 0.05; // 5% for endo-steel structure;
        mech_free_tons -= 8.0 - 2.5; // urbie engine +8 heatsinks (todo: add real XL'est engine)

        if mech_free_tons < equip_tons {
            tons_ok = false;
        }

        // Hardpoints
        let mut hardpoints_ok = true;
        let mut amt = 0;
        for (_, comp) in &m.components {
            let hps = *comp
                .hardpoint_count
                .get(&(hardpoint_kind.to_int() as u8))
                .unwrap_or(&0);
            let slots = comp.effective_slots;
            amt += std::cmp::min(hps, slots / want_weap.slots);
        }
        if amt < chosen_weapon_amt {
            hardpoints_ok = false;
        }

        // Wrap up
        let status = if hardpoints_ok && tons_ok && faction_ok {
            TriState::Yes
        } else {
            TriState::No
        };
        mechs_can_mount.insert(m.variant_name.clone(), FitStatus { fits: status });
    }

    let mut result: BTreeMap<String, (FitStatus, BTreeMap<String, f32>)> = Default::default();
    for (variant, amt) in &mechs_can_mount {
        result.insert(variant.to_string(), (amt.clone(), Default::default()));
    }

    for mech in mech_variants {
        for (quirk_name, quirk_value) in &mech.quirks {
            if !mechs_can_mount.contains_key(&mech.variant_name) {
                continue;
            }

            let mut is_good_quirk = false;
            let mut quirk_kind = "".to_string();
            let mut quirk_math = "";
            for x in &want_weap.hardpoint_aliases {
                // println!("{}", x);
                if quirk_name.starts_with(&(x.to_lowercase() + "_")) {
                    let tail = &quirk_name[x.len() + 1..];
                    let t2 = tail.split("_").collect::<Vec<_>>();
                    match t2.as_slice() {
                        [kind, math] => {
                            // println!("{:?} {:?}", kind, math);
                            is_good_quirk = true;
                            quirk_kind = kind.to_string();
                            quirk_math = math;
                        }
                        _ => {
                            // println!("AAAAA {:?}", t2)
                        }
                    };
                }
            }
            if !is_good_quirk {
                if generic_weapon_quirks.contains(&quirk_name.as_str()) {
                    quirk_kind = match quirk_name.as_str() {
                        "all_cooldown_multiplier" => "cooldown",
                        "all_heat_multiplier" => "heat",
                        "all_range_multiplier" => "range",
                        "all_velocity_multiplier" => "velocity",
                        _ => unreachable!(),
                    }
                    .to_string();
                    quirk_math = "multiplier";
                    is_good_quirk = true;
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

            // if !all_gauss_quirks.contains(&q.quirk_name.as_str()) {
            //     continue;
            // }

            let mech_entry = result.entry(mech.variant_name.to_string()).or_default();
            let q_typ = match quirk_kind.as_str() {
                "" => if quirk_name.contains("jamchance") {
                    "jamchance"
                } else if quirk_name.contains("minheatpenaltylevel") {
                    "hsl"
                } else {
                    ""
                }
                .to_string(),
                z => z.to_string(),
            };
            let value = if quirk_math == "multiplier" {
                (quirk_value * 100.0).round()
            } else {
                *quirk_value
            };
            let q_entry = mech_entry.1.entry(q_typ).or_default();
            *q_entry += value;
        }
    }

    return result;

}
