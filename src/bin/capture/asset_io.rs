//! Genuine held/missing asset reads, isolated from production asset files.
use bevy::{
    asset::{
        AssetApp,
        io::{
            AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
            file::FileAssetReader,
        },
    },
    prelude::*,
};
use std::{
    collections::BTreeMap,
    fs,
    future::poll_fn,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Poll, Waker},
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Default)]
pub(super) struct ReadGate {
    pub open: AtomicBool,
    pub reads: AtomicUsize,
    pub path_reads: Mutex<BTreeMap<String, usize>>,
    waiters: Mutex<Vec<Waker>>,
}
impl ReadGate {
    async fn wait(&self) {
        poll_fn(|context| {
            let mut waiters = self.waiters.lock().unwrap();
            if self.open.load(Ordering::SeqCst) {
                Poll::Ready(())
            } else {
                waiters.push(context.waker().clone());
                Poll::Pending
            }
        })
        .await;
    }
    pub fn release(&self) {
        self.open.store(true, Ordering::SeqCst);
        for waker in self.waiters.lock().unwrap().drain(..) {
            waker.wake();
        }
    }
}
struct ControlledReader {
    targets: Vec<&'static str>,
    file: FileAssetReader,
    gate: Arc<ReadGate>,
}
impl AssetReader for ControlledReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        if self.targets.iter().any(|target| path == Path::new(target)) {
            self.gate.reads.fetch_add(1, Ordering::SeqCst);
            *self
                .gate
                .path_reads
                .lock()
                .unwrap()
                .entry(path.to_string_lossy().into_owned())
                .or_default() += 1;
            self.gate.wait().await;
        }
        self.file.read(path).await
    }
    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        self.file.read_meta(path).await
    }
    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.file.read_directory(path).await
    }
    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        self.file.is_directory(path).await
    }
}
fn copy_assets(source: &Path, target: &Path, relative: &Path, omitted: &[&str]) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let child = relative.join(entry.file_name());
        if omitted.iter().any(|path| child == Path::new(path)) {
            continue;
        }
        if entry.path().is_dir() {
            copy_assets(
                &entry.path(),
                &target.join(entry.file_name()),
                &child,
                omitted,
            );
        } else {
            fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}
/// Register before AssetPlugin; omit only controlled paths in a private copy.
pub(super) fn prepare(
    app: &mut App,
    targets: &[&'static str],
    initially_open: bool,
) -> (PathBuf, Arc<ReadGate>) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("rogue-readiness-{}-{nonce}", std::process::id()));
    copy_assets(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"),
        &root,
        Path::new(""),
        targets,
    );
    let gate = Arc::new(ReadGate::default());
    gate.open.store(initially_open, Ordering::SeqCst);
    let reader_root = root.clone();
    let reader_gate = gate.clone();
    let targets = targets.to_vec();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || {
            Box::new(ControlledReader {
                targets: targets.clone(),
                file: FileAssetReader::new(&reader_root),
                gate: reader_gate.clone(),
            })
        }),
    );
    (root, gate)
}
pub(super) fn restore(root: &Path, targets: &[&str]) {
    for path in targets {
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(path),
            root.join(path),
        )
        .unwrap();
    }
}
