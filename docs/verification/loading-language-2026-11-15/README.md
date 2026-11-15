# Loading-screen language switching

The Start handler disabled both language buttons until startup failed or completed.
Buttons now remain available while resources prepare and the engine downloads.
The current status key and progress arguments are retained, so changing language
immediately translates the status without resetting progress or the loading/retry label.
The selected language is persisted and written to the URL used by game startup.

Validation: `tools/test_pwa_update_ui.cjs` passed all 9 scenarios, including switching
both ways before Start and while a real service-worker resource download is held.
The engine itself is stubbed in this UI regression suite. Existing cache reuse,
explicit updates, failure fallback and legacy migration scenarios also passed.
Desktop 1280×720 and phone landscape 844×390 screenshots were visually inspected.
Physical phone testing was not performed.

- [Desktop](language-during-loading.png)
- [Phone landscape](language-during-loading-mobile.png)
- [Test output](results.txt)

The same regression harness was also run against the parent revision's index.html.
It failed at the Chinese language click during loading: Playwright reported
`<button disabled ... data-language="zh-CN">` and `element is not enabled`.
See [old-version reproduction](old-version-reproduction.txt).
