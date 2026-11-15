//! Keep combat asset reads dormant while the player is choosing an expedition.
//! Handles remain real AssetServer handles; the gate delays IO, not identity.
#[cfg(any(target_arch = "wasm32", test))]
use std::{
    future::poll_fn,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Poll, Waker},
};

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Default)]
struct ReadGate {
    open: AtomicBool,
    waiters: Mutex<Vec<Waker>>,
}
#[cfg(any(target_arch = "wasm32", test))]
impl ReadGate {
    async fn wait(&self) {
        poll_fn(|context| {
            let mut waiters = self.waiters.lock().unwrap();
            if self.open.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                if !waiters.iter().any(|w| w.will_wake(context.waker())) {
                    waiters.push(context.waker().clone());
                }
                Poll::Pending
            }
        })
        .await
    }
    fn release(&self) {
        self.open.store(true, Ordering::Release);
        for waker in self.waiters.lock().unwrap().drain(..) {
            waker.wake();
        }
    }
}
#[cfg(any(target_arch = "wasm32", test))]
fn menu_asset(path: &std::path::Path) -> bool {
    ["fonts", "ui/icons", "ui/controls", "data"]
        .iter()
        .any(|root| path.starts_with(root))
}

#[cfg(any(target_arch = "wasm32", test))]
use bevy::asset::io::{AssetReader, AssetReaderError, PathStream, Reader};
#[cfg(any(target_arch = "wasm32", test))]
use std::path::Path;
#[cfg(any(target_arch = "wasm32", test))]
struct DemandReader<R> {
    inner: R,
    gate: std::sync::Arc<ReadGate>,
}
#[cfg(any(target_arch = "wasm32", test))]
impl<R: AssetReader> AssetReader for DemandReader<R> {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        if !menu_asset(path) {
            self.gate.wait().await;
        }
        self.inner.read(path).await
    }
    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        if !menu_asset(path) {
            self.gate.wait().await;
        }
        self.inner.read_meta(path).await
    }
    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.inner.read_directory(path).await
    }
    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        self.inner.is_directory(path).await
    }
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::*;
    use bevy::{
        asset::io::{AssetSourceBuilder, AssetSourceId, wasm::HttpWasmAssetReader},
        prelude::*,
    };
    use js_sys::wasm_bindgen::JsCast;
    use std::sync::Arc;

    #[derive(Resource)]
    struct DemandGate(Arc<ReadGate>);
    /// Must run before AssetPlugin builds the default source.
    pub(crate) fn install(app: &mut App) {
        let gate = Arc::new(ReadGate::default());
        let reader_gate = gate.clone();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(DemandReader {
                    inner: HttpWasmAssetReader::new("assets"),
                    gate: reader_gate.clone(),
                })
            }),
        )
        .insert_resource(DemandGate(gate))
        .add_systems(Update, complete_chapter_request);
    }
    pub(crate) fn request_chapter(stage: usize) -> bool {
        let Some(window) = web_sys::window() else {
            return false;
        };
        let Ok(callback) = js_sys::Reflect::get(window.as_ref(), &"roguePrepareChapter".into())
        else {
            return false;
        };
        let Some(callback) = callback.dyn_ref::<js_sys::Function>() else {
            return false;
        };
        callback
            .call1(window.as_ref(), &(stage as u32).into())
            .is_ok()
    }
    pub(crate) fn open_downloads() {
        let Some(window) = web_sys::window() else {
            return;
        };
        if let Ok(callback) = js_sys::Reflect::get(window.as_ref(), &"rogueOpenDownloads".into())
            && let Some(callback) = callback.dyn_ref::<js_sys::Function>()
        {
            let _ = callback.call0(window.as_ref());
        }
    }
    pub(crate) fn release_chapter() {
        let Some(window) = web_sys::window() else {
            return;
        };
        if let Ok(callback) = js_sys::Reflect::get(window.as_ref(), &"rogueReleaseChapter".into())
            && let Some(callback) = callback.dyn_ref::<js_sys::Function>()
        {
            let _ = callback.call0(window.as_ref());
        }
    }
    pub(crate) fn downloads_open() -> bool {
        web_sys::window()
            .and_then(|window| {
                js_sys::Reflect::get(window.as_ref(), &"rogueDownloadsOpen".into()).ok()
            })
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }
    fn complete_chapter_request(
        gate: Res<DemandGate>,
        mut request: ResMut<super::super::RunRequest>,
        mut menu: ResMut<super::super::CampaignMenu>,
        progress: Res<super::super::CampaignProgress>,
        session: Res<super::super::GameSession>,
    ) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(value) = js_sys::Reflect::get(window.as_ref(), &"rogueChapterDownload".into())
        else {
            return;
        };
        let Some(stage) = js_sys::Reflect::get(&value, &"stage".into())
            .ok()
            .and_then(|n| n.as_f64())
        else {
            return;
        };
        if stage.fract() != 0.0 || stage < 0.0 || stage >= super::super::CHAPTER_COUNT as f64 {
            return;
        }
        let stage = stage as usize;
        let status = js_sys::Reflect::get(&value, &"status".into())
            .ok()
            .and_then(|s| s.as_string());
        match status.as_deref() {
            Some("ready") if (menu.0 || session.finished) && progress.can_play(stage) => {
                // Consume before dispatch so repeated frames cannot reset a run.
                let _ = js_sys::Reflect::set(
                    window.as_ref(),
                    &"rogueChapterDownload".into(),
                    &js_sys::wasm_bindgen::JsValue::NULL,
                );
                gate.0.release();
                request.0 = Some(stage);
                menu.0 = false;
            }
            _ => {}
        }
    }
}
#[cfg(target_arch = "wasm32")]
pub(crate) use browser::{
    downloads_open, install, open_downloads, release_chapter, request_chapter,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Waker},
    };
    #[test]
    fn menu_assets_are_available_without_releasing_combat_reads() {
        for path in [
            "fonts/rogue_commander_ui.ttf",
            "ui/icons/castle.png",
            "ui/controls/settings.png",
            "data/campaign.json",
        ] {
            assert!(menu_asset(std::path::Path::new(path)));
        }
        for path in [
            "models/hero_storm_savant.glb",
            "audio/shot.wav",
            "ui/market/equipment-atlas.png",
            "ui/draft_backdrop.png",
            "ui/portraits/hero_storm_savant.png",
        ] {
            assert!(!menu_asset(std::path::Path::new(path)));
        }
    }
    #[test]
    fn combat_reader_stays_pending_until_explicit_chapter_release() {
        let gate = ReadGate::default();
        let mut read = pin!(gate.wait());
        let mut context = Context::from_waker(Waker::noop());
        assert!(read.as_mut().poll(&mut context).is_pending());
        assert!(read.as_mut().poll(&mut context).is_pending());
        assert_eq!(gate.waiters.lock().unwrap().len(), 1);
        gate.release();
        assert!(read.as_mut().poll(&mut context).is_ready());
        let mut later = pin!(gate.wait());
        assert!(later.as_mut().poll(&mut context).is_ready());
    }
    #[test]
    fn held_combat_read_does_not_start_io_or_block_menu_io() {
        use bevy::asset::io::VecReader;
        use std::sync::{Arc, atomic::AtomicUsize};
        struct CountingReader(Arc<AtomicUsize>);
        impl AssetReader for CountingReader {
            async fn read<'a>(&'a self, _: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(VecReader::new(vec![1, 2, 3]))
            }
            async fn read_meta<'a>(
                &'a self,
                path: &'a Path,
            ) -> Result<impl Reader + 'a, AssetReaderError> {
                self.read(path).await
            }
            async fn read_directory<'a>(
                &'a self,
                path: &'a Path,
            ) -> Result<Box<PathStream>, AssetReaderError> {
                Err(AssetReaderError::NotFound(path.to_owned()))
            }
            async fn is_directory<'a>(&'a self, _: &'a Path) -> Result<bool, AssetReaderError> {
                Ok(false)
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(ReadGate::default());
        let reader = DemandReader {
            inner: CountingReader(calls.clone()),
            gate: gate.clone(),
        };
        let mut combat = pin!(reader.read(Path::new("models/boss.glb")));
        let mut menu = pin!(reader.read(Path::new("ui/icons/castle.png")));
        let mut context = Context::from_waker(Waker::noop());
        assert!(combat.as_mut().poll(&mut context).is_pending());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(matches!(
            menu.as_mut().poll(&mut context),
            Poll::Ready(Ok(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        gate.release();
        assert!(matches!(
            combat.as_mut().poll(&mut context),
            Poll::Ready(Ok(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
