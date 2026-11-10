# industrialforegoing (Horizon port)

A Horizon port of [industrialforegoing](Buuz135/Industrial-Foregoing) — MIT.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- The block.** `industrialforegoing:water_condensator` on the
- `minecraft:iron_block`** template (upstream's
- `Properties.ofFullCopy(Blocks.IRON_BLOCK)`: hardness 5.0, blast resistance
- , correct tool for drops, iron sounds), with the **`subfacing`** state
- property Titanium's `RotationType.FOUR_WAY` uses — the jar's own four-variant
- blockstate selects on it unchanged.
- Placement rotation.** `state_for_placement` (API 76) sets `subfacing` from
- the placer's yaw, the face turning toward the placer (the e2e asserts `west`

## Not yet ported

- No GUI screen and no tooltip.** Upstream draws Titanium's tank, energy and
- progress bars in a 176-wide panel; the pin's screen helpers need a menu, and
- this machine has no slots, so slice 1 exposes the same numbers through
- `/industrialforegoing info` (precedent: `charginggadgets` draws its readouts
- as text; `/powah info`). The progress bar's client animation is absent.
- No redstone control modes.** Upstream's `RedstoneManager` offers
- ignore/on/once/with/without; the default action is `IGNORE`, which this port
- reproduces (the machine always runs). The modes are a gap.

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: Buuz135/Industrial-Foregoing. Port: AdLf0208. Licence: MIT — see [LICENSE](LICENSE);
the original notices are preserved.
