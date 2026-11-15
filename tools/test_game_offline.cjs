#!/usr/bin/env node
// Run against a built release served over localhost/HTTPS. Real game + real SW.
const {chromium} = require('playwright');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const assert = require('node:assert/strict');
(async () => {
  const base = process.env.PWA_GAME_URL || 'http://localhost:8767/';
  const output = path.resolve(process.env.PWA_GAME_OUTPUT || 'target/pwa-game-tests');
  await fs.mkdir(output, {recursive:true});
  const profile = await fs.mkdtemp(path.join(os.tmpdir(), 'rogue-game-cache-'));
  const launch = () => chromium.launchPersistentContext(profile, {
    executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    headless:true, viewport:{width:1280,height:720},
    args:['--no-sandbox','--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader'],
  });
  let context;
  const errors = [];
  const attach = page => {
    page.on('pageerror',error=>errors.push(String(error)));
    page.on('console',msg=>{ if(msg.type()==='error' && !msg.text().includes('exceptions for control flow')) errors.push(msg.text()); });
  };
  const ready = async page => {
    await page.locator('#start').click();
    await page.waitForFunction(()=>document.body.classList.contains('playing'),null,{timeout:180000});
    await page.waitForTimeout(30000); // Software WebGL shader/font warm-up; screenshots must still be inspected.
  };
  try {
    context=await launch(); let page=await context.newPage(); attach(page);
    await page.goto(base+'?lang=en');
    await ready(page);
    await page.screenshot({path:path.join(output,'first-online.png')});
    const status = await page.evaluate(async()=>{
      const reg=await navigator.serviceWorker.getRegistration();
      return new Promise(resolve=>{
        const ch=new MessageChannel();ch.port1.onmessage=e=>{ch.port1.close();resolve(e.data);};
        reg.active.postMessage({type:'STATUS',target:'current',wasmEncoding:'gzip'},[ch.port2]);
      });
    });
    await fs.writeFile(path.join(output,'first-status.json'),JSON.stringify(status,null,2));
    assert.equal(status.ready,true); assert.equal(status.downloadBytes,0);
    console.log('PASS real game first launch cached all selected resources', status.totalBytes);
    await context.close();
    context=await launch(); await context.setOffline(true);
    page=await context.newPage(); attach(page);
    await page.goto(base+'?lang=en');
    await ready(page);
    await page.screenshot({path:path.join(output,'cold-offline.png')});
    assert.deepEqual(errors.filter(e=>!e.includes('ERR_INTERNET_DISCONNECTED')),[]);
    await fs.writeFile(path.join(output,'results.json'),JSON.stringify({
      firstLaunchReady:status.ready,cachedBytes:status.totalBytes,
      browserRestarted:true,offlineBeforeNavigation:true,actualGamePlaying:true,
      errors,version:status.version,
    },null,2));
    console.log('PASS browser restart with network disabled: actual game loaded and rendered');
  } finally { if(context)await context.close(); await fs.rm(profile,{recursive:true,force:true}); }
})().catch(error=>{console.error(error);process.exitCode=1;});
