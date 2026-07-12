#!/usr/bin/env python3
"""Validate identities and arithmetic that JSON Schema cannot express."""

from datetime import date
from decimal import Decimal, ROUND_HALF_UP
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
BASE = ROOT / "docs" / "verification" / "compliance"
DATA = BASE / "capabilities.json"
SCHEMA = BASE / "capabilities.schema.json"


class Invalid(Exception):
    pass


def no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise Invalid(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def load(path):
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            parse_float=Decimal,
            object_pairs_hook=no_duplicates,
        )
    except (OSError, json.JSONDecodeError, Invalid) as error:
        raise Invalid(f"{path.relative_to(ROOT)}: {error}") from error
    if not isinstance(value, dict):
        raise Invalid(f"{path.relative_to(ROOT)}: root must be an object")
    return value


def keys(value, expected, context):
    if not isinstance(value, dict):
        raise Invalid(f"{context}: expected object")
    actual = set(value)
    expected = set(expected)
    if actual != expected:
        raise Invalid(
            f"{context}: missing={sorted(expected - actual)}, extra={sorted(actual - expected)}"
        )


def integer(value, context):
    if isinstance(value, bool) or not isinstance(value, int):
        raise Invalid(f"{context}: expected integer, got {value!r}")
    return value


def number(value, context):
    if isinstance(value, bool) or not isinstance(value, (int, Decimal)):
        raise Invalid(f"{context}: expected number, got {value!r}")
    return Decimal(value)


def percent(implemented, total):
    return int(
        (implemented * Decimal(100) / Decimal(total)).quantize(
            Decimal("1"), rounding=ROUND_HALF_UP
        )
    )


def validate():
    data = load(DATA)
    schema = load(SCHEMA)
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise Invalid("schema: expected JSON Schema draft 2020-12")
    keys(
        data,
        {
            "$schema", "schema_version", "status_date", "baseline_commit",
            "score_policy", "states", "summary", "engines", "external_prerequisites",
        },
        "audit",
    )
    if data["$schema"] != SCHEMA.name or data["schema_version"] != "gdsverify.capabilities/v1":
        raise Invalid("audit: wrong schema name/version")
    try:
        date.fromisoformat(data["status_date"])
    except (TypeError, ValueError) as error:
        raise Invalid("audit: status_date must be YYYY-MM-DD") from error
    if not isinstance(data["baseline_commit"], str) or not re.fullmatch(
        r"[0-9a-f]{40}", data["baseline_commit"]
    ):
        raise Invalid("audit: baseline_commit must be a lowercase 40-digit SHA")

    states = [
        "absent", "foundation", "partial", "integration_pending", "correlated", "qualified"
    ]
    if data["states"] != states:
        raise Invalid(f"audit: states must be exactly {states}")

    engine_ids = schema["properties"]["engines"]["required"]
    if engine_ids != ["drc", "lvs", "pex"]:
        raise Invalid("schema: engine identity/order must be drc, lvs, pex")
    if set(schema["properties"]["engines"]["properties"]) != set(engine_ids):
        raise Invalid("schema: engine properties and required identities differ")
    engines = data["engines"]
    keys(engines, engine_ids, "engines")
    all_score = Decimal(0)
    all_families = 0

    for engine_id in engine_ids:
        engine = engines[engine_id]
        keys(engine, {"implemented", "total", "percent", "claim", "groups"}, engine_id)
        if not isinstance(engine["claim"], str) or not engine["claim"].strip():
            raise Invalid(f"{engine_id}: claim must be nonempty")
        group_ids = schema["$defs"][f"{engine_id}_groups"]["required"]
        group_schema = schema["$defs"][f"{engine_id}_groups"]
        if group_schema.get("additionalProperties") is not False or set(
            group_schema["properties"]
        ) != set(group_ids):
            raise Invalid(f"schema: {engine_id} group properties/required identities differ")
        groups = engine["groups"]
        keys(groups, group_ids, f"{engine_id}.groups")
        score = Decimal(0)
        families = 0
        for group_id in group_ids:
            group = groups[group_id]
            context = f"{engine_id}.{group_id}"
            keys(group, {"families", "score", "state"}, context)
            group_families = integer(group["families"], f"{context}.families")
            group_score = number(group["score"], f"{context}.score")
            if group_families <= 0 or not 0 <= group_score <= group_families:
                raise Invalid(f"{context}: invalid families/score")
            if group["state"] not in states:
                raise Invalid(f"{context}: unknown state {group['state']!r}")
            if (group_score == 0) != (group["state"] == "absent"):
                raise Invalid(f"{context}: zero score and absent state must agree")
            families += group_families
            score += group_score

        declared_score = number(engine["implemented"], f"{engine_id}.implemented")
        declared_total = integer(engine["total"], f"{engine_id}.total")
        declared_percent = integer(engine["percent"], f"{engine_id}.percent")
        if (score, families, percent(score, families)) != (
            declared_score, declared_total, declared_percent
        ):
            raise Invalid(f"{engine_id}: group score/total/percent do not match declarations")

        constraints = schema["$defs"][f"{engine_id}_engine"]["allOf"][1]["properties"]
        for field in ("implemented", "total", "percent"):
            if number(engine[field], f"{engine_id}.{field}") != number(
                constraints[field]["const"], f"schema.{engine_id}.{field}"
            ):
                raise Invalid(f"{engine_id}: {field} conflicts with schema")
        all_score += score
        all_families += families

    summary = data["summary"]
    keys(summary, {"implemented", "total", "percent", "qualification"}, "summary")
    if (
        number(summary["implemented"], "summary.implemented"),
        integer(summary["total"], "summary.total"),
        integer(summary["percent"], "summary.percent"),
    ) != (all_score, all_families, percent(all_score, all_families)):
        raise Invalid("summary: engine score/total/percent do not match")
    if summary["qualification"] != "unqualified":
        raise Invalid("summary: qualification requires a separate external acceptance review")
    summary_schema = schema["properties"]["summary"]["properties"]
    for field, value in summary.items():
        expected = summary_schema[field]["const"]
        mismatch = (
            value != expected
            if field == "qualification"
            else number(value, f"summary.{field}")
            != number(expected, f"schema.summary.{field}")
        )
        if mismatch:
            raise Invalid(f"summary: {field} conflicts with schema")

    external = data["external_prerequisites"]
    if (
        not isinstance(external, list)
        or not external
        or any(not isinstance(item, str) or not item.strip() for item in external)
        or len(external) != len(set(external))
    ):
        raise Invalid("external_prerequisites must be unique nonempty strings")


def main():
    try:
        validate()
    except (KeyError, TypeError, Invalid) as error:
        print(f"capability validation: FAIL: {error}", file=sys.stderr)
        return 1
    print("capability validation: PASS (identities, states, sums and percentages)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
