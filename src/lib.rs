#![recursion_limit = "512"]
mod mwo_types;
mod some;

use mwo_types::Specialness;

use wasm_bindgen::prelude::*;
use yew::prelude::*;
#[allow(unused_imports)]
use yew::services::console::ConsoleService;

use anyhow::Error;
use std::collections::{BTreeMap, BTreeSet};

use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::{html, Component, ComponentLink, Html, ShouldRender};

type AsBinary = bool;

pub enum Format {
    Json,
    Toml,
}

pub enum WsAction {
    Connect,
    SendData(AsBinary),
    Disconnect,
    Lost,
}

pub enum MsgSettings {
    ToggleHeroes,
    ToggleSpecials,
    ToggleUnquirked,
    QuirkValue(String, i32),
}
pub enum MsgTweakLoadout {
    AddWeapon(String),
    RemoveWeapon(String),
    ChangeWeaponAmt(String, i32),
    ChangeWeaponAmmoAmt(String, f32),
}
pub enum Msg {
    FetchData,
    FetchReady(Result<mwo_types::MechdataCombined, Error>),
    Ignore,
    ToggleWeapons,
    // ChooseWeapon(String),
    // ChooseWeaponAmt(i32),
    ChooseMinSpeed(f32),
    ChooseArmor(String),
    TweakLoadout(MsgTweakLoadout),
    Settings(MsgSettings),
}

/// This type is used to parse data from `./static/data.json` file and
/// have to correspond the data layout from that file.
#[derive(serde::Deserialize, Debug)]
pub struct DataFromFile {
    value: u32,
}

#[derive(Debug)]
struct Settings {
    show_heroes: bool,
    show_specials: bool,
    show_unquirked: bool,
    min_quirks: BTreeMap<String, i32>,
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            show_heroes: true,
            show_specials: false,
            show_unquirked: true,

            min_quirks: Default::default(),
        }
    }
}

struct LoadoutSettings {
    chosen_weapons: Vec<(String, i32, f32)>,
    min_speed: f32,
    armor: String,
}
pub struct Model {
    link: ComponentLink<Model>,
    fetching: bool,
    data: Option<mwo_types::MechdataCombined>,
    ft: Option<FetchTask>,
    show_weap: bool,

    settings: Settings,
    loadout: LoadoutSettings,
}

impl Model {
    fn view_loadout2(&self, weapons: &[mwo_types::Weapon]) -> Html {
        let chosen_weapon_row = |weapon_name: &str, amt: i32, ammo_ton: f32| -> Html {
            let weapon_copy: String = weapon_name.to_string();
            let weapon_copy2: String = weapon_name.to_string();
            let weapon_copy3: String = weapon_name.to_string();
            let has_ammo = weapons
                .iter()
                .find(|w| w.name == weapon_name)
                .unwrap()
                .ammo_type
                .is_some();
            html! {<li>
                <button type="button" class="remove-wpn-button"
                    onclick=self.link.callback(move |_|{
                        Msg::TweakLoadout(MsgTweakLoadout::RemoveWeapon(
                            weapon_copy2.to_string()
                        ))
                    })>
                    {"X"}
                </button>
                { &weapon_name }
                <input type="number" class="smol-number" value={amt} min="0" max="20"
                    onchange=self.link.callback(move |change: yew::events::ChangeData|{
                        match change {
                            yew::events::ChangeData::Value(val) => {
                                Msg::TweakLoadout(MsgTweakLoadout::ChangeWeaponAmt(
                                    weapon_copy.to_string(),
                                    val.parse().unwrap()
                                ))
                            },
                            _ => Msg::Ignore,
                        }
                    })
                />
                { if has_ammo { html!{
                    <input type="number" class="smol-number" value={ammo_ton} min="0" max="100" step="0.5"
                        onchange=self.link.callback(move |change: yew::events::ChangeData|{
                            match change {
                                yew::events::ChangeData::Value(val) => {
                                    Msg::TweakLoadout(MsgTweakLoadout::ChangeWeaponAmmoAmt(
                                        weapon_copy3.to_string(),
                                        val.parse().unwrap()
                                    ))
                                },
                                _ => Msg::Ignore,
                            }
                        })
                    />
                }} else {html!{ }} }
            </li>}
        };

        html! {
            <div class="loadout-thing0">
            <table><tr>
                <td>
                    <ul class="weaponlist-chosen"
                        ondragover=self.link.callback(|dragover: yew::events::DragEvent|{
                            dragover.prevent_default();
                            Msg::Ignore
                        })
                        ondrop=self.link.callback(|drop: yew::events::DragEvent|{
                            drop.prevent_default(); // IMPORTANT!
                            let weap = drop.data_transfer().unwrap().get_data("text").unwrap();
                            Msg::TweakLoadout(MsgTweakLoadout::AddWeapon(weap.to_string()))
                        })
                    >
                        { self.loadout.chosen_weapons.iter().map(|(weapon, w_amt, ammo_ton)| {
                            chosen_weapon_row(weapon, *w_amt, *ammo_ton)
                        }).collect::<Html>() }
                    </ul>
                </td><td>
                    <ul class="weaponlist-available">
                        { weapons
                            .iter()
                            .filter(|weapon| !self.loadout.chosen_weapons.iter()
                                .any(|(chosen_weapon, _, _)| chosen_weapon == &weapon.name)
                            )
                            .map(|weapon: &mwo_types::Weapon| {
                                let name = weapon.name.to_string();
                                html!{
                                    <li draggable="true"
                                        ondragstart=self.link.callback(move |drag: yew::events::DragEvent|{
                                        drag.data_transfer().unwrap().set_data("text/plain", name.as_str()).unwrap();
                                        Msg::Ignore
                                    })>
                                    { weapon.name.as_str() }
                                    </li>
                                }
                            }).collect::<Html>()}
                    </ul>
                </td>
            </tr></table>
            </div>
        }
    }
    fn view_weapon_select(&self, weapons: &[mwo_types::Weapon]) -> Html {
        let mut weapons = weapons.to_vec();
        weapons.sort_by_key(|w| {
            (
                w.faction.to_owned(),
                w.hardpoint_aliases[0].to_owned(),
                w.name.to_owned(),
            )
        });
        html! {
            <div>
                <label>{"min speed"}</label>
                <input type="number" min="0" max="200" value={self.loadout.min_speed}
                    onchange=self.link.callback(|change: yew::events::ChangeData|{
                    match change {
                        yew::events::ChangeData::Value(val) => {
                            Msg::ChooseMinSpeed(val.parse().unwrap())
                        },
                        _ => Msg::Ignore,
                    }
                })/>
                <label>{"armor none"}</label>
                <input type="radio" checked={self.loadout.armor == "none"}
                    onchange=self.link.callback(|change: yew::events::ChangeData|{
                    match change {
                        yew::events::ChangeData::Value(val) => {
                            Msg::ChooseArmor("none".into())
                        },
                        _ => Msg::Ignore,
                    }
                })/>
                <label>{"armor full"}</label>
                <input type="radio" checked={self.loadout.armor == "full"}
                    onchange=self.link.callback(|change: yew::events::ChangeData|{
                    match change {
                        yew::events::ChangeData::Value(val) => {
                            Msg::ChooseArmor("full".into())
                        },
                        _ => Msg::Ignore,
                    }
                })/>
                { self.view_loadout2(&weapons) }
            </div>
        }
    }
    fn view_checkboxes(&self) -> Html {
        log::info!("settings: {:?}", self.settings);
        html! {
            <div>
                <input type="checkbox" id="show_heroes" checked={self.settings.show_heroes}
                    onclick=self.link.callback(|_| Msg::Settings(MsgSettings::ToggleHeroes))
                />
                <label for="show_heroes">{ "Show heroes" }</label>

                <input type="checkbox" id="show_specials" checked={self.settings.show_specials}
                    onclick=self.link.callback(|_| Msg::Settings(MsgSettings::ToggleSpecials))
                />
                <label for="show_specials">{ "Show specials" }</label>

                <input type="checkbox" id="show_unquirked" checked={self.settings.show_unquirked}
                    onclick=self.link.callback(|_| Msg::Settings(MsgSettings::ToggleUnquirked))
                />
                <label for="show_unquirked">{ "Show unquirked" }</label>
            </div>
        }
    }
    fn view_mech_list(
        &self,
        data: &mwo_types::MechdataCombined,
        _weapon: &str,
        amt: i32,
        ammo_ton: f32,
    ) -> Html {
        let mech_variants: Vec<_> = data
            .mech_variants
            .iter()
            .filter(|m| {
                use Specialness::*;
                match m.specialness {
                    Normal => true,
                    Hero => self.settings.show_heroes,
                    Special | Champion | Founder | Sarah | Phoenix => self.settings.show_specials,
                }
            })
            .cloned()
            .collect();
        let mech_map: std::collections::HashMap<String, mwo_types::Variant> = mech_variants
            .iter()
            .map(|m| (m.variant_name.clone(), m.clone()))
            .collect();
        let mechs_fitting = some::get_fitting_mechs(
            &data.equipment,
            &mech_variants,
            &mech_map,
            self.loadout.chosen_weapons.as_slice(),
            self.loadout.min_speed,
            &self.loadout.armor,
        );
        log::info!("{:?} mechs fit", mechs_fitting.len());
        let mut mechs_quirked = BTreeMap::<String, BTreeMap<String, BTreeMap<String, f32>>>::new();
        for mech in &mech_variants {
            let quirks_here = some::get_relevant_quirks(
                &data.equipment,
                mech,
                self.loadout.chosen_weapons.as_slice(),
            );
            mechs_quirked.insert(mech.variant_name.to_string(), quirks_here);
        }
        let mut some_mechs = mech_variants.clone();
        some_mechs.sort_by_key(|m| m.max_tons);
        ConsoleService::log("hello");
        let quirk_keys_present = mechs_quirked
            .values()
            .flat_map(|stuff| stuff.values())
            .flat_map(|stuff| stuff.keys())
            .collect::<BTreeSet<_>>();
        let quirk_renames = vec![("minheatpenaltylevel", "hsl")]
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let show_quirk_row = |mech: &mwo_types::Variant| -> Html {
            let empty_map = std::collections::BTreeMap::new();
            let can_mount = mechs_fitting.get(&mech.variant_name).unwrap();
            if can_mount.fits != some::TriState::Yes {
                log::info!("{:?} can't mount", mech.variant_name);
                return html! {};
            }

            let mut result_html_parts = vec![];
            let mut any_weapon_matches_quirk_filter = false;
            if self.settings.min_quirks.iter().all(|x| x.1 == &0){
                any_weapon_matches_quirk_filter = true;
            }
            let mech_row = match mechs_quirked.get(&mech.variant_name) {
                Some(r) => r,
                None => {
                    log::info!("Mech {:?} is not a quirked thing", mech.variant_name);
                    return html! {};
                }
            };
            for (weap_no, (weap_name, ..)) in self.loadout.chosen_weapons.iter().enumerate() {
                let weap_quirks = mech_row.get(weap_name).unwrap_or(&empty_map);
                let mut class = "".to_string();
                if can_mount.fits == some::TriState::No {
                    class += " cant-fit"
                }
                if weap_quirks.is_empty() {
                    class += " no-quirks"
                }
                // check quirk filters
                if self
                    .settings
                    .min_quirks
                    .iter()
                    .any(|(quirkname, min_value)| {
                        if !quirk_keys_present.contains(quirkname) {
                            return false;
                        };
                        if min_value == &0{
                            return false;
                        }
                        weap_quirks.get(quirkname).copied().unwrap_or(0.0).abs()
                            >= *min_value as f32
                    })
                {
                    any_weapon_matches_quirk_filter = true;
                }
                let subrows = self.loadout.chosen_weapons.len();

                // yes, this is a terrible copy-paste. But it appears to be inevitable...
                if weap_no == 0 {
                    result_html_parts.push(html! {
                        <tr class={class}>
                            <td rowspan={subrows}>{ mech.max_tons }</td>
                            <td rowspan={subrows}>{ &mech.chassis }</td> 
                            <td rowspan={subrows}>{ &mech.variant_name }</td> 
                            
                            <td>{ weap_name }</td>
                            {
                                quirk_keys_present.iter().map(|key: &&String| {
                                    let key: &str = key;
                                    html!{<td>
                                        { weap_quirks.get(key).map(|val|format!("{}", val)).unwrap_or("".to_string()) }
                                    </td>}
                                }).collect::<Html>()
                            }
                        </tr>
                    });
                } else {
                    result_html_parts.push(html! {
                        <tr class={class}>
                            <td>{ weap_name }</td>
                            {
                                quirk_keys_present.iter().map(|key: &&String| {
                                    let key: &str = key;
                                    html!{<td>
                                        { weap_quirks.get(key).map(|val|format!("{}", val)).unwrap_or("".to_string()) }
                                    </td>}
                                }).collect::<Html>()
                            }
                        </tr>
                    });
                }
            }

            if !any_weapon_matches_quirk_filter {
                return html! {};
            }
            result_html_parts.into_iter().collect::<Html>()
        };

        let mut table_class = String::new();
        if !self.settings.show_unquirked {
            table_class += &format!("no-unquirked");
        }
        html! {
            <table class={table_class}>
                <tr>
                    <th>{"ton"}</th>
                    <th>{"chassis"}</th>
                    <th>{"variant"}</th>
                    <th>{"weapon"}</th>
                    {
                        quirk_keys_present.iter().map(|key| {
                            let key = key.to_string();
                            let maybe_renamed = quirk_renames.get(&key.as_str()).map(|x| x.to_string()).unwrap_or(key.to_string());
                            let filter_val = self.settings.min_quirks.get(key.as_str()).copied().unwrap_or(0);
                            html! { <th>
                                {maybe_renamed} <br/>
                                <input type="number" min="0" max="200" class="quirk-filter" value={filter_val}
                                    onchange=self.link.callback(move |cd: yew::events::ChangeData| {
                                        let newval: i32 = match cd {
                                            yew::events::ChangeData::Value(s) => s.parse().unwrap(),
                                            _ => return Msg::Ignore,
                                        };
                                        Msg::Settings(MsgSettings::QuirkValue(key.clone(), newval))
                                    }) />
                            </th> }
                        }).collect::<Html>()
                    }
                </tr>
                { some_mechs.iter().map(|mech|{
                    { show_quirk_row(mech) }

                }).collect::<Html>() }
                </table>
        }
    }
    fn view_data(&self) -> Html {
        let data = if let Some(d) = &self.data {
            d
        } else {
            return html! {
                <p>{ "Fetching data..." }</p>
            };
        };

        html! {
           <div>
                { self.view_weapon_select(&data.equipment.weapons) }
                { self.view_checkboxes() }
                {
                    if let Some((weap, amt, ammo_amt)) = self.loadout.chosen_weapons.get(0) {
                        self.view_mech_list(data, weap.as_str(), *amt, *ammo_amt)
                    } else { html! {} }
                }
            </div>
        }
    }

    fn fetch_json(&mut self) -> yew::services::fetch::FetchTask {
        let callback = self.link.callback(
            move |response: Response<Json<Result<mwo_types::MechdataCombined, Error>>>| {
                let (meta, Json(data)) = response.into_parts();
                if meta.status.is_success() {
                    Msg::FetchReady(data)
                } else {
                    Msg::Ignore // FIXME: Handle this error accordingly.
                }
            },
        );
        let request = Request::get("mechdata_combined.min.json")
            .body(Nothing)
            .unwrap();
        FetchService::fetch(request, callback).unwrap()
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut m = Model {
            link,
            fetching: false,
            data: None,
            ft: None,
            show_weap: false,

            settings: Settings::default(),
            loadout: LoadoutSettings {
                chosen_weapons: vec![],
                min_speed: 0.0,
                armor: "none".into(),
            },
        };
        m.ft = Some(m.fetch_json());
        m
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FetchData => {
                self.fetching = true;
                let task = self.fetch_json();
                self.ft = Some(task);
            }
            Msg::FetchReady(response) => {
                self.fetching = false;
                self.ft = None;
                match response {
                    Ok(data) => self.data = Some(data),
                    Err(e) => ConsoleService::error(&format!("{:?}", e)),
                }
            }
            Msg::ToggleWeapons => self.show_weap = !self.show_weap,
            Msg::TweakLoadout(tlm) => match tlm {
                MsgTweakLoadout::AddWeapon(weap) => {
                    self.loadout.chosen_weapons.push((weap, 1, 0.0))
                }
                MsgTweakLoadout::RemoveWeapon(weap) => self
                    .loadout
                    .chosen_weapons
                    .retain(|(w, _, _)| w.as_str() != weap.as_str()),
                MsgTweakLoadout::ChangeWeaponAmt(weap, amt) => {
                    for (have_weap, have_weap_amt, have_ammo_amt) in
                        self.loadout.chosen_weapons.iter_mut()
                    {
                        if weap.as_str() == have_weap.as_str() {
                            *have_weap_amt = amt
                        }
                    }
                }
                MsgTweakLoadout::ChangeWeaponAmmoAmt(weap, ammo_amt) => {
                    for (have_weap, have_weap_amt, have_ammo_amt) in
                        self.loadout.chosen_weapons.iter_mut()
                    {
                        if weap.as_str() == have_weap.as_str() {
                            *have_ammo_amt = ammo_amt
                        }
                    }
                }
            },
            Msg::ChooseMinSpeed(spd) => self.loadout.min_speed = spd,
            Msg::ChooseArmor(arm) => self.loadout.armor = arm,
            Msg::Settings(set) => {
                use MsgSettings::*;
                match set {
                    ToggleHeroes => self.settings.show_heroes = !self.settings.show_heroes,
                    ToggleSpecials => self.settings.show_specials = !self.settings.show_specials,
                    ToggleUnquirked => self.settings.show_unquirked = !self.settings.show_unquirked,
                    QuirkValue(key, newval) => {
                        *self.settings.min_quirks.entry(key).or_default() = newval;
                    }
                }
            }
            Msg::Ignore => {
                return false;
            }
        }
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        match &self.data {
            None => html! { <div> { "Loading..." } </div> },
            Some(_) => self.view_data(),
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    App::<Model>::new().mount_to_body();
}
