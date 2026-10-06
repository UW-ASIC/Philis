'use strict';
(() => {
  const field = document.getElementById('notes-search');
  const results = document.getElementById('search-results');
  if (field && results) {
    const records = window.PHILIS_SEARCH || [];
    field.addEventListener('input', () => {
      const terms = field.value.trim().toLowerCase().split(/\s+/).filter(Boolean);
      results.replaceChildren();
      if (!terms.length) return;
      const matches = records.filter(r => terms.every(t => `${r.title} ${r.text}`.toLowerCase().includes(t)));
      const count = document.createElement('p');
      count.className = 'search-count';
      count.textContent = `${matches.length} matching sections${matches.length > 40 ? ' · showing first 40' : ''}`;
      results.append(count);
      matches.slice(0, 40).forEach(r => {
        const article = document.createElement('div');
        article.className = 'search-result';
        const link = document.createElement('a');
        link.href = r.url;
        link.textContent = r.title;
        const p = document.createElement('p');
        const at = Math.max(0, r.text.toLowerCase().indexOf(terms[0]) - 90);
        p.textContent = `${at ? '…' : ''}${r.text.slice(at, at + 300)}${r.text.length > at + 300 ? '…' : ''}`;
        article.append(link, p);
        results.append(article);
      });
    });
  }
})();
