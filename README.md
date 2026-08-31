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
