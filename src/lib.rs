#![recursion_limit = "512"]
mod mwo_types;
mod some;

use mwo_types::Specialness;

use wasm_bindgen::prelude::*;
use yew::prelude::*;
#[allow(unused_imports)]
use yew::services::console::ConsoleService;

use anyhow::Error;
use serde::{Deserialize, Serialize};
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
pub enum Msg {
    FetchData,
    FetchReady(Result<mwo_types::MechdataCombined2, Error>),
    Ignore,
    ToggleWeapons,
    ChooseWeapon(String),
    ChooseWeaponAmt(i32),
    Settings(MsgSettings),
}

/// This type is used to parse data from `./static/data.json` file and
/// have to correspond the data layout from that file.
#[derive(serde::Deserialize, Debug)]
pub struct DataFromFile {
    value: u32,
}

/// This type is used as a request which sent to websocket connection.
#[derive(Serialize, Debug)]
struct WsRequest {
    value: u32,
}

/// This type is an expected response from a websocket connection.
#[derive(Deserialize, Debug)]
pub struct WsResponse {
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
pub struct Model {
    link: ComponentLink<Model>,
    fetching: bool,
    data: Option<mwo_types::MechdataCombined2>,
    ft: Option<FetchTask>,
    show_weap: bool,
    chosen_weapon: Option<String>,
    chosen_weapon_amt: i32,

    settings: Settings,
}

impl Model {
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
                <select
                    onchange=self.link.callback(|change: yew::events::ChangeData|{
                     match change {
                         yew::events::ChangeData::Select(element) => {
                             Msg::ChooseWeapon(element.value())
                         },
                         _ => Msg::Ignore,
                     }
                } )>
                    {std::iter::once(
                        html!{ <option hidden=true disabled=true selected=true></option> }
                    ).chain(weapons.iter().map(|w|
                        html!{ <option> {w.name.as_str()}</option> }
                    )).collect::<Html>() }
                </select>
                <input type="number" min="0" max="20" value={self.chosen_weapon_amt}
                    onchange=self.link.callback(|change: yew::events::ChangeData|{
                    match change {
                        yew::events::ChangeData::Value(val) => {
                            Msg::ChooseWeaponAmt(val.parse().unwrap())
                        },
                        _ => Msg::Ignore,
                    }
                })/>
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
    fn view_mech_list(&self, data: &mwo_types::MechdataCombined2, weapon: &str, amt: i32) -> Html {
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
        let mechs_quirked = some::stuffs(&data.weapons, &mech_variants, &mech_map, weapon, amt);
        let mut some_mechs = mech_variants.clone();
        some_mechs.sort_by_key(|m| m.max_tons);
        ConsoleService::log("hello");
        let quirk_keys_present = mechs_quirked
            .values()
            .flat_map(|stuff| stuff.1.keys())
            .collect::<BTreeSet<_>>();
        let quirk_renames = vec![("minheatpenaltylevel", "hsl")]
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let show_quirk_row = |mech: &mwo_types::Variant| -> Html {
            let empty_map = (
                some::FitStatus {
                    fits: some::TriState::Maybe,
                },
                std::collections::BTreeMap::new(),
            );
            let (can_mount, quirks) = mechs_quirked.get(&mech.variant_name).unwrap_or(&empty_map);
            let mut class = "".to_string();
            if can_mount.fits == some::TriState::No {
                class += " cant-fit"
            }
            if quirks.is_empty() {
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
                    quirks.get(quirkname).copied().unwrap_or(0.0).abs() < *min_value as f32
                })
            {
                return html! {};
            }

            html! {
            <tr class={class}>
                <td>{ mech.max_tons }</td>
                <td>{ &mech.chassis }</td>
                <td>{ &mech.variant_name }</td>
                {
                    quirk_keys_present.iter().map(|key: &&String| {
                        let key: &str = key;
                        html!{<td>
                            { quirks.get(key).map(|val|format!("{}", val)).unwrap_or("".to_string()) }
                        </td>}
                    }).collect::<Html>()
                }
            </tr>
            }
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
                <p>{ "Data hasn't fetched yet." }</p>
            };
        };

        html! {
           <div>
                { self.view_weapon_select(&data.weapons) }
                { self.view_checkboxes() }
                {
                    if let Some(weap) = &self.chosen_weapon{
                        self.view_mech_list(data, weap.as_str(), self.chosen_weapon_amt)
                    } else { html! {} }
                }
            </div>
        }
    }

    fn fetch_json(&mut self) -> yew::services::fetch::FetchTask {
        let callback = self.link.callback(
            move |response: Response<Json<Result<mwo_types::MechdataCombined2, Error>>>| {
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
        Model {
            link,
            fetching: false,
            data: None,
            ft: None,
            show_weap: false,
            chosen_weapon: None,
            chosen_weapon_amt: 0,

            settings: Settings::default(),
        }
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
            Msg::ChooseWeapon(name) => self.chosen_weapon = Some(name),
            Msg::ChooseWeaponAmt(amt) => self.chosen_weapon_amt = amt,
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
        html! {
            <div>
                <nav class="menu">
                    {
                        if self.data.is_none() {
                        html!{<button onclick=self.link.callback(|_| Msg::FetchData)>
                            { "Fetch Data" }
                        </button>}
                        } else { html!{} }
                    }
                    { self.view_data() }
                </nav>
            </div>
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    App::<Model>::new().mount_to_body();
}
