import pathlib,subprocess,json,time
out=pathlib.Path(__file__).resolve().parent
start=time.monotonic()
with (out/"runtime.log").open("w") as log:
 try:
  p=subprocess.run(json.loads((out/"command.json").read_text()),stdout=log,stderr=subprocess.STDOUT,timeout=300);result={"exit":p.returncode,"elapsed":time.monotonic()-start}
 except subprocess.TimeoutExpired:
  subprocess.run(["docker","stop","-t","15","rogue-vortex-forge-restart-r2-nov6"],stdout=subprocess.DEVNULL);result={"timeout":True,"elapsed":time.monotonic()-start}
(out/"process.json").write_text(json.dumps(result,indent=2)+"\n")
print(result,flush=True)
