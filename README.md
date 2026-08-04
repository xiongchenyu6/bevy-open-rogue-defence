# Rogue Commander

一款以 Bevy 製作、面向手機的原創俯視角生存塔防。敵人從右側裂隙沿橫向戰線進攻左側城門；英雄自動攻擊，玩家透過走位、雙挑戰與多條三選一構築線守住 25 分鐘。玩法節奏參考《刀塔指揮家》的公開資料，但名稱、美術、角色、公式與內容均為原創。

**[▶ 在瀏覽器直接遊玩](https://xiongchenyu6.github.io/bevy-open-rogue-defence/)**（手機請使用橫屏；推薦最新版 Chrome／Edge。）

## 核心體驗

- 第 5、10、15、20、25 分鐘各出現一名 Boss；本專案在最終 Boss 後接原創的 60 秒單機「戰果刻印」。公開實戰則是約 10 秒後進入約 10 分鐘、多目標的存檔區，兩者不可混稱。
- `Q` 為 24 層爬塔；公開影片可確認 Lv23 木材 +150。本專案 `W` 有 15 級、每次 10 怪，從一次啟動到下一次啟動為 60 秒；Lv15 顯示每怪 630，最後一怪死亡時原子結算總額 6,300。`F4` 清除挑戰怪。
- `F` 用木材抽羈絆，`G` 優先消耗免費技能點，沒有技能點時改用金幣；公開實戰的首次引導 `F` 約在 00:12，本專案也在 12 秒存入首次免費羈絆抽取。
- 金幣、可消費的殺敵點與木材彼此獨立；黑市以殺敵點購買永久局內裝備。
- 黑市於 03:00 開放，固定顯示 5 格商品、每 180 秒自動換貨；Lv1 首次手動刷新為 60 殺敵。
- 城門耐久歸零即失敗；普通攻擊、30 種具專屬運行語義的技能與右側終極技會自動／手動協同清場。

## 構築系統

首版資料庫包含 30 個專屬技能、10 套完整 2／4／6 階羈絆與四種元素反應，涵蓋三條主要流派：

- **風暴**：連鎖、冷卻、技能暴擊，擅長清理密集怪群。
- **烈焰**：燃燒、破甲、爆發，擅長精英與首領。
- **召喚**：守衛、攔截、持續輸出，操作壓力最低。

最多同時保留 7 套未完成羈絆；每套在 2／4／6 點觸發階級效果，6 點時吞噬為永久效果並釋放槽位。滿 7 套時選到新羈絆會開啟 7 張替換面板，扣款只結算一次。12 名英雄各有初始屬性、被動與終極技，並在局內進行 R／SR／SSR／UR 進化。

每局會從 12 名英雄中抽出 6 名，保證力量、敏捷、智力至少各一名；每張英雄卡再從 3 個主題技能中抽取一個起手技，並提供一次免費重抽。瀏覽器首次開啟會建立新的 64-bit seed 並寫入網址；分享或重開含 `?seed=0x...` 的網址即可重播同一局。

## 手機介面

- 以 667×375 以上橫屏為基線，HUD 依安全區縮排；每局 6 名英雄的 3×2 選將、技能三選一、羈絆替換與黑市在小螢幕改用緊湊網格。
- 左側為可見的固定觸控搖桿，會鎖定起手 touch ID、套用死區並在失焦／取消時清除；右側閃避與終極技可和移動真正多指同按。
- 主要按鈕至少 48 dp，閃避至少 72 dp、終極技 112–128 dp；手機文案不顯示鍵盤快捷鍵。
- 右上提供暫停／恢復，切出瀏覽器會自動暫停；挑戰面板在手機上收進可開關抽屜。
- 公開影片中的選卡與黑市不會暫停戰鬥；本專案為手機可讀性暫停背景戰鬥，屬明確的易用性改編。
- 第一版同時保留鍵盤／滑鼠操作，便於桌面開發與測試。

## 技術結構

單一 Cargo package，`src/main.rs` 啟動 `src/lib.rs`；玩法拆為 `arena`、`waves`、`combat`、`bosses`、`archive`、`skills`、`bonds`、`progression`、`trial`、`market`、`hero`、`hero_abilities`、`pooling`、`debug_menu`、`audio`、`mobile_input` 與 `ui` plugins。完整原創內容表位於 `src/game/content.rs`；公開資料與改編邊界見 [`docs/research-notes.md`](docs/research-notes.md) 和 [`docs/gameplay-spec.md`](docs/gameplay-spec.md)。

## 已完成與待完成

- [x] **主循環：** 橫向怪道、自動戰鬥、移動、城門勝敗、25 分鐘與五 Boss 時間軸。
- [x] **構築：** 30 個專屬技能、10 套完整羈絆、四種元素反應、雙挑戰、12 英雄的 6 選 1／進化、固定五格黑市與三資源。
- [x] **局次隨機：** 每次開局自動產生可分享 seed；選將、起手技能、抽卡、波次與黑市均受 seed 驅動，選將免費重抽不消耗戰鬥 RNG。
- [x] **手機基線：** 667×375 至 915×412 響應式 HUD、固定多指搖桿、獨立右手動作、觸控選卡／市場、暫停與 Android GameActivity 工程骨架。
- [x] **Web 版本：** WebGL2/WASM、WAV／OGG 解碼、觸控安全 Canvas、約 11 MB gzip 串流載入、raw WASM 相容後備與 GitHub Pages 自動部署。
- [x] **運行驗證：** `0xA11CE` 正常構築連跑兩次，均擊敗五 Boss、完成 Archive 並在第 50,979 幀勝利；30 個 checkpoint、RNG 狀態與最終 PNG 完全一致。15.5 秒桌面、19.4 秒手機基線與本輪 18.0 秒選將／重抽影片均已回看。
- [x] **效能工程：** 敵人、20 個共享上限召喚物、256 個可見投射物、彈性無 mesh 邏輯投射物、128 個短期特效與 64 個傷害數字均集中池化；視覺池滿不會丟失敵人壓力或傷害。
- [x] **內容語義：** 30 個技能、10 套羈絆、四種反應與 19 件永久裝備均有運行語義；震動、完整教學和局外存檔仍屬後續手機功能。
- [ ] **擴大平衡：** 已驗證的 Storm 構築以 1/1,370 城門耐久貼線通關；12 英雄各自的完整長局、30% 以上通關餘量與 N2+ 尚待調整。
- [ ] **Android：** 接受 Google SDK 授權並補齊 SDK／NDK、產生 APK、真機效能測試；目前授權與工具鏈仍是阻塞項。

效能目標為中階 Android 裝置 60 FPS、壓力場景最低 30 FPS。普通波次同屏邏輯上限為 130；敵人池另保留 30 個活躍槽給 Q/W、Boss、Boss 召喚與 Archive，因此全域活躍上限為 160。

## 開發命令

進入 Nix shell 後，使用 `cargo run` 啟動、`cargo fmt` 格式化、`cargo check --all-targets` 快速檢查、`cargo test --all-targets` 執行測試、`cargo clippy --all-targets -- -D warnings` 執行靜態檢查。Bevy 與相關 crates 鎖定為 0.19.0。

```bash
nix develop
ROGUE_COMMANDER_SEED=0xA11CE cargo run
cargo run --bin capture -- still screenshots/showcase.png 300 showcase 0xA11CE
cargo run --release --bin capture -- simulate screenshots/balance-25m.png 60000 balance 0xA11CE
```

捕捉參數依次為模式、輸出、30 FPS 幀數、場景與十進位／十六進位種子。遊戲內按 `F7` 開關生成器；`Tab` 切換普通怪／精英／Boss，`[`／`]`（或數字鍵盤 4／6）選目標，精英模式用 `-`／`=`（或 2／8）選詞綴，`Enter` 生成。暫停或結算期間禁止生成。

## Web 與 GitHub Pages

`main` 分支每次推送都會由 [`.github/workflows/pages.yml`](.github/workflows/pages.yml) 建置並部署 GitHub Pages。工作流固定使用 `wasm-bindgen-cli 0.2.126`，只編譯實際遊戲 binary，不會把離屏捕捉工具帶進 WASM。現代瀏覽器下載約 11 MB 的 gzip sidecar；不支援 `DecompressionStream` 時會自動改用 raw WASM。

本機重現建置與預覽：

```bash
cargo install wasm-bindgen-cli --version 0.2.126 --locked
bash scripts/build-wasm.sh
python3 -m http.server 8000 --directory dist
```

開啟 `http://127.0.0.1:8000/`，點擊「開始遊戲」以解鎖瀏覽器音訊並載入 Bevy。產物位於忽略追蹤的 `dist/`；35 個模型與 6 個音訊會一併複製。網址不帶 `seed` 時會自動產生新局；例如 `http://127.0.0.1:8000/?seed=0xA11CE` 可固定重播。

## Android（GameActivity）

`android/` 採 Bevy 0.19 官方 GameActivity 結構，原生函式庫、Manifest metadata 與 Java 載入名稱均為 `rogue_commander`。目前只建置 `arm64-v8a`，最低支援 Android 8（API 26），並把根目錄 `assets/` 打包進 APK。

本機已有 JDK 21，但尚缺 Android SDK／NDK、`aarch64-linux-android` Rust 標準庫、`cargo-ndk`、ADB 與 Gradle 8.6；這些工具未由專案自動下載。Nix 可提供 Android SDK，但首次使用前必須由使用者明確接受 Google Android SDK 授權。安裝 Android Studio 或等效工具鏈後，設定 `ANDROID_SDK_ROOT` 與 `ANDROID_NDK_ROOT`，再執行：

```bash
rustup target add aarch64-linux-android
cargo install cargo-ndk
cargo ndk -t arm64-v8a -P 26 -o android/app/src/main/jniLibs build
gradle -p android assembleDebug
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n com.roguecommander.game/.MainActivity
adb logcat | grep 'RustStdoutStderr\|bevy\|wgpu'
```

工具鏈與授權已就緒後，也可直接執行 `tools/build_android.sh`；腳本會先確認 SDK、NDK、Rust Android 標準庫、`cargo-ndk` 與 Gradle，再輸出 Debug APK 的實際路徑。

目前 Nix shell 使用 Nix 提供的 Rust，沒有 `rustup`；若繼續沿用它，需在 shell 中另行加入 Android target standard library。專案不提交 Gradle wrapper binary，可直接使用 Gradle 8.6，或在工具鏈就緒後執行 `gradle -p android wrapper --gradle-version 8.6` 產生 wrapper。也可用 Android Studio 開啟 `android/`。每次修改 Rust 程式後，必須先重跑 `cargo ndk` 再組裝 APK。Release APK 尚未設定簽章。

## 專案狀態

目前桌面與瀏覽器版本可實際運行：右出怪左守門、資料化 25 波、12 種獨立敵人、5 名機制 Boss、30 個專屬技能、10 套完整羈絆、四種元素反應、12 英雄與三段進化、Q/W 雙挑戰、固定五格黑市、原創 60 秒戰果刻印、鍵盤／觸控 HUD、F7 生成器及離屏捕捉工具均已接入。12 英雄以力量／敏捷／智力各 4 名分布；每局 6 選 1、一次免費重抽與主題起手技都可由 URL seed 重播。Boss 的通用減傷、元素抗性、火抗與冰霜易傷統一在最終投射物傷害路徑結算。35 個原創 GLB（12 英雄＋12 敵人＋5 Boss＋6 召喚物）與 6 個原創合成音訊均由倉庫內腳本產生；模型腳本會逐一重匯入 35 檔並驗證靜態 mesh、落地樞軸與 PBR 材質。

驗收基線通過格式檢查、原生與 wasm32 `cargo check`、212/212 測試與 Clippy 零警告；音訊測試會逐一建立 1 個 OGG 與 5 個 WAV 的解碼器，避免 Web 版再次因未編入格式支援而崩潰。正常 `balance 0xA11CE` 從預設 48 攻擊、1.95 攻速與 1,000 城門開始，使用正常抽卡、Q/W 判定及每代最多兩件的黑市自購；兩次均在遊戲時間 1,572 秒後以 5/5 Boss、1,974 累計擊殺、Q14、W13、Archive `TimedOut`、城門 1/1,370 勝利。兩份 30-checkpoint 記錄及最終截圖 SHA-256 均一致。

本輪池峰值為敵人 24、投射物 161、傷害數字 63、召喚物 5；敵人、投射物及召喚物邏輯拒絕均為 0。128 格 FX 池拒絕 261 次純視覺命中特效，沒有改變傷害。桌面驗收見 [完整長局結果](screenshots/balance-25m.png)、[15.5 秒 Boss→Archive 實機影片](screenshots/rogue-commander-final.mp4)與[逐秒回看表](screenshots/rogue-commander-final-review.png)；手機基線見 [19.4 秒多指操作影片](screenshots/rogue-commander-mobile.mp4)與[回看表](screenshots/rogue-commander-mobile-review.png)。本輪新增的 [18.0 秒 12 英雄選將影片](screenshots/rogue-commander-roster.mp4)及[回看表](screenshots/rogue-commander-roster-review.png)為 800×390、25 FPS、H.264/yuv420p，涵蓋 6 選 1、免費重抽、Solar、搖桿移動、專屬終極技與顯示完整 seed 的暫停畫面；逐秒回看無灰邊、黑畫面或瀏覽器錯誤。

尚未完成的是 12 英雄逐一長局平衡、把全域 RNG 再拆成互不干擾的子流、中文／圖示資產、震動與局外存檔，以及 Android SDK 授權、APK 和真機 FPS；不得把桌面確定性驗收寫成 Android 效能證明。

## 素材表

| 素材 | 狀態 | 遊戲內尺寸 | 路徑／方案 | 成本 |
| --- | --- | --- | --- | --- |
| 12 名英雄低模 | 已接入 | 約 2.2–2.7 m 高 | `assets/models/hero_*.glb`；12 種輪廓、武器與獨立配色 | 0 |
| 12 類敵人／精英 | 已接入 | 約 0.6–2.4 m 高 | `assets/models/enemy_*.glb`；12 種獨立輪廓與資料化行為 | 0 |
| 5 名 Boss | 已接入 | 約 1.9–3.9 m 高 | `assets/models/boss_*.glb`；5 種獨立輪廓 | 0 |
| 6 種召喚物低模 | 已接入 | 約 0.5–2.8 m 高 | `assets/models/summon_*.glb`；炮塔、靈體、巨像等專屬輪廓 | 0 |
| 城門、怪道與裂隙 | 已接入 | 場地 48×30 m | Bevy 程序化 mesh／material | 0 |
| 攻擊與範圍特效 | 已接入 | 0.3–8.4 m | Bevy Sphere／Torus | 0 |
| 低模重建腳本 | 已接入 | 35 個 GLB | `tools/generate_models.py`；35 檔均逐一重匯入驗證靜態 mesh／PBR／地面樞軸 | 0 |
| 貼圖／骨骼動畫 | 未製作 | 1.8–5 m 高 | 首版採低模純材質美術；若改用付費生成會先確認費用 | 0 |
| UI 圖示與中文字體 | 未選定 | 48–128 px | 優先採可商用開源素材 | 0 |
| 音效與環境循環 | 已接入 | 0.16–24 秒 | `assets/audio/`；1 OGG＋5 WAV，含 Web 解碼回歸測試 | 0 |

任何付費生成素材都會在首次產生前另行確認費用。
