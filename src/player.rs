use std::collections::HashMap;

use crate::{config::INPUT, gui};

use super::gamelog::GameLog;
use rltk::{Point, Rltk, VirtualKeyCode};
use specs::prelude::*;

use super::{components, config, map, systems, RunState, State};
pub use components::*;

pub fn make_character(ecs: &mut World) {
    // Here goes a function that initializes all of the rpgish character stuff
    // for now we just add a spell to the hotbar

    // TODO: look at the selection from prev menu and make diff spell
    systems::spell::fireball_spell(ecs, config::CONFIG.hk1.clone());
}

fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) -> RunState {
    let mut positions = ecs.write_storage::<Position>();
    let mut players = ecs.write_storage::<Player>();
    let mut viewsheds = ecs.write_storage::<Viewshed>();

    let antagonists = ecs.read_storage::<Antagonistic>();
    let map = ecs.fetch::<map::Map>();

    let entities = ecs.entities();
    let mut wants_to_melee = ecs.write_storage::<WantsToMelee>();

    for (entity, _player, pos, viewshed) in
        (&entities, &mut players, &mut positions, &mut viewsheds).join()
    {
        if pos.x + delta_x < 1
            || pos.x + delta_x > map.width - 1
            || pos.y + delta_y < 1
            || pos.y + delta_y > map.height - 1
        {
            return RunState::AwaitingInput;
        }

        let destination_idx = map.xy_idx(pos.x + delta_x, pos.y + delta_y);
        for potential_target in map.tile_content[destination_idx].iter() {
            let target = antagonists.get(*potential_target);
            if let Some(_target) = target {
                wants_to_melee
                    .insert(
                        entity,
                        WantsToMelee {
                            target: *potential_target,
                        },
                    )
                    .expect("Add target failed");
                return RunState::PlayerTurn;
            }
        }
        if !map.blocked[destination_idx] {
            pos.x = i32::clamp(pos.x + delta_x, 0, map.width - 1);
            pos.y = i32::clamp(pos.y + delta_y, 0, map.height - 1);

            let mut ppos = ecs.write_resource::<Point>();
            ppos.x = pos.x;
            ppos.y = pos.y;
            viewshed.dirty = true;
        }
    }
    RunState::PlayerTurn
}

fn _cast_spell(ecs: &mut World) {
    let player_entity = ecs.fetch::<Entity>();
    let cursor = ecs.fetch::<Cursor>();
    let mut castables = ecs.write_storage::<WantsToCastSpell>();
    // having some trouble with ecs here. spell isn't passed in here, we join it based on
    // keypress???

    // return RunState::ShowTargeting { range: (), item: () },

    //gui::ranged_target(ecs, ctx: &mut Rltk, range: i32);
    // -> SelectResult

    // Flow: press C, 1
    // move cursor, hit enter
    // spell takes effect
    //
    // Do we have a
    castables
        .insert(
            *player_entity,
            WantsToCastSpell {
                source: *player_entity,
                target: Some(cursor.point),
            },
        )
        .expect("Unable to insert want to cast");
}

fn get_item(ecs: &mut World) -> RunState {
    let player_pos = ecs.fetch::<Point>();
    let player_entity = ecs.fetch::<Entity>();
    let entities = ecs.entities();
    let items = ecs.read_storage::<Item>();
    let positions = ecs.read_storage::<Position>();
    let mut gamelog = ecs.fetch_mut::<GameLog>();

    let mut target_item: Option<Entity> = None;
    for (item_entity, _item, position) in (&entities, &items, &positions).join() {
        if position.x == player_pos.x && position.y == player_pos.y {
            target_item = Some(item_entity);
        }
    }

    match target_item {
        None => {
            gamelog.entries.push("There is nothing here to pick up.".to_string());
            return RunState::AwaitingInput;
        },
        Some(item) => {
            let mut pickup = ecs.write_storage::<WantsToPickupItem>();
            pickup
                .insert(
                    *player_entity,
                    WantsToPickupItem {
                        collected_by: *player_entity,
                        item,
                    },
                )
                .expect("Unable to insert want to pickup");
            return RunState::PlayerTurn;
        }
    }
}

fn use_hotkey(_ecs: &mut World, key: VirtualKeyCode) -> RunState{
    // TODO: some kind of key:item entity lookup
    dbg!(key);
    RunState::PlayerTurn
}

fn descend(ecs: &mut World) -> RunState{
    if map::try_next_level(ecs) {
        return RunState::NextLevel;
    }
    RunState::AwaitingInput
}

fn to_main_menu() -> RunState {
    return RunState::MainMenu { game_started: true, menu_selection: (gui::MainMenuSelection::NewGame)};
}

// TODO: protect from overflow on char/item select window

pub fn player_input(gs: &mut State, ctx: &mut Rltk) -> RunState {
    let mut player_action_map: HashMap<VirtualKeyCode, Box<dyn Fn(&mut World) -> RunState>> = HashMap::new();
    player_action_map.insert(INPUT.left, Box::new(|w| try_move_player(-1, 0, w)));
    player_action_map.insert(INPUT.down, Box::new(|w| try_move_player(0, 1, w)));
    player_action_map.insert(INPUT.up, Box::new(|w| try_move_player(0, -1, w)));
    player_action_map.insert(INPUT.right, Box::new(|w| try_move_player(1, 0, w)));
    player_action_map.insert(INPUT.pick_up, Box::new(|w| get_item(w)));
    player_action_map.insert(INPUT.inventory, Box::new(|_| return RunState::ShowInventory { selection: 0 }));
    player_action_map.insert(INPUT.select, Box::new(|w| descend(w)));
    player_action_map.insert(INPUT.exit, Box::new(|_| to_main_menu()));
    player_action_map.insert(INPUT.wait, Box::new(|_| return RunState::PlayerTurn));

    player_action_map.insert(INPUT.hk1, Box::new(|w| use_hotkey(w, INPUT.hk1)));
    player_action_map.insert(INPUT.hk2, Box::new(|w| use_hotkey(w, INPUT.hk2)));
    player_action_map.insert(INPUT.hk3, Box::new(|w| use_hotkey(w, INPUT.hk3)));
    player_action_map.insert(INPUT.hk4, Box::new(|w| use_hotkey(w, INPUT.hk4)));

    match ctx.key {
        None => return RunState::AwaitingInput,
        Some(key) => {
            if !player_action_map.contains_key(&key){
                return RunState::PlayerTurn;
            }
            return player_action_map.get(&key).unwrap()(&mut gs.ecs);
        }
    }
}
