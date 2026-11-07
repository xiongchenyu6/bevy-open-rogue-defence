"""Compare every recorded simulation JSON value, ignoring only root entity identity."""
import argparse
import gzip
import json
from pathlib import Path

def records(path, strip_identity=True):
    data=gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()
    result=[]
    for line in data.splitlines():
        if not line.startswith('{'):continue
        row=json.loads(line)
        if row.get('type')=='runner_metadata':continue
        if strip_identity:row.pop('root',None)
        result.append(row)
    return result

def compare(left,right):
    a,b=records(left),records(right)
    differences=[]
    for index in range(max(len(a),len(b))):
        first=a[index] if index<len(a) else None
        second=b[index] if index<len(b) else None
        if first!=second:
            fields=sorted(key for key in set(first or {})|set(second or {}) if (first or {}).get(key)!=(second or {}).get(key))
            differences.append({'record_index':index,'left_type':(first or {}).get('type'),'right_type':(second or {}).get('type'),'left_frame':(first or {}).get('frame'),'right_frame':(second or {}).get('frame'),'different_fields':fields})
    original_a,original_b=records(left,False),records(right,False)
    first=differences[0]['record_index'] if differences else None
    first_records=None if first is None else {'record_index':first,'left':original_a[first] if first<len(original_a) else None,'right':original_b[first] if first<len(original_b) else None}
    return {'status':'RECORDED_PROJECTION_MISMATCH' if differences else 'PASS','matching_prefix_records':first if first is not None else len(a),'first_difference_original_records':first_records,'left_records':len(a),'right_records':len(b),'recorded_projection_equal':not differences,'differences':differences,'scope':'All recorded JSON fields preserved except root entity identity and runner wrapper metadata. This comparison is not an unrecorded-world or global determinism claim.'}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('left',type=Path);p.add_argument('right',type=Path);args=p.parse_args()
    print(json.dumps(compare(args.left,args.right),indent=2))
