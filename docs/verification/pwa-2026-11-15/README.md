# Mobile installation — 2026-11-15

- Relative-scope manifest, standalone display, landscape preference, 192/512 PNG
  icons, maskable icon and Apple touch icon. Build copies all web files.
- Mobile launch page offers installation or platform-specific instructions;
  browser play remains available. Standalone and appinstalled hide the invitation.
- Network-only service worker deliberately does not cache game builds or promise
  offline gameplay, avoiding stale WASM/assets between releases.
- Compact landscape launch page keeps both actions visible at 667×300.
  Portrait launch no longer hides installation behind the rotate overlay.

Browser verification (Chromium with emulated device UA/viewport):
- 844×390, 667×300 and 393×852 screenshots inspected.
- Actual clicks open/close iOS and Android instructions; English/Chinese switch works.
- Mock beforeinstallprompt invoked once; dismissal restores manual instructions.
- Mock appinstalled and navigator.standalone hide prompt and close open instructions.
- Real local service worker activated; manifest loaded with three valid RGBA8 icons.
- JS syntax checks pass. No browser page errors.

Native OS installation dialogs and physical iPhone/Android standalone launches
require device verification; mocks do not prove OS installation.

[Short landscape](short-landscape.png) · [Portrait](portrait.png) ·
[iPhone guide](ios-guide.png) · [Android guide](android-guide.png)
