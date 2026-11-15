// OyNIx website: small progressive enhancements. Every page works without it.
(function () {
  'use strict';

  // Releases are displayed on this site, but the assets remain hosted by GitHub.
  // Link directly to each asset's browser_download_url; never route through a
  // GitHub release/tag page.
  var releaseApi = 'https://api.github.com/repos/trail-b1az3r/OyNIx/releases?per_page=100';

  function escapeHtml(value) {
    return String(value == null ? '' : value).replace(/[&<>"']/g, function (c) {
      return ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c];
    });
  }

  function versionParts(release) {
    var tag = String(release.tag_name || release.name || '').replace(/^v/i, '');
    var core = tag.split('-')[0];
    return core.split('.').map(function (part) {
      var n = parseInt(part, 10);
      return Number.isFinite(n) ? n : 0;
    });
  }

  function isLegacy(release) {
    var v = versionParts(release);
    return v[0] < 4 || (v[0] === 4 && (v[1] || 0) <= 3);
  }

  function formatSize(bytes) {
    if (!Number.isFinite(bytes) || bytes <= 0) return '';
    var units = ['B', 'KB', 'MB', 'GB'];
    var n = bytes, i = 0;
    while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
    return (i ? n.toFixed(1) : String(n)) + ' ' + units[i];
  }

  function renderRelease(release) {
    var tag = release.tag_name || release.name || 'Untitled release';
    var assets = Array.isArray(release.assets) ? release.assets : [];
    var date = release.published_at ? new Date(release.published_at) : null;
    var dateText = date && !Number.isNaN(date.getTime())
      ? date.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' }) : '';
    var links = assets.map(function (asset) {
      if (!asset.browser_download_url) return '';
      var label = escapeHtml(asset.name || 'Download');
      var size = formatSize(Number(asset.size));
      return '<a class="btn release-download" href="' + escapeHtml(asset.browser_download_url) +
        '" download rel="noopener noreferrer">' + label + (size ? '<small>' + escapeHtml(size) + '</small>' : '') + '</a>';
    }).join('');
    return '<article class="release-card">' +
      '<div class="release-heading"><div><h3>' + escapeHtml(release.name || tag) + '</h3>' +
      '<p class="release-meta">' + escapeHtml(tag) + (dateText ? ' · ' + escapeHtml(dateText) : '') +
      (release.prerelease ? ' · Pre-release' : '') + '</p></div></div>' +
      (links ? '<div class="release-assets">' + links + '</div>' : '<p class="release-empty">No downloadable assets are attached to this release.</p>') +
      '</article>';
  }

  function fillList(id, releases) {
    var target = document.getElementById(id);
    if (!target) return;
    target.innerHTML = releases.length ? releases.map(renderRelease).join('') :
      '<p class="release-empty">There are no releases in this channel.</p>';
  }

  function renderReleases(releases) {
    var published = releases.filter(function (r) { return !r.draft; });
    var legacy = published.filter(isLegacy);
    var current = published.filter(function (r) { return !isLegacy(r); });
    fillList('release-stable', current.filter(function (r) { return !r.prerelease; }));
    fillList('release-beta', current.filter(function (r) { return !!r.prerelease; }));
    fillList('release-legacy', legacy);
    var status = document.getElementById('release-status');
    if (status) status.textContent = published.length + ' releases available · ' +
      (current.length) + ' current and ' + legacy.length + ' legacy';
  }

  function loadReleases() {
    var status = document.getElementById('release-status');
    if (!status) return;
    var all = [], page = 1;
    function next() {
      return fetch(releaseApi + '&page=' + page, { headers: { Accept: 'application/vnd.github+json' } })
        .then(function (response) {
          if (!response.ok) throw new Error('GitHub returned HTTP ' + response.status);
          return response.json();
        })
        .then(function (batch) {
          if (!Array.isArray(batch)) throw new Error('Unexpected release response');
          all = all.concat(batch);
          if (batch.length === 100 && page < 10) { page++; return next(); }
          renderReleases(all);
        });
    }
    next().catch(function () {
      status.textContent = 'Releases could not be loaded right now. Please try again later.';
    });
  }

  loadReleases();

  // Mobile menu
  var menu = document.querySelector('.menu-btn'), tabs = document.querySelector('.tabs');
  if (menu && tabs) menu.addEventListener('click', function () {
    var open = tabs.classList.toggle('open');
    menu.setAttribute('aria-expanded', open);
  });

  // Copy buttons on code blocks marked data-copy
  document.querySelectorAll('pre[data-copy]').forEach(function (pre) {
    var b = document.createElement('button');
    b.className = 'copy'; b.type = 'button'; b.textContent = 'Copy';
    b.addEventListener('click', function () {
      var text = pre.querySelector('code').innerText.replace(/^\s*#.*$/gm, '').replace(/\n{2,}/g, '\n').trim();
      navigator.clipboard.writeText(text).then(function () {
        b.textContent = 'Copied'; setTimeout(function () { b.textContent = 'Copy'; }, 1400);
      });
    });
    pre.appendChild(b);
  });

  // Segmented buttons switching panels.
  document.querySelectorAll('.seg[data-panels]').forEach(function (seg) {
    var buttons = seg.querySelectorAll('button');
    function pick(id) {
      buttons.forEach(function (b) {
        var on = b.dataset.panel === id;
        b.setAttribute('aria-selected', on);
        var p = document.getElementById(b.dataset.panel);
        if (p) p.hidden = !on;
      });
    }
    buttons.forEach(function (b) { b.addEventListener('click', function () { pick(b.dataset.panel); }); });
  });

  // Annotated screenshot: pins and notes highlight each other.
  document.querySelectorAll('[data-tour]').forEach(function (tour) {
    var pins = tour.querySelectorAll('.pin');
    var notes = document.querySelectorAll('#' + tour.dataset.tour + ' li');
    function on(i) {
      pins.forEach(function (p, k) { p.classList.toggle('on', k === i); });
      notes.forEach(function (n, k) { n.classList.toggle('on', k === i); });
    }
    pins.forEach(function (p, i) {
      p.addEventListener('mouseenter', function () { on(i); });
      p.addEventListener('focus', function () { on(i); });
      p.addEventListener('click', function () { on(i); notes[i] && notes[i].scrollIntoView({ block: 'nearest', behavior: 'smooth' }); });
    });
    notes.forEach(function (n, i) { n.addEventListener('mouseenter', function () { on(i); }); });
    tour.addEventListener('mouseleave', function () { on(-1); });
  });

  // Extension Gallery: search and category chips.
  var grid = document.querySelector('.grid[data-filter]');
  if (grid) {
    var input = document.querySelector('.search input');
    var chips = document.querySelectorAll('.chip');
    var empty = document.querySelector('.empty');
    var cat = '';
    function apply() {
      var q = (input.value || '').trim().toLowerCase(), shown = 0;
      grid.querySelectorAll('.ext').forEach(function (e) {
        var ok = (!cat || e.dataset.category === cat) && (!q || e.dataset.text.indexOf(q) >= 0);
        e.hidden = !ok; if (ok) shown++;
      });
      if (empty) empty.hidden = shown > 0;
    }
    input.addEventListener('input', apply);
    chips.forEach(function (c) {
      c.addEventListener('click', function () {
        cat = c.getAttribute('aria-pressed') === 'true' ? '' : c.dataset.category;
        chips.forEach(function (x) { x.setAttribute('aria-pressed', x.dataset.category === cat && cat !== ''); });
        apply();
      });
    });
  }
})();
