# Hero stays on the paved road

The actual renderer/input probe `road-boundary` runs 920 updates at 30 Hz using
seed `0xA11CE`. It starts the hero at the road center, disables enemy spawns and
hero attacks, then injects real Bevy keyboard and touch messages. It does not
teleport the hero to the tested edges or call the clamp/dodge helper directly.

Both 1280×720 and 844×390 runs pass:

- Hold ArrowUp, then press Shift while still holding outward: `(10, 0, -4.4)`.
- Hold ArrowDown, then press Shift at the opposite edge: `(-10, 0, 4.4)`.
- Hold the actual movement pad upward and tap the visible dodge control with a
  second touch: `(10, 0, -4.4)`.
- Repeat downward: `(-10, 0, 4.4)`.
- Assert `abs(z) <= 4.4` after every gameplay update, including every dodge.
- Cancel the held touch and assert movement intent returns to zero.

Each dodge must activate its real cooldown; the probe cannot pass merely because
input was ignored. Screenshots at frames 180/380/650/890 show the resulting edge
positions. Both touch-stage screenshots were visually inspected at both sizes:
the hero stands on the paving inside the side wall instead of the empty outer
terrain. Frame 100 is an intermediate movement screenshot.

The initial probe setup accidentally retained the chapter menu and correctly
failed its input assertion. The scenario was moved after the shared capture
battle setup; accepted logs here are from the corrected run. This is native
rendering with synthetic input, not physical Android/iOS validation.

## Reproduce

```sh
nix develop --command cargo build --bin capture
nix develop --command env \
  VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json \
  ROGUE_COMMANDER_LANG=en \
  ROGUE_CAPTURE_SAVE_FRAMES=100,180,380,650,890 \
  target/debug/capture frames /tmp/road-boundary 920 road-boundary 0xA11CE 844x390
```

Use `1280x720` for the desktop run. Logs record the adapter, each edge/dodge
assertion, final cancellation result, and successful screenshot writes.
