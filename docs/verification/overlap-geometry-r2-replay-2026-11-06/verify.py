#!/usr/bin/env python3
"""Independently verify lossless receipts and earliest recorded numeric drift."""
import gzip, hashlib, json, pathlib, io
P = pathlib.Path(__file__).resolve().parent
receipts = json.loads((P / 'raw-receipts.json').read_text())
for r in receipts:
    p = P / r['file']
    data = b''.join((P / n).read_bytes() for n in r['parts']) if 'parts' in r else p.read_bytes()
    assert hashlib.sha256(data).hexdigest() == r['compressed_sha256']
    h = hashlib.sha256(); actors = contacts = 0
    with gzip.GzipFile(fileobj=io.BytesIO(data), mode='rb') as f:
        for line in f:
            h.update(line)
            contacts += b'PROJECTILE_HIT ' in line
            if p.name.endswith('.jsonl.gz'):
                actors += json.loads(line).get('type') == 'trace'
    assert h.hexdigest() == r['uncompressed_sha256']
    assert (actors, contacts) == (r['actor_trace_rows'], r['actual_contact_rows'])
def rows(case):
    name = f'replay-{case}/active-h2-behind-sa11ce-c5-t0.jsonl.gz'
    r = next(r for r in receipts if r['file'] == name)
    data = b''.join((P / n).read_bytes() for n in r['parts']) if 'parts' in r else (P / name).read_bytes()
    with gzip.GzipFile(fileobj=io.BytesIO(data), mode='rb') as f:
        for line in f:
            r = json.loads(line)
            if r.get('type') == 'trace': yield r
def numeric(r):
    result = {k:v for k,v in r.items() if k not in ('enemies','projectiles','hero_positions','enemy_order')}
    for key in ('enemies','projectiles','hero_positions'):
        result[key] = sorted(row[1:] for row in r[key])
    return result
first = None
for a,b in zip(rows('a'), rows('b')):
    assert a['frame'] == b['frame']
    if numeric(a) != numeric(b):
        first = a['frame']; break
assert first == 61703, first
print('PASS lossless hashes/counts; numeric projection equal through 61702, first diff 61703')
