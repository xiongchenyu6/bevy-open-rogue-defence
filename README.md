# Rogue Commander

以 Bevy 制作、以复刻《刀塔指挥家》玩法为目标的俯视角生存塔防。当前羁绊采用26个家族、101张独立卡，包含实录核实内容与本作扩充；其余英雄、技能和关卡仍包含原型内容，尚未完整复刻原图。角色与场景使用CC0模型。

**[▶ 在瀏覽器直接遊玩](https://xiongchenyu6.github.io/bevy-open-rogue-defence/)**（手機請使用橫屏；入口可切換 EN／簡中，推薦最新版 Chrome／Edge。）

## 核心體驗

- 第 5、10、15、20、25 分鐘各出現一名 Boss；本專案在最終 Boss 後接原創的 60 秒單機「戰果刻印」。公開實戰則是約 10 秒後進入約 10 分鐘、多目標的存檔區，兩者不可混稱。
- `Q` 為 24 層爬塔；公開影片可確認 Lv23 木材 +150。本專案 `W` 有 15 級、每次 10 怪，從一次啟動到下一次啟動為 60 秒；Lv15 顯示每怪 630，最後一怪死亡時原子結算總額 6,300。`F4` 清除挑战怪并关闭自动重试。自动开关在解锁和冷却完成后启动，不再受隐藏DPS或Boss门槛限制。
- `F` 用木材抽羈絆，`G` 優先消耗免費技能點，沒有技能點時改用金幣；公開實戰的首次引導 `F` 約在 00:12，本專案也在 12 秒存入首次免費羈絆抽取。
- 金幣、可消費的殺敵點與木材彼此獨立；黑市以殺敵點購買永久局內裝備。
- 黑市於 03:00 開放，固定顯示 5 格商品、每 180 秒自動換貨；Lv1 首次手動刷新為 60 殺敵。
- 城門耐久歸零即失敗；普通攻擊、32 種技能與各英雄終結技自動清場，終結技不再佔用手動按鈕。

## 構築系統

启动先选择章节，再选择基础R角色。六章保留完整25波、5首领与最终回响，逐章变强；战果碎片可购买永久属性强化并保存，结算可直接下一关或重试。10、20、40级各招募新角色并吞噬旧角色原生三围；旧英雄普攻、被动和自动终结技永久融合，已学技能等级与升华保留；首次招募新英雄增加技能槽，重复招募不刷槽。规则见 [英雄吞噬与继承](docs/hero-fusion.md)。装备和羁绊保留。战意、秘法、神箭术集齐各奖励1枚转职果实，HUD主动使用，确认后消耗。等级上限60。

羁绊抽取独立卡，持有卡不会重复出现。集齐不同成员形成组合，永久保留效果并释放家族槽；完成前置后开放更深的卡池。

当前可走力量、敏捷、智力三条路线：“启程 → 我要玩力量 → 战意”、“启程 → 我要玩敏捷 → 游侠 → 神箭术”、“启程 → 我要玩智力 → 秘法”，再接凡人修仙传 → 灵气、元素大师 → 元素法神 → 毁灭 → 禁术等后继池，共101张独立卡。新池按20／40／50级或8／15／20分钟分阶段开放。游侠组合增加箭矢与穿透；敏捷组合持续成长；判官带免死与复活；登神组合增强此后每次升级收益。扩充路线与适配见 [构筑扩充说明](docs/build-expansion.md)；原始证据、已核实范围和未核实项目见 [参考记录](docs/dota-commander-reference.md)。

角色成长与自动挑战的最新验证见[验收记录](screenshots/career-validation.txt)。此前羁绊与武器验证包含271项测试、15组界面场景和11段录像，见[历史验收记录](screenshots/reference-validation.txt)、[中文战意卡面](screenshots/reference-bond-warpath.png)、[匕首近战](screenshots/attack-daggers.mp4)与[神箭术齐射](screenshots/attack-archery.mp4)。

羁绊集齐会播放图标汇合、金色庆祝和独立音效，显示解锁卡池或转职果实；完成徽章短暂庆祝后收起；按H或书本图标打开完整图鉴回看。见[羁绊完成反馈](docs/bond-completion-feedback.md)。

终结技自动施放，各英雄采用不同的范围、控制、召唤或支援效果及专属视觉表现；HUD不显示大招入口与冷却，无须点击或按空格。见[自动终结技](docs/automatic-ultimates.md)。

战斗技能采用HDR辉光与按状态生成的火焰、冰笼、眩晕、增益符文和病毒孢子。处决印新增真实疫蚀持续伤害与有限传播，见[技能强反馈](docs/skill-spectacle.md)。

技能升级会显示图标与等级变化，骷髅等召唤物随等级平滑长大，技能弹体也逐级增大。见[技能升级反馈](docs/skill-upgrade-feedback.md)。

当前有32个技能、12名英雄。非UR技能满级后可在30／40／50级进行三阶升华（另需对应羁绊或塔层条件）；UR满级后改为三选一独立进化，满槽可确认替换UR技能。十二名英雄的普通攻击分别设计出手、轨迹和命中反馈：六种近战刀光、火焰箭与机械弩矢，以及四系法术弹体。参见[武器表现设计](docs/weapon-presentation.md)。这部分是当前角色表现修正，不冒称原图完整英雄目录。

开局提供力量、敏捷、智力各一名基础角色。后续招募从英雄池抽取6名，排除当前角色；转职不额外授予起始技能。浏览器首次开启会建立新的64-bit seed并写入网址；分享或重开含 `?seed=0x...` 的网址即可重播同一局。

征募介面使用遊戲角色的離線渲染立繪與遺跡背景，點選卡牌先預覽，再按出戰確認。开局显示3张基础角色卡，后续等级/果实招募显示6张，窄直屏自动换行；最新捕捉见[基础开局](screenshots/career-base.png)、[40级转职](screenshots/career-level40.png)与[手机](screenshots/career-level40-mobile.png)。

### 四檔品階

| 品階 | 顏色 | 基礎權重 | 英雄屬性 | 英雄技能威力／冷卻 | 起手技 | 技能威力／冷卻 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| R | 白 | 52 | ×1.00 | ×1.00／×1.00 | Lv1 | ×1.00／×1.00 |
| SR | 藍 | 28 | ×1.12 | ×1.18／×0.92 | Lv2 | ×1.18／×0.95 |
| SSR | 橙 | 15 | ×1.28 | ×1.45／×0.82 | Lv3 | ×1.45／×0.88 |
| UR | 紅 | 5 | ×1.50 | ×1.85／×0.70 | Lv4 | ×1.85／×0.78 |

權重是屬性保底、技能解鎖池與羈絆偏好桶套用前的單一候選權重，不等同畫面最終機率。英雄按 `3／3／3／3` 分布，且每檔各含一名 STR／AGI／INT；技能為 `12／8／6／4`；羁绊按真实单卡品阶配置，不强配四档数量。黑市保留五個內部掉落檔，玩家可見映射統一為 White→R、Blue→SR、Purple／Orange→SSR、Red→UR。

## 手機介面

遊戲畫面頂部的語言按鈕可開啟設定頁，在 English／簡體中文之間即時切換，無需重新開始。選將、技能／羈絆、黑市、HUD 與暫停頁會保留目前狀態並同步翻譯；關閉設定會恢復開啟前的暫停狀態。桌面偏好保存在使用者設定目錄的 `rogue-commander/language`，網頁版使用 localStorage 並同步網址 `lang`，保留原有 seed。環境變數 `ROGUE_COMMANDER_LANG` 或網址 `lang` 可覆蓋已保存偏好。

技能與羈絆使用圖像卡牌、進度格與選中效果預覽；點卡或數字鍵只預覽，確認按鈕／Enter 才套用。重抽、放棄、稍後選擇改為圖標操作，懸停可查看說明。戰鬥 HUD 使用資源圖標、生命／能量条、羈絆圖標列與技能冷卻數字；種子留在暫停／結算，挑戰面板預設收起。


- 以 667×375 以上橫屏為基線，HUD 依安全區縮排；每局 6 名英雄的立繪選將、技能三選一、羈絆替換與黑市在小螢幕改用緊湊網格。
- 桌面預設鍵鼠布局：隱藏虛擬搖桿，底部為緊湊經驗／技能／Shift 閃避列。触屏設備或實際觸摸會啟用搖桿與大按鈕，之後按鍵切回鍵鼠；縮小電腦視窗不會切成手機控件。觸控搖桿鎖定起手 touch ID、套用死區並在失焦／取消時清除，閃避支援多指同按。
- 主要按鈕至少 48 dp，閃避至少 72 dp、終極技 112–128 dp；手機文案不顯示鍵盤快捷鍵。
- 英文與簡體中文覆蓋入口、選將、HUD、技能／羈絆抽取、挑戰、黑市、暫停與結算；網址 `lang` 優先於本機偏好，切換語言會保留同一個 run seed。
- 右上提供暫停／恢復，切出瀏覽器會自動暫停；挑戰面板在手機上收進可開關抽屜。
- 右上以減速、加速、還原三個按鈕控制 1×／2×／4×／8×／16×／32× 戰鬥速度；波次、移動、攻擊、冷卻與特效使用同一套 Bevy 遊戲時間同步加速。切換不會重置 seed，仍可用同一 seed 重開；跨倍速不承諾逐幀一致。
- 公開影片中的選卡與黑市不會暫停戰鬥；本專案為手機可讀性暫停背景戰鬥，屬明確的易用性改編。
- 第一版同時保留鍵盤／滑鼠操作，便於桌面開發與測試。

## 造型素材驗證

角色與場景已替換為具體造型素材，角色具備待機、行走、攻擊與受擊動作。查看[全角色總覽](screenshots/model-cast-review.png)、[桌面實戰](screenshots/models-desktop.png)、[844×390 手機尺寸](screenshots/models-mobile.png)及[6 秒／30 FPS 動畫錄製](screenshots/models-animation.mp4)。手機尺寸截圖是桌面 GPU 的離屏渲染，不代表 Android 真機效能。

此前模型替換驗收：249/249 測試與 Clippy 零警告通過，包含動作時長、暫停和池化重用回歸；驗收命令與結果見 [model-validation.txt](screenshots/model-validation.txt)，素材結構驗證見 [model-verification.json](screenshots/model-verification.json)；重建與來源說明見 [SOURCES.md](assets/models/SOURCES.md)。

## 技術結構

單一 Cargo package，`src/main.rs` 啟動 `src/lib.rs`；玩法拆為 `arena`、`environment`、`waves`、`combat`、`bosses`、`archive`、`skills`、`bonds`、`progression`、`trial`、`market`、`hero`、`hero_abilities`、`pooling`、`debug_menu`、`audio`、`hit_feedback`、`mobile_input` 與 `ui` plugins。完整原創內容表位於 `src/game/content.rs`；公開資料與改編邊界見 [`docs/research-notes.md`](docs/research-notes.md) 和 [`docs/gameplay-spec.md`](docs/gameplay-spec.md)。

## 已完成與待完成

- [x] **主循環：** 橫向怪道、自動戰鬥、移動、城門勝敗、25 分鐘與五 Boss 時間軸。
- [x] **構築：** 30个原型技能、10个家族与30张独立羁绊卡、雙挑戰、基础R三选一开局、10/20/40级及果实转职、技能／羈絆／黑市四檔品階、固定五格黑市與三資源。
- [x] **局次隨機：** 每次開局自動產生可分享 seed；選將、技能與羈絆按 52／28／15／5 基礎權重抽取，起手技能、波次與黑市也受 seed 驅動，選將不消耗戰鬥 RNG。
- [x] **手機基線：** 667×375 至 915×412 響應式 HUD、固定多指搖桿、獨立右手動作、觸控選卡／市場、暫停與 Android GameActivity 工程骨架。
- [x] **戰鬥加速：** 右上三個按鈕減速、加速及還原，倍速為 1×／2×／4×／8×／16×／32×，桌面按 `T` 加速；普攻會補齊跨幀週期，超出單幀預算的攻擊順延但不丟失，敵人抵達半徑會當幀結算，投射物採 swept collision，避免高倍速或低 FPS 時漏射、折返與穿怪。
- [x] **Web 版本：** WebGL2/WASM、WAV／OGG 解碼、觸控安全 Canvas、約 11 MB gzip 串流載入、raw WASM 相容後備與 GitHub Pages 自動部署。
- [x] **雙語與字庫：** EN／簡中入口與完整正式 UI；WASM／Android 共用 347 KB Noto Sans SC 子集，字形回歸測試會覆蓋所有目前玩家可見中英文字符。
- [x] **此前原型運行驗證（不代表新羁绊平衡验收）：** `0xA11CE` 正常構築連跑兩次，均擊敗五 Boss、完成 Archive 並在第 50,979 幀勝利；30 個 checkpoint、RNG 狀態與最終 PNG 完全一致。15.5 秒桌面、19.4 秒手機基線、18.0 秒選將／重抽、19.5 秒四檔品階與本輪 18.0 秒戰鬥加速影片均已回看。
- [x] **效能工程：** 敵人、20 個共享上限召喚物、256 個可見投射物、彈性無 mesh 邏輯投射物、128 個短期特效與 64 個傷害數字均集中池化；視覺池滿不會丟失敵人壓力或傷害。
- [x] **內容語義：** 30 个技能、30 张独立羁绊卡与19件永久装备接入运行逻辑；震動、完整教學和局外存檔仍屬後續手機功能。
- [ ] **擴大平衡：** 已驗證的 Storm 構築以 1/1,370 城門耐久貼線通關；四檔倍率、12 英雄各自的完整長局、30% 以上通關餘量與 N2+ 尚待調整。
- [ ] **Android：** 接受 Google SDK 授權並補齊 SDK／NDK、產生 APK、真機效能測試；目前授權與工具鏈仍是阻塞項。

效能目標為中階 Android 裝置 60 FPS、壓力場景最低 30 FPS。普通波次同屏邏輯上限為 130；敵人池另保留 30 個活躍槽給 Q/W、Boss、Boss 召喚與 Archive，因此全域活躍上限為 160。

## 開發命令

進入 Nix shell 後，使用 `cargo run` 啟動、`cargo fmt` 格式化、`cargo check --all-targets` 快速檢查、`cargo test --all-targets` 執行測試、`cargo clippy --all-targets -- -D warnings` 執行靜態檢查。Bevy 與相關 crates 鎖定為 0.19.0。

```bash
nix develop
ROGUE_COMMANDER_SEED=0xA11CE cargo run
ROGUE_COMMANDER_LANG=zh-CN ROGUE_COMMANDER_SEED=0x1 cargo run
cargo run --bin capture -- still screenshots/draft-desktop.png 120 draft 0xA11CE 1280x720
cargo run --bin capture -- still screenshots/draft-mobile.png 120 draft-preview 0xA11CE 844x390
cargo run --release --bin capture -- simulate screenshots/balance-25m.png 60000 balance 0xA11CE
```

捕捉參數依次為模式、輸出、30 FPS 幀數、場景、十進位／十六進位種子與可選的 `WIDTHxHEIGHT`。`draft`、`draft-preview`、`draft-confirm` 分別捕捉初選、換人預覽和點卡直接出戰；harness 同時驗證暫停／出戰狀態與按鈕邊界。遊戲內按 `T` 逐級加速至32×，減速與還原使用右上按鈕；按 `F7` 開關生成器，`Tab` 切換普通怪／精英／Boss，`[`／`]`（或數字鍵盤 4／6）選目標，精英模式用 `-`／`=`（或 2／8）選詞綴，`Enter` 生成。暫停或結算期間禁止生成。

## Web 與 GitHub Pages

`main` 分支每次推送都會由 [`.github/workflows/pages.yml`](.github/workflows/pages.yml) 建置並部署 GitHub Pages。工作流固定使用 `wasm-bindgen-cli 0.2.126`，只編譯實際遊戲 binary，不會把離屏捕捉工具帶進 WASM。現代瀏覽器下載約 11 MB 的 gzip sidecar；不支援 `DecompressionStream` 時會自動改用 raw WASM。

本機重現建置與預覽：

```bash
cargo install wasm-bindgen-cli --version 0.2.126 --locked
bash scripts/build-wasm.sh
python3 -m http.server 8000 --directory dist
```

開啟 `http://127.0.0.1:8000/`，點擊「開始遊戲」以解鎖瀏覽器音訊並載入 Bevy。產物位於忽略追蹤的 `dist/`；47 個模型、6 個音訊、中文字庫與 OFL 授權會一併複製。網址不帶 `seed` 時會自動產生新局；例如 `http://127.0.0.1:8000/?seed=0xA11CE&lang=zh-CN` 可固定以簡中重播。語言優先順序為網址、`localStorage`、瀏覽器偏好，切換只更新 `lang`，不會改動 `seed`。

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

目前桌面與瀏覽器版本可實際運行：右出怪左守門、資料化 25 波、12 種獨立敵人、5 名機制 Boss、30个原型技能、10个家族与30张独立羁绊卡、12英雄、10/20/40级招募与吞噬转职、Q/W 雙挑戰、固定五格黑市、原創 60 秒戰果刻印、EN／簡中鍵盤／觸控 HUD、1×／2×／4×／8×／16×／32× 戰鬥加速、F7 生成器及離屏捕捉工具均已接入。每個品階各有一名力量、敏捷與智力英雄；开局3名基础R角色，后续6选1招募使用品阶权重，均可由URL seed重播，技能、羈絆與黑市也使用同一套白／藍／橙／紅可見色制。Boss 的通用減傷、元素抗性、火抗與冰霜易傷統一在最終投射物傷害路徑結算。35 個角色 GLB（12 英雄＋12 敵人＋5 Boss＋6 召喚物）已改用 KayKit／Quaternius 的 CC0 造型素材，保留骨骼動畫；投石塔使用機械臂動作。英雄以武器、配色區分，場景使用 12 個 KayKit 城牆、火炬、碎石與聖物資產。來源與授權見 [`assets/models/SOURCES.md`](assets/models/SOURCES.md)；`tools/import_cast.py` 保留 Idle／Walk／Attack／Hit 四組動作並移除未使用資料，`tools/verify_models.py` 驗證實際載入清單、動畫、紋理與 buffer。6 個合成音訊仍由倉庫腳本產生。


替換美術前的玩法驗收基線通過格式檢查、原生與 wasm32 `cargo check`、246/246 測試與 Clippy 零警告；音訊測試會逐一建立 1 個 OGG 與 5 個 WAV 的解碼器，避免 Web 版再次因未編入格式支援而崩潰。字庫測試會解析實際 TTF cmap，缺少任何 UI 字符或子集意外膨脹都會直接失敗。正常 `balance 0xA11CE` 從預設 48 攻擊、1.95 攻速與 1,000 城門開始，使用正常抽卡、Q/W 判定及每代最多兩件的黑市自購；兩次均在遊戲時間 1,572 秒後以 5/5 Boss、1,974 累計擊殺、Q14、W13、Archive `TimedOut`、城門 1/1,370 勝利。兩份 30-checkpoint 記錄及最終截圖 SHA-256 均一致。

以下为此前原型版本的历史验证，未覆盖本次基础开局与转职变更。上述長局池峰值為敵人 24、投射物 161、傷害數字 63、召喚物 5；敵人、投射物及召喚物邏輯拒絕均為 0。128 格 FX 池拒絕 261 次純視覺命中特效，沒有改變傷害。桌面驗收見 [完整長局結果](screenshots/balance-25m.png)、[15.5 秒 Boss→Archive 實機影片](screenshots/rogue-commander-final.mp4)與[逐秒回看表](screenshots/rogue-commander-final-review.png)；手機基線見 [19.4 秒多指操作影片](screenshots/rogue-commander-mobile.mp4)與[回看表](screenshots/rogue-commander-mobile-review.png)。[18.0 秒 12 英雄選將影片](screenshots/rogue-commander-roster.mp4)及[回看表](screenshots/rogue-commander-roster-review.png)為 800×390、25 FPS，涵蓋 6 選 1、免費重抽與 Solar 實戰；[19.5 秒四檔品階實機影片](screenshots/rogue-commander-rarity.mp4)及[回看表](screenshots/rogue-commander-rarity-review.png)為 844×390、30 FPS，`0x1` 可見四檔英雄。最新 [19.0 秒簡中實機影片](screenshots/rogue-commander-zh.mp4)及[回看表](screenshots/rogue-commander-zh-review.png)同為 844×390、H.264/yuv420p，展示簡中選將、UR 英雄 HUD、技能卡與 R／SSR／SR 羈絆卡；EN↔簡中切換前後 seed 均為 `0x0000000000000001`，中文沒有缺字方框。本輪新增的 [18.0 秒戰鬥加速實機影片](screenshots/rogue-commander-speed.mp4)及[回看表](screenshots/rogue-commander-speed-review.png)為 844×390、H.264/yuv420p，連續展示當時的 1×／2×／3× 高亮、戰鬥推進與無遮擋的手機 HUD。所有影片已完整解碼及回看，本輪 JavaScript／WASM console 錯誤與例外均為 0。

尚未完成的是四檔倍率與 12 英雄逐一長局平衡、把全域 RNG 再拆成互不干擾的子流、震動與局外存檔，以及 Android SDK 授權、APK 和真機 FPS；不得把桌面確定性驗收寫成 Android 效能證明。

## 素材表

| 素材 | 狀態 | 遊戲內尺寸 | 路徑／方案 | 成本 |
| --- | --- | --- | --- | --- |
| 12 名英雄低模 | 已接入 | 約 2.2–2.7 m 高 | `assets/models/hero_*.glb`；12 種輪廓、武器、獨立配色與 emissive 強調 | 0 |
| 12 類敵人／精英 | 已接入 | 約 0.6–2.4 m 高 | `assets/models/enemy_*.glb`；12 種獨立輪廓與資料化行為，11 種 emissive 強調 | 0 |
| 5 名 Boss | 已接入 | 約 1.9–3.9 m 高 | `assets/models/boss_*.glb`；5 種獨立輪廓與 emissive 強調 | 0 |
| 6 種召喚物低模 | 已接入 | 約 0.5–2.8 m 高 | `assets/models/summon_*.glb`；炮塔、靈體、巨像等專屬輪廓與 emissive 強調 | 0 |
| 城門、怪道與裂隙 | 已接入 | 場地 48×30 m | `environment` 程序化美術層：堡壘、旗幟、符文、腐化地表、巨石、植被與火盆 | 0 |
| 攻擊與範圍特效 | 已接入 | 0.3–8.4 m | Bevy Sphere／Torus | 0 |
| 品階 UI 配色 | 已接入 | 卡框／標籤 | Bevy 程式化白／藍／橙／紅 | 0 |
| 造型素材匯入 | 已接入 | 35 個動畫角色＋12 個場景 GLB | `tools/import_cast.py`、`tools/import_environment_models.py`、`tools/verify_models.py`；保留造型、紋理與骨骼，四組動作與嵌入資源校驗 | 0 |
| 貼圖／骨骼動畫 | 已接入 | 35 個角色 | CC0 模型內嵌貼圖與 Idle／Walk／Attack／Hit 動作 | 0 |
| 英雄立繪／征募背景 | 已接入 | 12 張 384×480／1600×1000 | `assets/ui/`；由遊戲模型離線渲染，來源與重建命令見 `assets/ui/SOURCES.md` | 0 |
| 簡中 UI 字體 | 已接入 | 369 KB／隨 UI 文案產生子集 | `assets/fonts/rogue_commander_ui.ttf`；Noto Sans SC 子集，SIL OFL 1.1 | 0 |
| 技能／羈絆／操作圖示 | 已接入 | 56 張 256×256 RGBA | `assets/ui/icons/`；Blender 造型渲染，`tools/verify_ui_assets.py` 依內容表校驗 | 0 |
| 音效與環境循環 | 已接入 | 0.16–24 秒 | `assets/audio/`；1 OGG＋13 WAV（含命中、暴擊、擊殺、重擊殺、英雄受擊、城門受擊），含 Web 解碼回歸測試 | 0 |

角色、場景與征募立繪均使用 CC0 模型；立繪和遺跡背景由本地 Blender 渲染，沒有付費生成。選將先預覽英雄與起手技能，再確認出戰。

英雄屬性面板、三按鈕倍速與回旋刃外觀詳見 [操作與數值說明](docs/hero-attributes-and-speed.md)。

UR满级后可选择一条独立进化路线，详见 [UR分路与技能表现](docs/ur-evolution.md)；完整关卡与永久成长详见 [战役成长](docs/campaign.md)。
