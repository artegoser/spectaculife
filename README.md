# spectaculife

Spectacular life simulation.

## Controls

- `Space` — pause / resume
- `N` — single simulation step
- `I` — reset/reinitialize the world
- `O` — organics layer
- `L` — life layer
- `P` — pollution layer
- `S` — soil-energy layer
- `D` — energy-path layer
- `H` — HUD
- LMB/RMB drag — pan camera
- mouse wheel — zoom

The same controls and current simulation settings are shown in the in-game HUD.

## Simulation config

Simulation/genetics tuning lives in `assets/simulation.ron`.

Probability tables use named relative weights rather than numeric `match` ranges. The weights do not need to sum to 100. For example:

```ron
[
    (kind: MultiplySelf, weight: 6),
    (kind: Nothing, weight: 4),
    (kind: KillCell, weight: 1),
]
```

is automatically interpreted as 6/11, 4/11 and 1/11 respectively. Changing one weight does not require changing any other range.

Set `SPECTACULIFE_CONFIG=/path/to/file.ron` to load another config file at startup.

## Simulation stepping

A simulation tick is staged instead of updating cells in scan order. Independent
cell calculations run in parallel with Rayon, then neighbor writes are committed
as a separate phase. Energy sent during tick `N` is queued and can only be used
by the adjacent recipient in tick `N + 1`, so it cannot cascade through a pipe
chain in a single tick.

Rayon uses the machine's logical CPU count by default. Set `RAYON_NUM_THREADS`
when launching the program if you want to override the worker count.

## Rendering LOD

Close views use `bevy_fast_tilemap` so the original 16x16 organism sprites remain crisp.
When the camera reaches `render.mip_lod_camera_scale` (16 by default), rendering switches
to a precomposed world texture with a complete mip chain down to 1x1. This makes
zoomed-out views represent the average contents of many cells instead of aliasing a
single arbitrary cell. The threshold is configured in `assets/simulation.ron`.
