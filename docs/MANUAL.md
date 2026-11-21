# SALVE manual

## Signal flow

```
in ─► detector (STEREO / L-R / M-S / MID / SIDE) ─► 48 dynamic bells ─► MIX with dry ─► DELTA ─► TRIM ─► SAFE
```

## Controls

| Control | What it does |
|---|---|
| **DEPTH** | How much the resonances are pulled down. |
| **SHARPNESS** | Narrow (surgical) to wide (smooth) cuts. |
| **SELECTIVITY** | How far a peak must stick out above its neighbours before it's treated. Higher = only the worst offenders. |
| **ATTACK / RELEASE** | How fast the cuts react and recover (ms). |
| **SOFT / HARD** | Soft eases into the cut; hard clamps it. |
| **CHANNELS** | STEREO is linked; L / R and M / S treat each side on its own; MID or SIDE treats only that part. |
| **MIX / TRIM** | Blend with the dry signal and set the output level. |
| **DELTA** | Plays only what SALVE removes, so you can check you're not cutting the tone itself. |
| **CARVE** | Sidechain mode. The key (sidechain input) decides where to cut, so the track makes room for the kick or vocal. |

## Display

- The spectrum shows input and output, with the cuts hanging from the top in gold.
- The band-editor curve has six nodes. Drag a node to change its frequency and amount; double-click to put it back to 100 %.
- The LO / HI tabs at the bottom of the display limit the range SALVE works on.
