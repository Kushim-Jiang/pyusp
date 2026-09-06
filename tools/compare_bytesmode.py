#!/usr/bin/env python3
"""Compare bytes-mode harness trace vs engine GDI reference stage-by-stage."""
import json, re, sys

# harness output: lines "<name>: g1 g2 ..." (also 'final:' printed twice; take both)
# engine json: stages[{m: 'wineusp lookup <name> (item 0)', glyphs:[{g,...}]}]

def parse_harness(path):
    seq = []
    for line in open(path, encoding='utf-8'):
        m = re.match(r'^([a-z]+#?\d*/\d*|cmap|final): (.*)$', line.strip())
        if m:
            gids = [int(x) for x in m.group(2).split()]
            seq.append((m.group(1), gids))
    # harness also printf's its own trailing "final:" after the trace stage
    # list; drop the duplicate if the trace's "final" stage is already present.
    nfinal = sum(1 for n, _ in seq if n == 'final')
    if nfinal > 1:
        for i in range(len(seq) - 1, -1, -1):
            if seq[i][0] == 'final':
                del seq[i]
                break
    return seq

def parse_engine(path):
    data = json.load(open(path, encoding='utf-8'))
    seq = []
    for st in data.get('stages', []):
        m = st.get('m', '')
        mm = re.match(r'^wineusp lookup (.*) \(item \d+\)$', m)
        if not mm:
            continue
        name = mm.group(1)
        gids = [g['g'] for g in st.get('glyphs', [])]
        seq.append((name, gids))
    return seq

def norm(name):
    # rclt#k/49 -> rclt (index may differ if engine skipped? should be same)
    return re.sub(r'#\d+/\d+', '', name)

h = parse_harness(sys.argv[1])
e = parse_engine(sys.argv[2])
print(f'harness stages={len(h)} engine stages={len(e)}')
ok = True
if len(h) != len(e):
    print('LENGTH MISMATCH')
    ok = False
else:
    for i, (hn, hg) in enumerate(h):
        en, eg = e[i]
        if norm(hn) != norm(en):
            print(f'stage {i}: NAME mismatch {hn!r} vs {en!r}')
            ok = False
            break
        if hg != eg:
            print(f'stage {i} {hn}: GID mismatch\n  h={hg}\n  e={eg}')
            ok = False
            break
print('MATCH' if ok else 'MISMATCH')
sys.exit(0 if ok else 1)
