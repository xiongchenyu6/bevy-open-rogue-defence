# Noto Sans SC UI subset

`rogue_commander_ui.ttf` is a project-specific subset of the official Noto Sans
SC variable TrueType font. It is used by both the web loading shell and the Bevy
UI so Chinese text does not depend on fonts installed on the player's device.

- Upstream: `google/fonts`
- Commit: `2796410152d4f9524b68ed46e69c1b60f8e0f7c3`
- Source: `ofl/notosanssc/NotoSansSC[wght].ttf`
- Source SHA-256: `a3041811a78c361b1de50f953c805e0244951c21c5bd412f7232ef0d899af0da`
- License source SHA-256: `babcfe66c8a098b2fa279bc724a3a342f8124f77ce18941fbcc1bbb39823cded`
- Subsetter: fontTools `4.63.0`
- Glyph corpus SHA-256: `99af9e45e80d065372f8cd930cfb1fb82509b46f85eafc47d0fdab4b90bc729b`
- Subset SHA-256: `d07e165bdbcd48c924e9b0bdbb8b8632d5ee4a243238e70fa5f5f1171bdd4ac4`
- Subset size: 471,652 bytes
- License: SIL Open Font License 1.1; see `OFL.txt`

`glyphs.txt` contains the non-ASCII characters currently present in the Rust
game sources and web entry page. Basic printable ASCII and non-breaking space
are added explicitly. Regenerate this file from the localization catalog after
changing player-facing copy, then rebuild and run the font coverage tests.

```sh
source_url='https://raw.githubusercontent.com/google/fonts/2796410152d4f9524b68ed46e69c1b60f8e0f7c3/ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf'
curl -fL "$source_url" -o /tmp/NotoSansSC-wght.ttf
printf '%s  %s\n' \
  a3041811a78c361b1de50f953c805e0244951c21c5bd412f7232ef0d899af0da \
  /tmp/NotoSansSC-wght.ttf | sha256sum --check
uvx --from fonttools==4.63.0 pyftsubset /tmp/NotoSansSC-wght.ttf \
  --text-file=assets/fonts/glyphs.txt \
  --unicodes='U+0020-007E,U+00A0' \
  --no-ignore-missing-unicodes \
  --notdef-glyph --notdef-outline --recommended-glyphs \
  --name-IDs='*' --name-languages='*' \
  --output-file=assets/fonts/rogue_commander_ui.ttf
```
