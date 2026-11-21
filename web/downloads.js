// Contextual chapter preparation and offline management, reached through settings.
export function createDownloadUI({worker, request, language, updates}) {
  const dialog = document.createElement('dialog');
  dialog.id = 'downloads-dialog';
  dialog.setAttribute('aria-labelledby', 'downloads-title');
  dialog.innerHTML = `<h2 id="downloads-title"></h2><p id="downloads-summary" aria-live="polite"></p><progress id="downloads-meter" hidden></progress><div id="downloads-groups"></div><p id="downloads-note"></p><section id="downloads-update"><p id="downloads-update-status" role="status"></p><small id="downloads-version"></small><button id="downloads-update-check" type="button"></button><button id="downloads-update-apply" type="button" hidden></button></section><div class="downloads-actions"><button id="downloads-confirm" type="button"></button><button id="downloads-remove" type="button"></button><button id="downloads-close" type="button"></button></div>`;
  document.body.append(dialog);
  window.rogueDownloadsOpen = false;
  dialog.addEventListener('close', () => { window.rogueDownloadsOpen = false; });
  const title = dialog.querySelector('h2'), summary = dialog.querySelector('#downloads-summary');
  const meter = dialog.querySelector('progress'), groups = dialog.querySelector('#downloads-groups');
  const note = dialog.querySelector('#downloads-note'), confirm = dialog.querySelector('#downloads-confirm');
  const remove = dialog.querySelector('#downloads-remove'), close = dialog.querySelector('#downloads-close');
  const updateSection = dialog.querySelector('#downloads-update'), updateStatus = dialog.querySelector('#downloads-update-status');
  const version = dialog.querySelector('#downloads-version');
  const checkUpdate = dialog.querySelector('#downloads-update-check'), applyUpdate = dialog.querySelector('#downloads-update-apply');
  window.rogueExpeditionActive = false;
  const text = {
    en: {version: 'Installed', availableVersion: 'Available', checkUpdate: 'Check for updates', applyUpdate: 'Update & restart', expeditionUpdate: 'New version available. Return to the chapter menu to update safely.', chapter: 'Prepare expedition', manage: 'Downloads & offline play', checking: 'Checking saved resources…',
      size: n => `${n} to download`, ready: 'These resources are saved and ready to play offline.',
      shared: 'All six chapters share this battle pack. Heroes, enemies and skills are prepared before combat.',
      optional: 'Download everything for offline play. Saved files are reused; only missing content is fetched.',
      progress: (a,b) => `Saving resources · ${a} / ${b}`, error: 'Download incomplete. Check your connection and free storage, then retry. Saved files will be reused.',
      unavailable: 'Offline storage is unavailable. This expedition will load online.',
      online: 'Continue online', download: 'Download & play', all: 'Download all for offline play', retry: 'Retry download',
      close: 'Back', cancel: 'Cancel download', clear: 'Remove downloaded extras',
      protected: 'Resources in an open expedition cannot be removed. Close other game windows and return to the chapter menu first.',
      removed: 'Extra downloads removed. Your progress and launcher are safe.',
      core: 'Launcher & shared interface', combat: 'Shared expedition pack', optionalGroup: 'Credits & supporting files',
      saved: 'Saved', missing: 'Not downloaded', used: n => `Saved content: ${n}`},
    'zh-CN': {version: '当前版本', availableVersion: '可用版本', checkUpdate: '检查更新', applyUpdate: '更新并重启', expeditionUpdate: '发现新版本，请先返回章节菜单，再安全更新。', chapter: '准备远征', manage: '下载与离线游玩', checking: '正在检查已保存的资源…',
      size: n => `需要下载 ${n}`, ready: '资源已保存，可以离线游玩。',
      shared: '六个章节共用战斗资源包。进入战斗前会准备好英雄、敌人和技能。',
      optional: '可下载全部内容供离线游玩。已有文件会复用，只下载缺少的内容。',
      progress: (a,b) => `正在保存资源 · ${a} / ${b}`, error: '下载未完成。请检查网络和可用空间后重试，已保存文件会继续复用。',
      unavailable: '当前无法保存离线资源，本次远征将联网加载。',
      online: '联网继续', download: '下载并开始', all: '下载全部离线内容', retry: '重试下载', close: '返回', cancel: '取消下载',
      clear: '移除已下载的额外内容', protected: '正在使用的远征资源无法移除。请关闭其他游戏窗口，并先返回章节菜单。',
      removed: '额外下载已移除，游戏进度和启动资源已保留。',
      core: '启动与公共界面', combat: '共用远征资源包', optionalGroup: '制作名单与附属文件', saved: '已保存', missing: '未下载', used: n => `已保存内容：${n}`},
  };
  const bytes = n => n < 1048576 ? `${Math.ceil(n / 1024)} KB` : `${(n / 1048576).toFixed(1)} MB`;
  let mode = 'manage', stage = 0, status = null, phase = 'checking', busy = false;
  let received = 0, total = 0, generation = 0, currentWorker;
  function render() {
    const t = text[language()] || text.en;
    const update = updates?.status();
    updateSection.hidden = mode !== 'manage' || !updates;
    updateStatus.textContent = update?.available && window.rogueExpeditionActive ? t.expeditionUpdate : update?.text || '';
    version.textContent = update?.currentRevision ? `${t.version} ${update.currentRevision.slice(0, 7)}${update.availableRevision ? ` · ${t.availableVersion} ${update.availableRevision.slice(0, 7)}` : ''}` : '';
    checkUpdate.textContent = t.checkUpdate; applyUpdate.textContent = t.applyUpdate;
    checkUpdate.disabled = busy || update?.busy;
    applyUpdate.hidden = !update?.available;
    applyUpdate.disabled = busy || update?.busy || window.rogueExpeditionActive;
    title.textContent = t[mode === 'chapter' ? 'chapter' : 'manage'];
    summary.textContent = phase === 'progress' ? t.progress(bytes(received), bytes(total)) :
      ['error','protected','removed','unavailable'].includes(phase) ? t[phase] :
      !status ? t.checking : status.downloadBytes ? t.size(bytes(status.downloadBytes)) : t.ready;
    note.textContent = t[mode === 'chapter' ? 'shared' : 'optional'];
    confirm.textContent = phase === 'unavailable' ? t.online : phase === 'error' ? t.retry : t[mode === 'chapter' ? 'download' : 'all'];
    confirm.hidden = mode === 'manage' && (status?.ready === true || phase === 'unavailable');
    confirm.disabled = busy || update?.busy || phase === 'checking';
    close.textContent = t[busy ? 'cancel' : 'close'];
    remove.textContent = t.clear;
    remove.hidden = mode !== 'manage'; remove.disabled = busy || update?.busy || !status;
    meter.hidden = phase !== 'progress'; meter.max = Math.max(1,total); meter.value = received;
    groups.replaceChildren();
    if (mode === 'manage' && status) {
      for (const group of status.groups || []) {
        const row = document.createElement('p');
        row.textContent = `${t[group.id === 'optional' ? 'optionalGroup' : group.id] || group.id} · ${group.ready ? t.saved : t.missing} · ${bytes(group.totalBytes - group.downloadBytes)} / ${bytes(group.totalBytes)}`;
        groups.append(row);
      }
    }
  }
  async function rpc(type, onProgress, selected) {
    currentWorker = await worker();
    if (!currentWorker) throw new Error('OFFLINE_STORAGE_UNAVAILABLE');
    return request(currentWorker, type, onProgress, type === 'PREPARE' ? 120000 : 15000, 'client', selected);
  }
  let pinQueue = Promise.resolve();
  function pinGroups(selected) {
    pinQueue = pinQueue.catch(() => {}).then(() => rpc('PIN_GROUPS', null, selected));
    return pinQueue;
  }
  async function finishChapter(id) {
    await pinGroups(['combat']);
    if (id !== generation) return;
    window.rogueExpeditionActive = true;
    window.rogueChapterDownload = {stage, status: 'ready'};
    dialog.close();
  }
  async function show(nextMode, nextStage = 0) {
    if (busy) return;
    const id = ++generation;
    mode = nextMode; stage = nextStage; status = null; phase = 'checking';
    if (mode === 'chapter') window.rogueChapterDownload = {stage, status: 'downloading'};
    render(); if (!dialog.open) { window.rogueDownloadsOpen = true; dialog.showModal(); }
    try {
      const result = await rpc('STATUS', null, mode === 'chapter' ? ['combat'] : ['all']);
      if (id !== generation) return;
      status = result;
      phase = 'ready';
      if (mode === 'manage') void updates?.check();
      if (mode === 'chapter' && status.ready) await finishChapter(id);
    } catch (error) {
      if (id !== generation) return;
      console.warn('Resource status unavailable', error); phase = error.message === 'OFFLINE_STORAGE_UNAVAILABLE' ? 'unavailable' : 'error';
    }
    render();
  }
  confirm.addEventListener('click', async () => {
    if (busy) return;
    if (phase === 'unavailable' && mode === 'chapter') {
      window.rogueExpeditionActive = true;
      window.rogueChapterDownload = {stage, status: 'ready'}; dialog.close(); return;
    }
    const id = generation; busy = true; phase = 'progress'; received = 0; total = status?.downloadBytes || 0; render();
    try {
      const result = await rpc('PREPARE', data => {
        if (id !== generation) return;
        received = data.received; total = data.total; render();
      }, mode === 'chapter' ? ['combat'] : ['all']);
      if (id !== generation) return;
      status = result;
      phase = 'ready';
      if (mode === 'chapter') await finishChapter(id);
    } catch (error) {
      if (id !== generation) return;
      console.warn('Resources remain incomplete', error); phase = 'error';
    } finally { busy = false; render(); }
  });
  async function back() {
    generation++;
    if (mode === 'chapter') {
      window.rogueChapterDownload = {stage, status: 'cancelled'};
      void pinGroups([]).catch(() => {});
    }
    dialog.close();
    if (busy && currentWorker) await request(currentWorker, 'CANCEL').catch(() => {});
  }
  close.addEventListener('click', () => { void back(); });
  dialog.addEventListener('cancel', event => { event.preventDefault(); void back(); });
  remove.addEventListener('click', async () => {
    busy = true; render();
    try {
      await rpc('REMOVE_GROUPS', null, ['combat','optional']);
      status = await rpc('STATUS', null, ['all']); phase = 'removed';
    } catch { phase = 'protected'; }
    finally { busy = false; render(); }
  });
  checkUpdate.addEventListener('click', async () => { await updates?.check(); render(); });
  applyUpdate.addEventListener('click', async () => { await updates?.apply(); render(); });
  window.roguePrepareChapter = stage => { void show('chapter', stage); };
  window.rogueOpenDownloads = () => { void show('manage'); };
  window.rogueReleaseChapter = () => {
    window.rogueExpeditionActive = false; render();
    return pinGroups([]).catch(() => {});
  };
  return {render};
}
