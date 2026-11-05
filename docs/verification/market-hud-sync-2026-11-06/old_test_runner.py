import subprocess,json,time,pathlib
p=pathlib.Path('target/verification/market-hud-sync-2026-11-06');cmd=['nix','develop','--command','cargo','test','--locked','--release','--lib','paused_purchase_updates_hud_and_market_wallet_in_the_purchase_frame','-j2'];t=time.monotonic()
with (p/'old-test.log').open('w') as f:r=subprocess.run(cmd,stdout=f,stderr=subprocess.STDOUT)
(p/'old-test-process.json').write_text(json.dumps(dict(command=cmd,exit=r.returncode,elapsed=time.monotonic()-t),indent=2)+'\n')
