#!/usr/bin/env python3
"""Restart arguments for docs/plans/workflows-m2only.js after a crash or stop.

    python3 docs/plans/restart_args.py <journal.jsonl> [<journal.jsonl> ...] > args.json

Never `resumeFromRunId` this workflow: resume matches agents by call order, and
parallel modules finish in a different order every run, so finished agents re-run.
Instead this reads what already happened and launches fresh:

- merged:   items in an "M2+ <module> <n>: merge (...)" commit on m2 (git is the truth);
- pending:  per module, segments a journal shows hardened but not merged, with the
            card path and the furthest step that finished (exec / review / fix), so the
            new run starts each segment at the next step and repeats no finished agent;
- preset:   items a hardener genuinely deferred (not ones it skipped on a relayed
            chat message);
- segStart: highest card number per module, so new cards never overwrite old ones.
Journals are applied in the order given (later runs override earlier ones).
"""
import json, os, re, subprocess, sys

W2 = "/home/omare/Documents/Projects/Rust/philis-m2"
BAIL = re.compile(r"relayed|user message|not started|confirm|cached replay", re.I)


def merged_items():
    out = subprocess.run(["git", "-C", f"{W2}/integrate", "log", "--format=%s", "main..m2"],
                         capture_output=True, text=True, check=True).stdout
    ids = set()
    for line in out.splitlines():
        m = re.match(r"M2\+ [a-z-]+ \d+: merge \(([^)]*)\)", line)
        if m:
            ids.update(x.strip() for x in m.group(1).split(","))
    return ids


def main():
    merged = merged_items()
    seg = {}  # tag -> {harden, exec, review, fix, integrate}
    for path in sys.argv[1:]:
        lab = {}
        for line in open(path):
            try:
                d = json.loads(line)
            except ValueError:
                continue
            if d["type"] == "started":
                lab[d["key"]] = d["label"]
            elif d["type"] == "result" and d["key"] in lab:
                step, tag = lab[d["key"]].split(":", 1)
                if "#" in tag:
                    seg.setdefault(tag, {})[step] = d["result"]
    pending, preset = {}, {}
    for tag, s in seg.items():
        h = s.get("harden")
        if not h:
            continue
        mod, n = tag.split("#")
        for it in h["items"]:
            if it["cls"] == "defer" and it["id"] not in merged and not BAIL.search(it.get("reason", "")):
                preset[it["id"]] = "deferred: " + it.get("reason", "")[:240]
        ids = [it["id"] for it in h["items"] if it["cls"] != "defer" and it["id"] not in merged]
        if not ids or not os.path.exists(h["card_path"]):
            continue
        integ = s.get("integrate")
        if integ and integ.get("status") == "merged":
            continue
        entry = {"seg": int(n), "card": h["card_path"], "ids": ids}
        ex = s.get("exec")
        # an exec that bailed on a relayed message did nothing: redo exec
        if ex and any(r["status"] == "done" for r in ex["results"]) and not all(BAIL.search(r.get("note", "")) for r in ex["results"] if r["status"] != "done"):
            entry["exec"] = ex
            if s.get("review") is not None:
                entry["review"] = s["review"]
                entry["fixDone"] = "fix" in s or not s["review"].get("issues")
        pending.setdefault(mod, []).append(entry)
    for m in pending:
        pending[m].sort(key=lambda e: e["seg"])
    start = {}
    for m in os.listdir(W2):
        if not os.path.isdir(f"{W2}/{m}"):
            continue
        nums = [0]
        for wt in (m, "integrate"):
            d = f"{W2}/{wt}/docs/plans/cards"
            if os.path.isdir(d):
                nums += [int(x.group(1)) for f in os.listdir(d) if (x := re.match(rf"m2-{re.escape(m)}-(\d+)\.md$", f))]
        if m != "integrate":
            start[m] = max(nums)
    json.dump({"cap2": 15000000, "merged": sorted(merged), "segStart": start, "pending": pending, "preset": preset}, sys.stdout)


if __name__ == "__main__":
    main()
