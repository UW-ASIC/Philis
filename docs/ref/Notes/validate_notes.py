#!/usr/bin/env python3
"""Check the offline handbook's links, structure, source identities and SVG assets.

Usage: python3 ref/Notes/validate_notes.py [--hash-sources]
This validates documentation artifacts, not PDK rules or engine performance.
No third-party Python packages are required.
"""
from __future__ import annotations
import argparse
import hashlib
from html.parser import HTMLParser
import json
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit
import xml.etree.ElementTree as ET

BASE = Path(__file__).resolve().parent
VOID = set('area base br col embed hr img input link meta param source track wbr'.split())
SVG_NS = '{http://www.w3.org/2000/svg}'


class Document(HTMLParser):
    def __init__(self, name: str, data: str):
        super().__init__(convert_charrefs=True)
        self.name, self.ids, self.links, self.resources = name, set(), [], []
        self.stack, self.errors, self.headings = [], [], []
        self.counts = {}
        self.feed(data)
        self.close()
        if self.stack:
            self.errors.append(f'unclosed tags: {self.stack}')
        for tag in ('html', 'head', 'body', 'main', 'h1'):
            if self.counts.get(tag, 0) != 1:
                self.errors.append(f'expected one {tag}, got {self.counts.get(tag, 0)}')
        if '{{' in data or '}}' in data:
            self.errors.append('unresolved authoring placeholder')

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        self.counts[tag] = self.counts.get(tag, 0) + 1
        if a.get('id'):
            if a['id'] in self.ids:
                self.errors.append(f'duplicate id {a["id"]}')
            self.ids.add(a['id'])
        if tag == 'a' and a.get('href') is not None:
            self.links.append(a['href'])
        if a.get('src'):
            self.resources.append(a['src'])
        if tag == 'link' and a.get('href'):
            self.resources.append(a['href'])
        if tag == 'html' and a.get('lang') != 'en':
            self.errors.append('missing English language declaration')
        if tag not in VOID:
            self.stack.append(tag)

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)
        if tag not in VOID:
            self.handle_endtag(tag)

    def handle_endtag(self, tag):
        if tag in VOID:
            return
        if not self.stack or self.stack[-1] != tag:
            self.errors.append(f'improper closing tag {tag}; stack ends {self.stack[-4:]}')
            if tag in self.stack:
                self.stack = self.stack[:self.stack.index(tag)]
        else:
            self.stack.pop()


def validate(hash_sources=False):
    manifest = json.loads((BASE / 'sources.json').read_text())
    book = json.loads((BASE / 'handbook-manifest.json').read_text())
    sources = {(BASE.parent / s['filename']).resolve(): s for s in manifest['sources']}
    errors = []
    docs = {}
    for path in sorted(BASE.glob('*.html')):
        doc = Document(path.name, path.read_text())
        docs[path.resolve()] = doc
        errors += [f'{path.name}: {e}' for e in doc.errors]
    links = pdf_links = 0
    for path, doc in docs.items():
        for url in doc.links + doc.resources:
            parts = urlsplit(url)
            if parts.scheme or parts.netloc:
                if url in doc.resources:
                    errors.append(f'{path.name}: online dependency {url}')
                continue
            target = (path.parent / unquote(parts.path)).resolve() if parts.path else path
            links += 1
            if not target.exists():
                errors.append(f'{path.name}: missing local target {url}')
                continue
            fragment = unquote(parts.fragment)
            if fragment and target.suffix == '.html':
                if target not in docs or fragment not in docs[target].ids:
                    errors.append(f'{path.name}: missing HTML anchor {url}')
            if target.suffix.lower() == '.pdf':
                pdf_links += 1
                if target not in sources:
                    errors.append(f'{path.name}: PDF absent from source manifest: {url}')
                elif fragment:
                    m = re.fullmatch(r'page=(\d+)', fragment)
                    if not m or not (1 <= int(m[1]) <= sources[target]['pages']):
                        errors.append(f'{path.name}: invalid physical PDF page {url}')
    for path, source in sources.items():
        if not path.is_file():
            errors.append(f'missing source {source["filename"]}')
        elif hash_sources:
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            if digest != source['sha256']:
                errors.append(f'changed source identity {source["id"]}')
    svgs = list((BASE / 'diagrams').glob('*.svg'))
    for path in svgs:
        try:
            root = ET.fromstring(path.read_text())
            if root.tag != SVG_NS + 'svg':
                errors.append(f'{path.name}: missing SVG namespace')
            view = [float(x) for x in root.get('viewBox', '').split()]
            if len(view) != 4 or view[2] <= 0 or view[3] <= 0:
                errors.append(f'{path.name}: invalid viewBox')
            if root.find(SVG_NS + 'title') is None or root.find(SVG_NS + 'desc') is None:
                errors.append(f'{path.name}: missing title/description')
            ids = [x.get('id') for x in root.iter() if x.get('id')]
            if len(ids) != len(set(ids)):
                errors.append(f'{path.name}: duplicate SVG ids')
            for el in root.iter(SVG_NS + 'rect'):
                if float(el.get('width', '0')) <= 0 or float(el.get('height', '0')) <= 0:
                    errors.append(f'{path.name}: nonpositive rectangle size')
        except (ET.ParseError, ValueError) as exc:
            errors.append(f'{path.name}: invalid SVG: {exc}')
    search_text = (BASE / 'search-data.js').read_text()
    data = json.loads(search_text.removeprefix('window.PHILIS_SEARCH = ').rstrip().removesuffix(';'))
    for record in data:
        parts = urlsplit(record['url'])
        target = (BASE / parts.path).resolve()
        if target not in docs or parts.fragment not in docs[target].ids:
            errors.append(f'search index: missing {record["url"]}')
    for source in manifest['sources']:
        if (BASE / source['notes']).resolve() not in docs:
            errors.append(f'missing source companion {source["id"]}')
    if len(book['chapters']) != 30 or len(manifest['sources']) != 13:
        errors.append('incorrect chapter/source count')
    constraints = (BASE / 'constraints.html').read_text()
    # IDs use a source-independent mechanism prefix and a numeric suffix.
    constraint_ids = re.findall(r'<tr\s+id="([^"]+)"', constraints)
    if len(constraint_ids) != 84:
        errors.append(f'expected 84 constraint rows, found {len(constraint_ids)}')
    # Reference integrity is mechanical; it does not repeat the semantic review.
    provenance = json.loads((BASE / 'theory-provenance.json').read_text())
    ledger = json.loads((BASE / 'citation-audit.json').read_text())
    audit = json.loads((BASE / 'audit-report.json').read_text())
    record_ids = [r['id'] for r in provenance['records']]
    if len(record_ids) != len(set(record_ids)):
        errors.append('duplicate theory record identity')
    source_by_id = {s['id']: s for s in manifest['sources']}
    for record in provenance['records']:
        if record['id'] not in docs[(BASE / 'theory-provenance.html').resolve()].ids:
            errors.append(f'missing theory record anchor {record["id"]}')
        if record['kind'] not in {'source', 'derived', 'foundation', 'proposal'}:
            errors.append(f'unknown provenance type {record["id"]}')
        source_id = record.get('source')
        if source_id:
            source = source_by_id.get(source_id)
            if source is None or record['source_sha256'] != source['sha256']:
                errors.append(f'theory source identity mismatch {record["id"]}')
            elif any(not isinstance(n, int) or not 1 <= n <= source['pages']
                     for n in record['pdf_pages']):
                errors.append(f'theory PDF locator out of bounds {record["id"]}')
        elif record['kind'] == 'source':
            errors.append(f'source result without a source {record["id"]}')
    mappings = provenance['constraint_map']
    if {r['id'] for r in mappings} != set(constraint_ids) or len(mappings) != 84:
        errors.append('constraint-to-theory mapping is incomplete or duplicated')
    for mapping in mappings:
        if not mapping['records'] or any(r not in record_ids for r in mapping['records']):
            errors.append(f'unknown constraint theory reference {mapping["id"]}')
    citation_ids = [r['citation_id'] for r in ledger['reviews']]
    if len(citation_ids) != len(set(citation_ids)) or len(citation_ids) != audit['original_citation_occurrences']:
        errors.append('original citation ledger identity/count mismatch')
    counts = {}
    for record in ledger['reviews']:
        counts[record['status']] = counts.get(record['status'], 0) + 1
    if counts != ledger['status_counts'] or counts != audit['citation_status_counts']:
        errors.append('citation audit status totals disagree')
    if len(record_ids) != audit['theory_records']:
        errors.append('theory audit record count mismatch')
    for name, expected in audit.get('audited_article_sha256', {}).items():
        article_data = (BASE / name).read_text()
        article = re.search(r'<article>([\s\S]*?)</article>', article_data)
        if article is None or hashlib.sha256(article[1].encode()).hexdigest() != expected:
            errors.append(f'article changed since semantic audit: {name}')
    return {
        'artifact_checks_passed': not errors,
        'html_pages': len(docs), 'teaching_chapters': len(book['chapters']),
        'source_pdfs': len(sources), 'source_physical_pages': manifest['total_physical_pages'],
        'source_hashes_checked': hash_sources, 'local_references_checked': links,
        'pdf_page_links_checked': pdf_links, 'standalone_svg_files': len(svgs),
        'search_sections': len(data), 'constraint_entries': len(constraint_ids),
        'theory_records_checked': len(record_ids),
        'constraint_theory_mappings_checked': len(mappings),
        'original_citation_audit_entries_checked': len(citation_ids),
        'errors': errors,
        'scope': 'Structural documentation validation only. Does not certify engine behavior, foundry rules, source mathematics or browser layout.',
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hash-sources', action='store_true')
    args = parser.parse_args()
    result = validate(args.hash_sources)
    # Preserve separately reported runtime/visual checks on structural reruns.
    report_path = BASE / 'validation-report.json'
    if report_path.exists():
        previous = json.loads(report_path.read_text())
        for key in ('javascript_checks', 'visual_checks', 'engine_changes'):
            if key in previous:
                result[key] = previous[key]
    report_path.write_text(json.dumps(result, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps(result, indent=2, ensure_ascii=False))
    raise SystemExit(0 if result['artifact_checks_passed'] else 1)
