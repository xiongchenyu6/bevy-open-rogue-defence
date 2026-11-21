import { createDownloadUI } from './downloads.js';
// Startup-only resource preparation and explicit updates. Never reload a live run.
const card = document.querySelector('#cache-card');
const label = document.querySelector('#cache-status');
const meter = document.querySelector('#cache-progress');
const update = document.querySelector('#cache-update');
const later = document.querySelector('#cache-later');
const start = document.querySelector('#start');
const encoding = typeof DecompressionStream === 'function' ? 'gzip' : 'raw';
let language = 'en';
let registration;
let active;
let candidate;
let started = false;
let busy = false;
let dismissed = false;
let state = 'checking';
let amount = 0;
let total = 0;
let checkingUpdate = false;
let notifiedVersion;
let currentRevision;
let availableRevision;
const notice = document.createElement('p');
notice.id = 'update-notice'; notice.hidden = true; notice.setAttribute('role', 'status');
document.body.append(notice);
const copy = {
  en: {
    notice: 'New version available. Open Settings → Downloads & offline play to update after your expedition.',
    checking: 'Checking saved game resources…',
    first: 'Save the launcher first. Expedition resources download when you choose a chapter.',
    ready: 'Launcher saved · Chapters download only when needed',
    available: () => total === 0 ? 'Update saved · Ready to switch versions' : `A new version is available · ${total < 104858 ? "< 0.1" : mb(total)} MB to download`,
    progress: () => `Saving game resources · ${mb(amount)} / ${mb(total)} MB`,
    switching: 'Update ready. Restarting…',
    error: 'Resources could not be saved. Check your connection and available storage, then retry.',
    updateError: 'Update failed. Your current version is still available. Try again when connected.',
    unsupported: 'Local caching is unavailable in this browser. An internet connection is required.',
    update: 'Download & update', cachedUpdate: 'Use new version', later: 'Later', retry: 'Retry update',
  },
  'zh-CN': {
    notice: '发现新版本。远征结束后，可在设置 → 下载与离线游玩中更新。',
    checking: '正在检查已保存的游戏资源…',
    first: '先保存启动资源，选择章节时再下载战斗内容。',
    ready: '启动资源已保存 · 章节内容按需下载',
    available: () => total === 0 ? '新版本已保存 · 可直接更新' : `发现新版本 · 需下载 ${total < 104858 ? "小于 0.1" : mb(total)} MB`,
    progress: () => `正在保存游戏资源 · ${mb(amount)} / ${mb(total)} MB`,
    switching: '更新已就绪，正在重新启动…',
    error: '资源保存失败，请检查网络和可用存储空间后重试。',
    updateError: '更新未完成，当前版本仍可使用。联网后可重试。',
    unsupported: '当前浏览器无法保存离线资源，本次需要联网游玩。',
    update: '下载并更新', cachedUpdate: '使用新版', later: '稍后', retry: '重试更新',
  },
};
function mb(bytes) { return (bytes / 1048576).toFixed(1); }
function render() {
  const text = copy[language];
  const value = text[state];
  label.textContent = typeof value === 'function' ? value() : value;
  card.hidden = started;
  const offer = !dismissed && !busy && candidate && ['available', 'updateError'].includes(state);
  update.hidden = !offer;
  later.hidden = !offer;
  update.textContent = text[state === 'updateError' ? 'retry' : total === 0 ? 'cachedUpdate' : 'update'];
  later.textContent = text.later;
  meter.hidden = state !== 'progress';
  meter.max = Math.max(1, total);
  meter.value = amount;
  meter.setAttribute('aria-label', label.textContent);
  notice.textContent = text.notice;
  downloads?.render();
}
export function setCacheLanguage(value) { language = value === 'zh-CN' ? value : 'en'; render(); downloads.render(); }
function request(worker, type, onProgress, timeout = 15000, target = 'latest', groups) {
  return new Promise((resolve, reject) => {
    const channel = new MessageChannel();
    let timer;
    const arm = () => {
      clearTimeout(timer);
      timer = setTimeout(() => finish(new Error('Resource worker timed out')), timeout);
    };
    const finish = (error, value) => {
      clearTimeout(timer); channel.port1.close();
      if (error) reject(error); else resolve(value);
    };
    channel.port1.onmessage = ({ data }) => {
      if (data.type === 'PROGRESS') { arm(); onProgress?.(data); }
      else if (data.type === 'ERROR') finish(new Error(data.message || data.error || 'Resource preparation failed'));
      else finish(null, data);
    };
    arm();
    try { worker.postMessage({ type, target, groups, wasmEncoding: encoding }, [channel.port2]); }
    catch (error) { finish(error); }
  });
}
async function inspect(worker, target = 'latest') {
  if (!worker) return null;
  try {
    const result = await request(worker, 'STATUS', null, 3000, target);
    return result.version ? result : null;
  } catch { return null; }
}
async function discover() {
  if (busy || !registration) return;
  const current = registration.active;
  const currentStatus = await inspect(current, started ? 'client' : 'current');
  if (currentStatus) { active = current; currentRevision = currentStatus.revision; }
  const waiting = registration.waiting || current;
  const next = await inspect(waiting);
  if (busy) return;
  if (next && currentStatus && next.version !== currentStatus.version) {
    candidate = waiting; availableRevision = next.revision;
    if (started && notifiedVersion !== next.version) {
      notifiedVersion = next.version; notice.hidden = false;
      setTimeout(() => { notice.hidden = true; }, 10000);
    }
    if (!dismissed || started) { total = next.downloadBytes; state = 'available'; }
  } else {
    candidate = null; availableRevision = null;
    state = currentStatus?.ready ? 'ready' : 'first';
  }
  render();
}
function watch(worker) {
  worker?.addEventListener('statechange', () => { void discover(); });
}
let downloads;
const connected = (async () => {
  if (!('serviceWorker' in navigator) || !window.isSecureContext) {
    state = 'unsupported'; render(); return null;
  }
  try {
    try {
      registration = await navigator.serviceWorker.register('./sw.js', { scope: './', updateViaCache: 'none' });
    } catch (error) {
      registration = await navigator.serviceWorker.getRegistration('./');
      if (!registration) throw error;
    }
    watch(registration.installing);
    registration.addEventListener('updatefound', () => watch(registration.installing));
    navigator.serviceWorker.addEventListener('controllerchange', () => { void discover(); });
    await discover();
    return registration;
  } catch (error) {
    console.warn('Local game cache unavailable', error);
    state = 'unsupported'; render(); return null;
  }
})();
async function installedWorker() {
  const reg = await connected;
  if (!reg) return null;
  if (reg.installing) {
    await new Promise((resolve, reject) => {
      const worker = reg.installing;
      const timer = setTimeout(() => reject(new Error('Installation timed out')), 30000);
      const changed = () => {
        if (['installed', 'activated', 'redundant'].includes(worker.state)) {
          clearTimeout(timer); resolve();
        }
      };
      worker.addEventListener('statechange', changed); changed();
    });
  }
  const worker = reg.active;
  if (await inspect(worker)) return worker;
  // Migrate the former network-only worker before loading the game engine.
  // First installation may activate while STATUS is in flight. Keep the object
  // we inspected: registration.waiting becomes null after activation.
  const waiting = reg.waiting;
  if (await inspect(waiting)) return waiting;
  throw new Error('Resource worker is not ready');
}
function progress(data) {
  state = 'progress'; amount = data.received; total = data.total; render();
}
async function prepare(worker, target = 'current', groups) {
  const status = await request(worker, 'STATUS', null, 15000, target, groups);
  if (!status.ready) {
    amount = 0; total = status.downloadBytes; state = 'progress'; render();
    await request(worker, 'PREPARE', progress, 120000, target, groups);
  }
  // Best effort: the browser ultimately decides whether persistent storage is granted.
  try { await navigator.storage?.persist?.(); } catch { /* cache still works */ }
}
async function switchVersion(worker, resumeStart) {
  if (resumeStart) {
    try { sessionStorage.setItem('rogue-cache-resume-start', '1'); } catch { /* user can tap Start */ }
  }
  state = 'switching'; render();
  await request(worker, 'ACTIVATE');
  if (worker.state !== 'activated') {
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('Activation timed out')), 20000);
      const changed = () => {
        if (worker.state === 'activated') { clearTimeout(timer); resolve(); }
      };
      worker.addEventListener('statechange', changed); changed();
    });
  }
  location.reload();
  // Prevent the current page from starting old JS while navigation is pending.
  await new Promise(() => {});
}
export async function prepareGameCache() {
  busy = true; render();
  try {
    const worker = await installedWorker();
    if (!worker) return false;
    // A quick Start click can race updatefound. Offer an already-installed new
    // release before initializing the old engine; Later remains an explicit opt-out.
    const nextWorker = registration.waiting || worker;
    const [current, next] = await Promise.all([inspect(worker, 'current'), inspect(nextWorker)]);
    if (!dismissed && state !== 'updateError' && next && current && next.version !== current.version) {
      candidate = nextWorker; currentRevision = current.revision; availableRevision = next.revision;
      total = next.downloadBytes; state = 'available';
      return 'update-available';
    }
    await prepare(worker, 'current', ['core']);
    if (worker === registration.waiting) await switchVersion(worker, true);
    active = worker; state = 'ready'; return true;
  } catch (error) {
    state = 'error'; throw error;
  } finally { busy = false; render(); }
}
export function markGameStarted() { started = true; render(); }
export function consumeCacheResume() {
  try {
    const resume = sessionStorage.getItem('rogue-cache-resume-start') === '1';
    sessionStorage.removeItem('rogue-cache-resume-start'); return resume;
  } catch { return false; }
}
update.addEventListener('click', async () => {
  if (busy || started || !candidate) return;
  busy = true; start.disabled = true; render();
  try { await prepare(candidate, 'latest'); await switchVersion(candidate, false); }
  catch (error) { console.warn('Game update not activated', error); state = 'updateError'; }
  finally { busy = false; start.disabled = false; render(); }
});
later.addEventListener('click', async () => {
  dismissed = true; state = (await inspect(active, 'current'))?.ready ? 'ready' : 'first'; render();
});
async function checkForUpdates() {
  if (checkingUpdate || busy) return;
  checkingUpdate = true;
  try {
    const reg = await connected;
    await reg?.update();
    await discover();
  } catch { /* Offline sessions retain their current version. */ }
  finally { checkingUpdate = false; }
}
async function applyUpdate() {
  if (!candidate || busy || window.rogueExpeditionActive) return;
  busy = true; render();
  try { await prepare(candidate, 'latest'); await switchVersion(candidate, false); }
  catch (error) { console.warn('Game update not activated', error); state = 'updateError'; }
  finally { busy = false; render(); }
}
window.addEventListener('online', () => { void checkForUpdates(); });
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible') void checkForUpdates();
});
setInterval(() => {
  if (document.visibilityState === 'visible') void checkForUpdates();
}, 5 * 60 * 1000);
downloads = createDownloadUI({worker: installedWorker, request, language: () => language,
  updates: {
    status: () => ({available: Boolean(candidate), currentRevision, availableRevision, busy, text: label.textContent}),
    check: checkForUpdates, apply: applyUpdate,
  },
});
render();
