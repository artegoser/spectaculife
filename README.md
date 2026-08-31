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

## Configuration

Simulation, ecology and genetics live in `assets/simulation.ron`. Rendering lives
separately in `assets/render.ron`; renderer settings are not part of the simulation
configuration.

Probability tables use named relative weights rather than numeric `match` ranges. The
weights do not need to sum to 100. For example:

```ron
[
    (kind: MultiplySelf, weight: 1),
    (kind: Nothing, weight: 1),
    (kind: KillCell, weight: 3),
]
```

is interpreted as 20%, 20% and 60% inside that three-entry example. Changing one
weight does not require recalculating any numeric ranges.

`assets/simulation.ron` also exposes cell maintenance, organic mass, construction
costs, generator extraction/efficiency, energy transfer, collision damage, predation,
death recycling and mutation behavior.

Set `SPECTACULIFE_CONFIG=/path/to/file.ron` to load another simulation config and
`SPECTACULIFE_RENDER_CONFIG=/path/to/render.ron` to load another render config.

## Simulation stepping

A simulation tick is staged instead of updating cells in scan order. Independent cell
calculations run in parallel with Rayon, then neighbor writes are committed as a
separate phase. Energy sent during tick `N` is queued and can only be used by the
adjacent recipient in tick `N + 1`, so it cannot cascade through a pipe chain in a
single tick.

Rayon uses the machine's logical CPU count by default. Set `RAYON_NUM_THREADS` when
launching the program if you want to override the worker count.

## Rendering LOD

Close views use `bevy_fast_tilemap` so the original 16x16 organism sprites remain
crisp. Zoomed-out views are crossfaded into a precomposed world texture with a full
trilinear mip chain down to 1x1. The logarithmic fade range and soil visualization
scale are configured in `assets/render.ron`.

## Evolution defaults

The default random body program is tuned near the critical branching point rather
than far below or above it. With the shipped weights, a Stem has an expected
`4 * P(MultiplySelf) = 1.0` same-organism Stem children before spatial/resource
competition. This gives a broad morphology-size distribution without making every
founder die after a few cells or making random growth automatically explode.

`CreateSeed` is reproduction, not somatic growth. By default it creates a detached
organism with its own configurable starting energy and does not inherit a branch of
the parent's energy network. Only `CreateSeed` mutates the genome; `MultiplySelf`
copies it unchanged.
