import pathlib,hashlib,json,gzip,tarfile,io
p=pathlib.Path(__file__).resolve().parent
r=json.loads((p/'raw-receipt.json').read_text());parts=[]
for part in r['parts']:
 b=(p/part['file']).read_bytes();assert hashlib.sha256(b).hexdigest()==part['sha256'];parts.append(b)
b=b''.join(parts);assert hashlib.sha256(b).hexdigest()==r['tar_gzip_sha256']
want=json.loads((p/'raw-source-files.json').read_text());actual={}
with gzip.GzipFile(fileobj=io.BytesIO(b)) as z,tarfile.open(fileobj=z,mode='r|') as t:
 for member in t:actual[member.name]=hashlib.sha256(t.extractfile(member).read()).hexdigest()
assert actual==want and len(actual)==r['source_file_count']
result=json.loads((p/'forge/result.json').read_text());process=json.loads((p/'forge/process.json').read_text())
assert process['exit']==1 and not process.get('timeout') and result['terminal']=='NO LIVES REMAIN'
assert result['purchases'][0]['wallet_before']-result['purchases'][0]['wallet_after']==180
assert result['acquired_materials']==['Arc Lens'] and not result.get('completed')
assert len(result['market_inspections'])==19
assert not any('echocore'==''.join(o['name'].lower().split()) for visit in result['market_inspections'] for o in visit['offers'])
for name,want in json.loads((p/'forge/harness-snapshot.json').read_text())['files'].items():assert hashlib.sha256((p/'forge/harness-source'/name).read_bytes()).hexdigest()==want
print('PASS complete lossless raw archive, frozen helper hashes, real defeat, one purchase and19 no-EchoCore inspections; forge/restart incomplete')
