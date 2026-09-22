#!/usr/bin/env python3
"""Regenerate the Generic PDK deck (GENERIC_DECK_JSON in src/generic_deck.rs).

The Generic PDK is the SUPER-STRICT exhaustive envelope of the target PDKs: a
Device that passes DRC against it is guaranteed to pass every real PDK. This
script reads the target decks and prints the merged deck JSON to stdout; paste
the printed body into `GENERIC_DECK_JSON`.

Strictness rule, per DRC rule signature (kind + layer fields + window):
  * min_*  -> MAX across PDKs   (hardest lower bound)
  * min_density.min_frac -> MAX; max_density.max_frac -> MIN; max_width -> MIN
  * off_grid.grid -> LCM of grids
  * angle.allowed -> INTERSECTION of allowed sets
  * layers -> only UNIVERSAL roles (intersection of layer roles across PDKs);
    sky130-only roles like hvtp/lvtn/npn/pnp/rpm are dropped
  * a rule referencing any non-universal layer is DROPPED (a Device must not
    depend on a layer a weaker PDK lacks)
  * min_width > max_width on the same layer -> kept, flagged `// NOTE:` (stderr).

Usage (from this tools/ dir; default paths assume the Philis repo layout):
  python3 gen_generic_deck.py [sky130.json generic_finfet.json]
"""
import json
import sys
from math import gcd
from functools import reduce

DEFAULT_PDKS = ["../../../../pdks/sky130.json", "../../../../pdks/generic_finfet.json"]

MAXF = {"min", "min_frac"}   # merge toward strictness by max
MINF = {"max", "max_frac"}   # merge toward strictness by min
LAYER_FIELDS = ("layer", "ref", "reference", "inner", "outer", "layer_a", "layer_b")


def lcm(a, b):
    return a * b // gcd(a, b)


def sig(body):
    fields = tuple(sorted((f, body[f]) for f in LAYER_FIELDS + ("window",) if f in body))
    return (body["kind"], fields)


def normalize(pdk):
    """drc map -> {sig: body}. Handles NAMED rules (explicit "kind") and
    KIND-KEYED defaults (key IS the kind, no "kind" field)."""
    out = {}
    for key, body in pdk["drc"].items():
        b = dict(body)
        b.setdefault("kind", key)
        b["_id"] = key
        out[sig(b)] = b
    return out


def merge_bodies(bodies):
    out = dict(bodies[0])
    for b in bodies[1:]:
        for f, v in b.items():
            if f in ("kind", "_id") or f in LAYER_FIELDS or f == "window":
                continue
            if f == "grid":
                out[f] = lcm(out[f], v)
            elif f == "allowed":
                out[f] = sorted(set(out[f]) & set(v))
            elif f in MAXF:
                out[f] = max(out[f], v)
            elif f in MINF:
                out[f] = min(out[f], v)
            else:
                out[f] = v
    return out


def main(paths):
    pdks = [json.load(open(p)) for p in paths]
    universal = reduce(lambda a, b: a & b, (set(p["layers"]) for p in pdks))
    layers = {r: pdks[0]["layers"][r] for r in sorted(universal)}

    per_sig = {}
    for p in pdks:
        for s, b in normalize(p).items():
            per_sig.setdefault(s, []).append(b)

    merged, dropped = {}, []
    for s, bodies in per_sig.items():
        b = merge_bodies(bodies)
        refs = [b[f] for f in LAYER_FIELDS if f in b]
        if any(r not in universal for r in refs):
            dropped.append((b["_id"], refs))
            continue
        merged[b["_id"]] = b

    notes = []
    minw = {b["layer"]: b["min"] for b in merged.values()
            if b["kind"] == "min_width" and "layer" in b}
    for b in merged.values():
        if b["kind"] == "max_width" and minw.get(b.get("layer"), -1) > b["max"]:
            notes.append(f'// NOTE: {b["_id"]} max_width {b["max"]} < min_width '
                         f'{minw[b["layer"]]} on {b["layer"]} -- infeasible, kept per spec')

    drc = {}
    for rid, b in merged.items():
        entry = {"kind": b["kind"]}
        for f in ("layer", "ref", "inner", "outer", "layer_a", "layer_b", "window"):
            if f in b:
                entry[f] = b[f]
        for f in ("min", "max", "grid", "min_frac", "max_frac", "allowed"):
            if f in b:
                entry[f] = b[f]
        drc[rid] = entry

    # Roles and the routing stack are merged the same way as layers: keep only
    # what every source PDK agrees on, so the envelope stays an upper bound on
    # strictness. Without these sections `verify::Pdk::from_json` rejects the
    # deck, and the non-gpurify `GenericPdk` has no roles to resolve.
    role_maps = [p.get("cell", {}).get("layers", {}) for p in pdks]
    scalar_roles = reduce(
        lambda a, b: a & b,
        ({k for k, v in m.items() if isinstance(v, str)} for m in role_maps),
    )
    cell_layers = {}
    for r in sorted(scalar_roles):
        targets = {m[r] for m in role_maps}
        # A role must resolve to the same layer everywhere, and that layer must
        # have survived the universal-layer intersection above.
        if len(targets) == 1 and next(iter(targets)) in universal:
            cell_layers[r] = next(iter(targets))
    for key in ("routing_metals", "routing_vias"):
        stacks = [m.get(key, []) for m in role_maps]
        common = [l for l in stacks[0] if all(l in s for s in stacks) and l in universal]
        cell_layers[key] = common

    conductors = reduce(
        lambda a, b: [c for c in a if c in b],
        (p.get("connectivity", {}).get("conductors", []) for p in pdks),
    )
    vias, seen = [], set()
    for p in pdks:
        for v in p.get("connectivity", {}).get("vias", []):
            key = v["layer"]
            if key in seen or key not in universal:
                continue
            if all(c in universal for c in v["connects"]):
                vias.append(v)
                seen.add(key)

    deck = {
        "layers": layers,
        "drc": dict(sorted(drc.items())),
        "connectivity": {
            "conductors": [c for c in conductors if c in universal],
            "vias": vias,
        },
        "cell": {"layers": cell_layers},
    }
    print(json.dumps(deck, indent=2))
    sys.stderr.write(f"layers={len(layers)} rules={len(drc)} dropped={len(dropped)}\n")
    for did, refs in dropped:
        sys.stderr.write(f"  dropped {did} (non-universal refs {refs})\n")
    for n in notes:
        sys.stderr.write(n + "\n")


if __name__ == "__main__":
    main(sys.argv[1:] or DEFAULT_PDKS)
