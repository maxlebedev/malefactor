use rltk::{Point, Rltk, VirtualKeyCode, RGB};
use specs::prelude::*;
use std::cmp::{max, min};

use super::map::{MAPHEIGHT, MAPWIDTH};
use super::{components, config, GameLog, Player, RunState, State};
pub use components::*;


#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuSelection {
    NewGame,
    LoadGame,
    Quit,
}

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuResult {
    NoSelection { selected: MainMenuSelection },
    Selected { selected: MainMenuSelection },
}

#[derive(PartialEq, Copy, Clone)]
pub enum ItemMenuResult {
    Cancel,
    NoResponse,
    Up,
    Down,
    Selected,
    Drop,
}

#[derive(PartialEq, Copy, Clone)]
pub enum SelectResult {
    Cancel,
    NoResponse,
    Selected,
}

pub fn draw_ui(ecs: &World, ctx: &mut Rltk) {
    let white = RGB::named(rltk::WHITE);
    let black = RGB::named(rltk::BLACK);
    let yellow = RGB::named(rltk::YELLOW);
    let red = RGB::named(rltk::RED);
    ctx.draw_box(0, MAPHEIGHT, MAPWIDTH - 1, 6, white, black);

    let combat_stats = ecs.read_storage::<CombatStats>();
    let players = ecs.read_storage::<Player>();
    for (_player, stats) in (&players, &combat_stats).join() {
        let health = format!(" HP: {} / {} ", stats.hp, stats.max_hp);
        ctx.print_color(12, MAPWIDTH, yellow, black, &health);

        ctx.draw_bar_horizontal(28, MAPHEIGHT, 51, stats.hp, stats.max_hp, red, black);
    }
    let log = ecs.fetch::<GameLog>();


    let mut y = MAPHEIGHT + 1; // 44;
    for s in log.entries.iter().rev() {
        if y < MAPHEIGHT + 6 { // 49
            ctx.print(2, y, s);
        }
        y += 1;
    }
}

pub fn show_inventory(
    gs: &mut State,
    ctx: &mut Rltk,
    selection: usize,
) -> (ItemMenuResult, Option<Entity>) {
    let white = RGB::named(rltk::WHITE);
    let black = RGB::named(rltk::BLACK);
    let yellow = RGB::named(rltk::YELLOW);
    let magenta = RGB::named(rltk::MAGENTA);

    let fgcolor = white;
    let bgcolor = black;
    let hlcolor = magenta;

    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names, &entities)
        .join()
        .filter(|item| item.0.owner == *player_entity);

    let halfwidth = MAPWIDTH / 2;
    ctx.draw_box(0, 0, halfwidth, MAPHEIGHT, fgcolor, bgcolor);
    ctx.draw_box(halfwidth + 1, 0, halfwidth, MAPHEIGHT, fgcolor, bgcolor);
    ctx.print_color_centered(0, yellow, bgcolor, "Inventory");
    ctx.print_color_centered(MAPHEIGHT, yellow, bgcolor, "ESCAPE to cancel");

    let inv_offset = 2;
    let mut equippable: Vec<Entity> = Vec::new();
    for (y, item) in inventory.enumerate() {
        let mut color = fgcolor;
        if y == selection {
            color = hlcolor;
        }
        ctx.print_color(
            inv_offset,
            y + inv_offset,
            color,
            bgcolor,
            &item.1.name.to_string(),
        );
        equippable.push(item.2);
    }

    let up = config::cfg_to_kc(&config::CONFIG.up);
    let down = config::cfg_to_kc(&config::CONFIG.down);
    let exit = config::cfg_to_kc(&config::CONFIG.exit);
    let drop = config::cfg_to_kc(&config::CONFIG.drop);
    let select = config::cfg_to_kc(&config::CONFIG.select);
    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => match key {
            _ if key == exit => (ItemMenuResult::Cancel, None),
            _ if key == up => (ItemMenuResult::Up, None),
            _ if key == down => (ItemMenuResult::Down, None),
            _ if key == drop => (ItemMenuResult::Drop, Some(equippable[selection])),
            _ if key == select => (ItemMenuResult::Selected, Some(equippable[selection])),
            _ => (ItemMenuResult::NoResponse, None),
        },
    }
}

pub fn ranged_target(gs: &mut State, ctx: &mut Rltk, range: i32) -> (SelectResult, Option<Point>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let player_pos = gs.ecs.fetch::<Point>();
    let viewsheds = gs.ecs.read_storage::<Viewshed>();

    let yellow = RGB::named(rltk::YELLOW);
    let black = RGB::named(rltk::BLACK);
    let cyan = RGB::named(rltk::CYAN);
    ctx.print_color(5, 0, yellow, black, "Select Target:");

    // Highlight available target cells
    let mut available_cells = Vec::new();
    let visible = viewsheds.get(*player_entity);
    if let Some(visible) = visible {
        // We have a viewshed
        for idx in visible.visible_tiles.iter() {
            let distance = rltk::DistanceAlg::Pythagoras.distance2d(*player_pos, *idx);
            if distance <= range as f32 {
                ctx.set_bg(idx.x, idx.y, RGB::named(rltk::BLUE));
                available_cells.push(idx);
            }
        }
    } else {
        return (SelectResult::Cancel, None);
    }

    // Draw mouse cursor
    let mouse_pos = ctx.mouse_pos();
    let mut valid_target = false;
    for idx in available_cells.iter() {
        if idx.x == mouse_pos.0 && idx.y == mouse_pos.1 {
            valid_target = true;
        }
    }
    if valid_target {
        ctx.set_bg(mouse_pos.0, mouse_pos.1, cyan);
        if ctx.left_click {
            return (
                SelectResult::Selected,
                Some(Point::new(mouse_pos.0, mouse_pos.1)),
            );
        }
    } else {
        ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::RED));
        if ctx.left_click {
            return (SelectResult::Cancel, None);
        }
    }

    (SelectResult::NoResponse, None)
}

pub fn main_menu(gs: &mut State, ctx: &mut Rltk) -> MainMenuResult {
    let save_exists = super::systems::save_load::does_save_exist();
    let runstate = gs.ecs.fetch::<RunState>();

    let white = RGB::named(rltk::WHITE);
    let yellow = RGB::named(rltk::YELLOW);
    let black = RGB::named(rltk::BLACK);
    let magenta = RGB::named(rltk::MAGENTA);

    ctx.print_color_centered(15, yellow, black, "Malefactor");

    let states = [
        MainMenuSelection::NewGame,
        MainMenuSelection::LoadGame,
        MainMenuSelection::Quit,
    ];

    let mut idx: i8;
    let state_num: i8 = states.len() as i8;

    if let RunState::MainMenu {
        menu_selection: selection,
    } = *runstate
    {
        let mut ngcolor = white;
        let mut lgcolor = white;
        let mut qcolor = white;
        match selection {
            MainMenuSelection::NewGame => {
                ngcolor = magenta;
                idx = 0;
            }
            MainMenuSelection::LoadGame => {
                lgcolor = magenta;
                idx = 1;
            }
            MainMenuSelection::Quit => {
                qcolor = magenta;
                idx = 2;
            }
        }

        ctx.print_color_centered(24, ngcolor, black, "Begin New Game");
        ctx.print_color_centered(25, lgcolor, black, "Load Game");
        ctx.print_color_centered(26, qcolor, black, "Quit");

        let down = config::cfg_to_kc(&config::CONFIG.down);
        let up = config::cfg_to_kc(&config::CONFIG.up);
        let exit = config::cfg_to_kc(&config::CONFIG.exit);
        match ctx.key {
            None => {
                return MainMenuResult::NoSelection {
                    selected: selection,
                }
            }
            Some(key) => match key {
                _ if key == exit => {
                    return MainMenuResult::NoSelection {
                        selected: MainMenuSelection::Quit,
                        // TODO: here we can continue. maybe?
                        // Alternatively there would need to be a continue button
                    }
                }
                _ if key == up => {
                    idx = max(0, idx - 1);
                    let mut newselection = states[idx as usize];
                    if newselection == MainMenuSelection::LoadGame && !save_exists {
                        newselection = MainMenuSelection::NewGame;
                    }
                    return MainMenuResult::NoSelection {
                        selected: newselection,
                    };
                }
                _ if key == down => {
                    idx = min(state_num-1, idx + 1);
                    let mut newselection = states[idx as usize];

                    if newselection == MainMenuSelection::LoadGame && !save_exists {
                        newselection = MainMenuSelection::Quit;
                    }
                    return MainMenuResult::NoSelection {
                        selected: newselection,
                    };
                }
                VirtualKeyCode::Return => {
                    return MainMenuResult::Selected {
                        selected: selection,
                    }
                }
                _ => {
                    return MainMenuResult::NoSelection {
                        selected: selection,
                    }
                }
            },
        }
    }

    MainMenuResult::NoSelection {
        selected: MainMenuSelection::NewGame,
    }
}
