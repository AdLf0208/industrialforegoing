// Game-side glue for the Industrial Foregoing slice 1: the Water Condensator's
// block entity, its FE buffer, its water tank, its tick and the harness
// commands.
//
// Injected into `crate::game` as `industrialforegoing_state`; it needs the game,
// so it is not part of the standalone library (only `runtime.rs` is).
//
// The block entity keeps upstream's saved values: the buffer's `energy` int
// (API 142 keeps it there), the tank under `fluids` (API 54's shape) and the
// progress bar as `progress`. Every server tick counts the bar down and, at
// zero, counts the water source blocks on the six sides and runs one attempt.
//
// Read from Industrial Foregoing 1.21-3.6.39 (`WaterCondensatorTile`,
// `IndustrialWorkingTile`, Titanium's `ProgressBarComponent`,
// `EnergyStorageComponent` and `FluidTankComponent`). See `../../FINDINGS.md`.

use std::array;

use horizon_client::containers::{BlockEntity, ModBlockEntity, ModEnergyBuffer, ModFluidTanks};
use horizon_client::text::Text;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::Game;
use crate::game::industrialforegoing::{
    CAPACITY_FE, Condensator, MAX_PROGRESS, TANK_CAPACITY_MB,
};
use crate::game::native_server::{NativeCommand, NativeServerPlayer};

/// The machine (`ModuleResourceProduction.WATER_CONDENSATOR`).
pub const BLOCK: &str = "industrialforegoing:water_condensator";
/// The block state property Titanium's `RotationType.FOUR_WAY` sets.
pub const SUBFACING: &str = "subfacing";
/// The saved keys: the buffer's amount (API 142's own), the bar and the tank.
const KEY_ENERGY: &str = "energy";
const KEY_PROGRESS: &str = "progress";
/// `minecraft:water`, the only fluid the tank takes.
const WATER: &str = "minecraft:water";
/// A side the tank is read and drained through (all six reach it).
const READ_FACE: Direction = Direction::Up;

/// The six sides, in `Direction`'s order (down, up, north, south, west, east).
const SIDES: [[i32; 3]; 6] = [
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];

// ---------------------------------------------------------------------------
// Declaration hooks
// ---------------------------------------------------------------------------

/// The machine has a block entity (`native_block_entity`, API 15).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == BLOCK {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

/// ... and ticks on the server (`native_block_entity_ticks`, API 15).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity_ticks(block: &str, ticks: &mut bool) {
    if block == BLOCK {
        *ticks = true;
    }
}

/// The FE buffer (`native_block_entity_energy`, API 142): 10,000 FE, any side
/// through either way, as Titanium's `EnergyStorageComponent(10000, 10, 20)`
/// takes and gives up to its capacity.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity_energy(block: &str, energy: &mut Option<ModEnergyBuffer>) {
    if block == BLOCK {
        *energy = Some(ModEnergyBuffer {
            capacity: CAPACITY_FE,
            max_insert: CAPACITY_FE,
            max_extract: CAPACITY_FE,
            insert_faces: [true; 6],
            extract_faces: [true; 6],
        });
    }
}

/// One 16,000 mB water tank, reachable from every side for reading and
/// draining but never filled from outside (`SidedFluidTankComponent` with
/// `Action.DRAIN`; the machine fills it itself).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity_fluids(block: &str, fluids: &mut Option<ModFluidTanks>) {
    if block != BLOCK {
        return;
    }
    let all = vec![0usize];
    *fluids = Some(ModFluidTanks {
        tanks: vec![TANK_CAPACITY_MB],
        faces: array::from_fn(|_| all.clone()),
        inputs: Vec::new(),
        outputs: vec![0],
    });
}

// ---------------------------------------------------------------------------
// The tick
// ---------------------------------------------------------------------------

/// One server tick (`native_block_entity_tick`, API 17): count the bar down,
/// and at zero pass the water sources beside it to the pure attempt.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_tick(game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.id != BLOCK {
        return;
    }
    let mut machine = Condensator::from_saved(
        read_int(&entity.nbt, KEY_ENERGY),
        read_tank(&entity.nbt),
        read_progress(&entity.nbt),
    );
    let sources = water_sources(game, pos);
    let before_energy = machine.energy;
    let before_water = machine.water_mb;
    machine.tick(sources);
    if machine.energy != before_energy {
        write_int(&mut entity.nbt, KEY_ENERGY, machine.energy);
    }
    if machine.water_mb != before_water {
        write_tank(&mut entity.nbt, machine.water_mb);
    }
    write_int(&mut entity.nbt, KEY_PROGRESS, machine.progress);
    // `setChanged`: the chunk is written with what this tick did.
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
}

/// The water source blocks on the six sides of `pos` (`getWaterSources`: a
/// `FluidState.isSource`, which for water is the still, non-falling state —
/// the block registry's default `minecraft:water` state; flowing water and a
/// waterlogged block's water are not).
fn water_sources(game: &Game<'_>, pos: [i32; 3]) -> u32 {
    let Some(source) = game.pack.assets.registry.default_state(WATER) else {
        return 0;
    };
    SIDES
        .iter()
        .filter(|[dx, dy, dz]| {
            game.world
                .block(pos[0] + dx, pos[1] + dy, pos[2] + dz)
                == source
        })
        .count() as u32
}

/// A placed machine faces its placer (`RotationType.FOUR_WAY` with the four
/// `subfacing` values), `native_state_for_placement` (API 76).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_state_for_placement(
    level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId,
    context: &horizon_world::level::PlacementContext,
    answer: &mut Option<Option<horizon_world::registry::BlockStateId>>,
) {
    let Some(block) = level.registry.state(state).map(|state| state.name.clone()) else {
        return;
    };
    if &*block != BLOCK {
        return;
    }
    if let Some(faced) = level.native_state_with_property(state, SUBFACING, facing_for_yaw(context.yaw))
    {
        *answer = Some(Some(faced));
    }
}

/// The horizontal `subfacing` a machine takes when a placer with `yaw` places
/// it: Titanium's `FOUR_WAY` turns the block toward the placer.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn facing_for_yaw(yaw: f32) -> &'static str {
    let quarter = ((f64::from(yaw) / 90.0 + 0.5).floor() as i64 & 3) as u8;
    match quarter {
        0 => "north",
        1 => "east",
        2 => "south",
        _ => "west",
    }
}

// ---------------------------------------------------------------------------
// The `/industrialforegoing` harness controls
// ---------------------------------------------------------------------------

/// The commands (`native_register_commands`, API 8). Upstream has none: they
/// exist so the harness can read and set a machine without a GUI (precedent:
/// `/powah info|set`, `/charginggadgets info|energy`).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("industrialforegoing info <pos:block_pos>"));
    commands.push(NativeCommand::op(
        "industrialforegoing energy <pos:block_pos> <amount:integer>",
        2,
    ));
    commands.push(NativeCommand::op(
        "industrialforegoing water <pos:block_pos> <amount:integer>",
        2,
    ));
    commands.push(NativeCommand::op(
        "industrialforegoing progress <pos:block_pos> <ticks:integer>",
        2,
    ));
    commands.push(NativeCommand::op(
        "industrialforegoing drain <pos:block_pos> <amount:integer>",
        2,
    ));
}

/// `/industrialforegoing info|energy|water|progress|drain` (`native_perform_command`,
/// API 8).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_perform_command(
    game: &mut Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    if words.first() != Some(&"industrialforegoing") {
        return;
    }
    *ran = true;
    match words.get(1).copied() {
        Some("info") => {
            let Some(pos) = parse_pos(&words[2..]) else {
                game.native_feedback(Text::literal(
                    "usage: /industrialforegoing info <x> <y> <z>",
                ));
                return;
            };
            game.native_feedback(Text::literal(readout(game, pos)));
        }
        Some("energy") => {
            let Some((pos, amount)) = parse_pos_amount(&words[2..]) else {
                game.native_feedback(Text::literal(
                    "usage: /industrialforegoing energy <x> <y> <z> <amount>",
                ));
                return;
            };
            let amount = amount.clamp(0, CAPACITY_FE);
            let changed = game
                .native_with_mod_block_entity(pos, |entity| {
                    if entity.id != BLOCK {
                        return false;
                    }
                    write_int(&mut entity.nbt, KEY_ENERGY, amount);
                    true
                })
                .unwrap_or(false);
            game.native_feedback(Text::literal(if changed {
                format!("Water Condensator at {pos:?}: {amount}/{CAPACITY_FE} FE.")
            } else {
                format!("{pos:?} is not a water condensator.")
            }));
        }
        Some("water") => {
            let Some((pos, amount)) = parse_pos_amount(&words[2..]) else {
                game.native_feedback(Text::literal(
                    "usage: /industrialforegoing water <x> <y> <z> <amount>",
                ));
                return;
            };
            let amount = amount.clamp(0, TANK_CAPACITY_MB);
            let changed = game
                .native_with_mod_block_entity(pos, |entity| {
                    if entity.id != BLOCK {
                        return false;
                    }
                    write_tank(&mut entity.nbt, amount);
                    true
                })
                .unwrap_or(false);
            game.native_feedback(Text::literal(if changed {
                format!(
                    "Water Condensator at {pos:?}: water {amount}/{TANK_CAPACITY_MB} mB."
                )
            } else {
                format!("{pos:?} is not a water condensator.")
            }));
        }
        Some("progress") => {
            let Some((pos, ticks)) = parse_pos_amount(&words[2..]) else {
                game.native_feedback(Text::literal(
                    "usage: /industrialforegoing progress <x> <y> <z> <ticks>",
                ));
                return;
            };
            let progress = ticks.clamp(0, MAX_PROGRESS);
            let changed = game
                .native_with_mod_block_entity(pos, |entity| {
                    if entity.id != BLOCK {
                        return false;
                    }
                    write_int(&mut entity.nbt, KEY_PROGRESS, progress);
                    true
                })
                .unwrap_or(false);
            game.native_feedback(Text::literal(if changed {
                format!("Water Condensator at {pos:?}: progress {progress}/{MAX_PROGRESS}.")
            } else {
                format!("{pos:?} is not a water condensator.")
            }));
        }
        Some("drain") => {
            let Some((pos, amount)) = parse_pos_amount(&words[2..]) else {
                game.native_feedback(Text::literal(
                    "usage: /industrialforegoing drain <x> <y> <z> <amount>",
                ));
                return;
            };
            let dimension = game.native_dimension().to_owned();
            match game.native_fluid_extract(&dimension, pos, READ_FACE, amount.max(0)) {
                Some((fluid, taken)) => game.native_feedback(Text::literal(format!(
                    "Water Condensator at {pos:?}: drained {taken} mB of {fluid}."
                ))),
                None => game.native_feedback(Text::literal(format!(
                    "Water Condensator at {pos:?}: no drainable fluid."
                ))),
            }
        }
        _ => game.native_feedback(Text::literal(
            "usage: /industrialforegoing info|energy|water|progress|drain ...",
        )),
    }
}

/// The machine's state, as `/industrialforegoing info` prints it.
fn readout(game: &Game<'_>, pos: [i32; 3]) -> String {
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&(pos[0], pos[1], pos[2]))
    else {
        return format!("{pos:?} is not a mod block entity.");
    };
    if entity.id != BLOCK {
        return format!("{pos:?} is a {} not a water condensator.", entity.id);
    }
    let energy = read_int(&entity.nbt, KEY_ENERGY).clamp(0, CAPACITY_FE);
    let water = read_tank(&entity.nbt).clamp(0, TANK_CAPACITY_MB);
    let progress = read_progress(&entity.nbt).unwrap_or(MAX_PROGRESS);
    let sources = water_sources(game, pos);
    let facing = game
        .native_block_property(pos, SUBFACING)
        .unwrap_or_else(|| "unknown".to_owned());
    format!(
        "Water Condensator at {pos:?}: {energy}/{CAPACITY_FE} FE, water {water}/{TANK_CAPACITY_MB} mB, sources {sources}, progress {progress}/{MAX_PROGRESS}, facing {facing}, t={}",
        game.world.game_time
    )
}

fn parse_pos(words: &[&str]) -> Option<[i32; 3]> {
    Some([
        words.first()?.parse().ok()?,
        words.get(1)?.parse().ok()?,
        words.get(2)?.parse().ok()?,
    ])
}

fn parse_pos_amount(words: &[&str]) -> Option<([i32; 3], i32)> {
    let pos = parse_pos(words)?;
    let amount = words.get(3)?.parse().ok()?;
    Some((pos, amount))
}

// ---------------------------------------------------------------------------
// Saved-compound reads
// ---------------------------------------------------------------------------

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&JavaString::from(key)) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(JavaString::from(key), Tag::Int(value));
}

/// A machine saved before this slice (or just placed) has no bar yet: full, as
/// `ProgressBarComponent` starts it.
fn read_progress(nbt: &Compound) -> Option<i32> {
    match nbt.get(&JavaString::from(KEY_PROGRESS)) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

/// The tank's amount, in the pin's `fluids` shape (API 54): one entry per tank.
fn read_tank(nbt: &Compound) -> i32 {
    let Some(Tag::List(list)) = nbt.get(&JavaString::from("fluids")) else {
        return 0;
    };
    let Some(Tag::Compound(entry)) = list.first() else {
        return 0;
    };
    match entry.get(&JavaString::from("Fluid")) {
        Some(Tag::String(fluid)) if fluid.to_string_lossy() == WATER => {}
        _ => return 0,
    }
    match entry.get(&JavaString::from("Amount")) {
        Some(Tag::Int(amount)) => *amount,
        _ => 0,
    }
}

/// Write the tank back in the pin's `fluids` shape (one tank).
fn write_tank(nbt: &mut Compound, mb: i32) {
    let mut entry = Compound::new();
    entry.insert(
        JavaString::from("Fluid"),
        Tag::String(JavaString::from(if mb > 0 {
            WATER
        } else {
            "minecraft:empty"
        })),
    );
    entry.insert(
        JavaString::from("Amount"),
        Tag::Int(mb.clamp(0, TANK_CAPACITY_MB)),
    );
    nbt.insert(JavaString::from("fluids"), Tag::List(vec![Tag::Compound(entry)]));
}
