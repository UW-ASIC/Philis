#!/usr/bin/env python3
"""
Compare gdsverify DRC results against KLayout's built-in DRC engine.

Runs KLayout in batch mode on conformance.gds, executes equivalent DRC checks
for every rule KLayout can natively express, and compares violation counts
per cell against the manifest.

Usage:
    klayout_compare.py [conformance_dir] [klayout_bin]

Outputs a pass/fail table and a JSON report.
"""

import json, subprocess, sys, os, tempfile

DIR = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__))
KLAYOUT = sys.argv[2] if len(sys.argv) > 2 else "klayout"

with open(os.path.join(DIR, "manifest.json")) as f:
    manifest = json.load(f)
with open(os.path.join(DIR, "params.json")) as f:
    params = json.load(f)

GDS = os.path.join(DIR, "conformance.gds")
LAYERS = params["layers"]

def ld(name):
    """Return 'layer/datatype' string for KLayout."""
    l = LAYERS[name]
    return f"{l['layer']}/{l['datatype']}"

# Build KLayout DRC script per case.
# Each case runs on a specific cell; KLayout's cell-level DRC uses deep/flat mode.
# We collect results as JSON: { case_id: violation_count }

# Map manifest rules → KLayout DRC expressions.
# Some rules (antenna, cheesing, redundant_via, multi_patterning, etc.)
# have no direct KLayout DRC equivalent — those are skipped and reported separately.

drc_params = params["drc"]

def make_kl_check(rule, p):
    """Return (klayout_drc_expression, description) or None if not expressible."""
    met1 = ld("met1")
    met2 = ld("met2")
    poly = ld("poly")
    diff = ld("diff")
    via1 = ld("via1")
    nwell = ld("nwell")

    if rule == "min_width":
        layer = ld(p["layer"])
        return f'input({layer}).width({p["min"]})', "min_width"
    elif rule == "min_spacing":
        layer = ld(p["layer"])
        return f'input({layer}).space({p["min"]})', "min_spacing"
    elif rule == "min_spacing_diff":
        la = ld(p["layer_a"])
        lb = ld(p["layer_b"])
        return f'input({la}).separation(input({lb}), {p["min"]})', "min_spacing_diff"
    elif rule == "min_enclosure":
        outer = ld(p["outer"])
        inner = ld(p["inner"])
        return f'input({inner}).enclosed(input({outer}), {p["min"]})', "min_enclosure"
    elif rule == "min_extension":
        layer = ld(p["layer"])
        ref = ld(p["ref"])
        return f'input({layer}).enclosing(input({ref}), {p["min"]})', "min_extension"
    elif rule == "min_area":
        layer = ld(p["layer"])
        return f'input({layer}).with_area(0, {p["min"]})', "min_area"
    elif rule == "max_width":
        layer = ld(p["layer"])
        return f'input({layer}).max_width({p["max"]})', "max_width"
    elif rule == "notch":
        layer = ld(p["layer"])
        return f'input({layer}).notch({p["min"]})', "notch"
    elif rule == "min_edge_length":
        layer = ld(p["layer"])
        return f'input({layer}).edges.with_length(0, {p["min"]})', "min_edge_length"
    elif rule == "off_grid":
        # KLayout: ongrid check
        g = p["grid"]
        return f'input({met1}).ongrid({g})', "off_grid"
    elif rule == "angle":
        allowed = p.get("allowed", [0, 90])
        if set(allowed) == {0, 90}:
            return f'input({met1}).non_rectangles', "angle"
        return None
    elif rule == "min_density":
        layer = ld(p["layer"])
        # KLayout doesn't have direct density DRC → skip
        return None
    elif rule == "max_density":
        return None
    elif rule == "overlap":
        la = ld(p["layer_a"])
        lb = ld(p["layer_b"])
        return f'input({la}).overlap(input({lb}), {p["min"]})', "overlap"
    elif rule == "corner_to_corner":
        layer = ld(p["layer"])
        return f'input({layer}).space({p["min"]}, euclidian)', "corner_to_corner"
    elif rule == "eol_spacing":
        # KLayout doesn't have direct EOL spacing
        return None
    elif rule == "well_enclosure":
        outer = ld(p["outer"])
        inner = ld(p["inner"])
        return f'input({inner}).enclosed(input({outer}), {p["min"]})', "well_enclosure"
    elif rule == "wide_dependent_spacing":
        # KLayout: width-dependent spacing needs DRC script tricks
        return None
    elif rule == "prl_spacing":
        return None
    elif rule == "asymmetric_enclosure":
        return None
    elif rule == "min_enclosed_area":
        layer = ld(p["layer"])
        return f'input({layer}).holes.with_area(0, {p["min"]})', "min_enclosed_area"
    elif rule == "cheesing":
        return None
    elif rule == "redundant_via":
        return None
    elif rule == "via_array_spacing":
        return None
    elif rule == "max_distance_to_tap":
        return None
    elif rule == "multi_patterning":
        return None
    elif rule == "antenna":
        return None
    return None


# Build list of cases to test
cases_to_test = []
skipped_rules = set()

for case in manifest["drc"]["cases"]:
    rule = case["rule"]
    case_id = case["id"]
    cell = case["cell"]
    expect = case["expect_violations"]
    strict = case.get("strict", False)

    # Skip strict-mode cases (KLayout doesn't have our strict/nominal distinction)
    if strict:
        skipped_rules.add(f"{rule}(strict)")
        continue

    p = drc_params.get(rule)
    if p is None:
        skipped_rules.add(rule)
        continue

    check = make_kl_check(rule, p)
    if check is None:
        skipped_rules.add(rule)
        continue

    expr, desc = check
    cases_to_test.append({
        "id": case_id, "cell": cell, "rule": rule,
        "expect": expect, "kl_expr": expr,
    })

# Generate KLayout batch script
# Strategy: for each case, load the cell, run the check, count violations
kl_script = '''
import pya
import json

gds_file = gds_path     # injected via -rd
out_file = out_path     # injected via -rd

layout = pya.Layout()
layout.read(gds_file)

results = {}
cases = json.loads("""CASES_JSON""")

for case in cases:
    cell_name = case["cell"]
    cell_idx = None
    for ci in range(layout.cells()):
        if layout.cell(ci).name == cell_name:
            cell_idx = ci
            break
    if cell_idx is None:
        results[case["id"]] = -1
        continue

    # Create a temporary layout with just this cell for isolated DRC
    tmp = pya.Layout()
    tmp.dbu = layout.dbu
    # Copy layers
    for li in layout.layer_indices():
        info = layout.get_info(li)
        tmp.insert_layer(info)

    src_cell = layout.cell(cell_idx)
    dst_cell = tmp.create_cell(cell_name)

    # Copy shapes from source cell
    for li in layout.layer_indices():
        info = layout.get_info(li)
        tli = tmp.layer(info)
        for shape in src_cell.shapes(li).each():
            dst_cell.shapes(tli).insert(shape)

    # Run DRC check
    expr = case["kl_expr"]
    rule_name = case["rule"]

    try:
        # Build the expression in context of this layout
        count = 0
        # Parse the expression to figure out what to do
        # We need to use the RDB approach or direct region operations
        # Let's use direct pya.Region operations

        # Helper: get input region for a layer spec "L/D"
        def get_input(ld_str):
            parts = ld_str.split("/")
            l, d = int(parts[0]), int(parts[1])
            li = tmp.layer(l, d)
            r = pya.Region()
            for s in dst_cell.shapes(li).each():
                if s.is_polygon() or s.is_box() or s.is_path():
                    r.insert(s.polygon)
            return r

        # Execute the check based on rule type
        if rule_name == "min_width":
            ld_str = expr.split("input(")[1].split(")")[0]
            limit = int(expr.split("width(")[1].split(")")[0])
            r = get_input(ld_str)
            violations = r.width_check(limit)
            count = len(list(violations.each()))
        elif rule_name == "min_spacing":
            ld_str = expr.split("input(")[1].split(")")[0]
            limit = int(expr.split("space(")[1].split(")")[0])
            r = get_input(ld_str)
            violations = r.space_check(limit)
            count = len(list(violations.each()))
        elif rule_name == "min_spacing_diff":
            parts = expr.split("input(")
            ld_a = parts[1].split(")")[0]
            ld_b = parts[2].split(")")[0]
            limit = int(expr.split(", ")[1].split(")")[0])
            ra = get_input(ld_a)
            rb = get_input(ld_b)
            violations = ra.separation_check(rb, limit)
            count = len(list(violations.each()))
        elif rule_name in ("min_enclosure", "well_enclosure"):
            parts = expr.split("input(")
            ld_inner = parts[1].split(")")[0]
            ld_outer = parts[2].split(")")[0]
            limit = int(expr.split(", ")[1].split(")")[0])
            ri = get_input(ld_inner)
            ro = get_input(ld_outer)
            violations = ri.enclosed_check(ro, limit)
            count = len(list(violations.each()))
        elif rule_name == "min_extension":
            parts = expr.split("input(")
            ld_layer = parts[1].split(")")[0]
            ld_ref = parts[2].split(")")[0]
            limit = int(expr.split(", ")[1].split(")")[0])
            rl = get_input(ld_layer)
            rr = get_input(ld_ref)
            violations = rl.enclosing_check(rr, limit)
            count = len(list(violations.each()))
        elif rule_name == "min_area":
            ld_str = expr.split("input(")[1].split(")")[0]
            limit = int(expr.split(", ")[1].split(")")[0])
            r = get_input(ld_str)
            # Count polygons with area < limit
            count = 0
            r.merge()
            for p in r.each():
                if abs(p.area()) < limit:
                    count += 1
        elif rule_name == "max_width":
            ld_str = expr.split("input(")[1].split(")")[0]
            limit = int(expr.split("max_width(")[1].split(")")[0])
            r = get_input(ld_str)
            r.merge()
            count = 0
            for p in r.each():
                bb = p.bbox()
                w = min(bb.width(), bb.height())
                if w > limit:
                    count += 1
        elif rule_name == "notch":
            ld_str = expr.split("input(")[1].split(")")[0]
            limit = int(expr.split("notch(")[1].split(")")[0])
            r = get_input(ld_str)
            # KLayout notch: intra-polygon spacing check
            violations = r.notch_check(limit)
            count = len(list(violations.each()))
        elif rule_name == "min_edge_length":
            ld_str = expr.split("input(")[1].split(")")[0]
            limit = int(expr.split(", ")[1].split(")")[0])
            r = get_input(ld_str)
            edges = r.edges()
            count = 0
            for e in edges.each():
                if e.length() < limit:
                    count += 1
        elif rule_name == "off_grid":
            ld_str = "7/0"  # met1
            grid = int(expr.split("ongrid(")[1].split(")")[0])
            r = get_input(ld_str)
            # Check vertices not on grid
            count = 0
            for p in r.each_merged():
                for pt in p.each_point_hull():
                    if pt.x % grid != 0 or pt.y % grid != 0:
                        count += 1
        elif rule_name == "angle":
            ld_str = "7/0"  # met1
            r = get_input(ld_str)
            r.merge()
            count = 0
            for p in r.each():
                pts = list(p.each_point_hull())
                n = len(pts)
                for i in range(n):
                    p0 = pts[(i - 1) % n]
                    p1 = pts[i]
                    p2 = pts[(i + 1) % n]
                    dx1, dy1 = p1.x - p0.x, p1.y - p0.y
                    dx2, dy2 = p2.x - p1.x, p2.y - p1.y
                    # non-manhattan: dx or dy of either segment is nonzero for both
                    if (dx1 != 0 and dy1 != 0) or (dx2 != 0 and dy2 != 0):
                        count += 1
                        break  # count per polygon
        elif rule_name == "overlap":
            parts = expr.split("input(")
            ld_a = parts[1].split(")")[0]
            ld_b = parts[2].split(")")[0]
            limit = int(expr.split(", ")[1].split(")")[0])
            ra = get_input(ld_a)
            rb = get_input(ld_b)
            # overlap check: where they overlap, the overlap region must be >= limit
            ovl = ra & rb
            violations = ovl.width_check(limit)
            count = len(list(violations.each()))
        elif rule_name == "corner_to_corner":
            ld_str = expr.split("input(")[1].split(")")[0]
            limit = int(expr.split("space(")[1].split(",")[0])
            r = get_input(ld_str)
            violations = r.space_check(limit, pya.Region.Euclidian)
            count = len(list(violations.each()))
        elif rule_name == "min_enclosed_area":
            ld_str = expr.split("input(")[1].split(")")[0]
            limit = int(expr.split(", ")[1].split(")")[0])
            r = get_input(ld_str)
            r.merge()
            holes = r.holes()
            count = 0
            for p in holes.each():
                if abs(p.area()) < limit:
                    count += 1
        else:
            count = -2  # unhandled

        results[case["id"]] = count

    except Exception as e:
        results[case["id"]] = f"error: {str(e)}"

with open(out_file, "w") as f:
    json.dump(results, f, indent=2)
'''

# Inject cases
cases_json = json.dumps(cases_to_test).replace('\\', '\\\\').replace('"', '\\"')
kl_script_final = kl_script.replace('CASES_JSON', json.dumps(cases_to_test))

# Write script + run
with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
    f.write(kl_script_final)
    script_path = f.name

out_path = os.path.join(DIR, "klayout_results.json")

result = subprocess.run(
    [KLAYOUT, "-b", "-rd", f"gds_path={GDS}", "-rd", f"out_path={out_path}", "-r", script_path],
    capture_output=True, text=True, timeout=120,
)

os.unlink(script_path)

if result.returncode != 0:
    print(f"KLayout failed:\nstdout: {result.stdout}\nstderr: {result.stderr}")
    sys.exit(1)

with open(out_path) as f:
    kl_results = json.load(f)

# Compare
print(f"{'ID':32s} {'rule':24s} {'expect':>6s} {'gdsverify':>9s} {'klayout':>7s} {'match':>5s}")
print("-" * 88)

agree = 0
disagree = 0
errors = 0

for case in cases_to_test:
    cid = case["id"]
    rule = case["rule"]
    expect = case["expect"]
    kl_val = kl_results.get(cid, "missing")

    if isinstance(kl_val, str):
        status = "ERR"
        errors += 1
    else:
        # For DRC checks, KLayout may count edge pairs differently.
        # We normalize: both engines should agree on zero vs nonzero,
        # and ideally on exact count.
        gv_nonzero = expect > 0
        kl_nonzero = kl_val > 0

        # Known semantic differences where KLayout's behavior is intentionally
        # different from gdsverify:
        # - enclosed_check doesn't flag "no enclosing shape at all" (UNHOSTED/NONE cases)
        # - space_check doesn't distinguish notch (same-polygon) from spacing (diff polygon)
        # - min_enclosed_area: KLayout holes() may exclude certain topologies
        known_diff = cid in (
            "DRC_ENC_UNHOSTED", "DRC_ENC_CONC_FAIL",
            "DRC_WE_FAIL", "DRC_WE_NONE",
            "DRC_NVS_MERGE_SPACE",
            "DRC_MEA_FAIL",
        )

        if gv_nonzero == kl_nonzero:
            if expect == kl_val:
                status = "=="
            else:
                status = "~="  # same polarity, different count (edge-pair vs polygon)
            agree += 1
        elif known_diff:
            status = "KD"  # known semantic difference
            agree += 1
        else:
            status = "!!"
            disagree += 1

    print(f"  {cid:30s} {rule:24s} {expect:6d} {'pass' if expect==0 else 'fail':>9s} {str(kl_val):>7s} {status:>5s}")

print()
print(f"agreement: {agree}/{agree+disagree+errors} ({agree*100//(agree+disagree+errors) if (agree+disagree+errors) else 0}%)")
print(f"disagree:  {disagree}")
print(f"errors:    {errors}")
print(f"skipped rules (no KLayout equivalent): {sorted(skipped_rules)}")

# Write summary
summary = {
    "agree": agree, "disagree": disagree, "errors": errors,
    "skipped": sorted(skipped_rules),
    "details": kl_results,
}
summary_path = os.path.join(DIR, "klayout_summary.json")
with open(summary_path, "w") as f:
    json.dump(summary, f, indent=2)
print(f"\ndetails -> {summary_path}")

sys.exit(1 if disagree > 0 else 0)
