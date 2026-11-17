// OyNIx website: small progressive enhancements. Every page works without it.
(function () {
  'use strict';

  var releaseApi = 'https://api.github.com/repos/trail-b1az3r/OyNIx/releases?per_page=100';
  var directAssetPrefix = 'https://github.com/trail-b1az3r/OyNIx/releases/download/';

  function escapeHtml(value) {
    return String(value == null ? '' : value).replace(/[&<>"']/g, function (c) {
      return ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c];
    });
  }

  function versionParts(release) {
    var raw = String(release.tag_name || release.name || '').trim().replace(/^v\s*/i, '');
    var match = raw.match(/(\d+)(?:\.(\d+))?(?:\.(\d+))?/);
    return match ? [Number(match[1]), Number(match[2] || 0), Number(match[3] || 0)] : [0, 0, 0];
  }

  function versionLabel(release) {
    var parts = versionParts(release);
    return parts.join('.');
  }

  function compareVersions(a, b) {
    var av = versionParts(a), bv = versionParts(b);
    for (var i = 0; i < 3; i++) {
      if (av[i] !== bv[i]) return bv[i] - av[i];
    }
    return String(b.tag_name || b.name || '').localeCompare(String(a.tag_name || a.name || ''), undefined, { numeric: true });
  }

  function isLegacy(release) {
    var v = versionParts(release);
    return v[0] < 4 || (v[0] === 4 && v[1] <= 3);
  }

  function formatSize(bytes) {
    if (!Number.isFinite(bytes) || bytes <= 0) return '';
    var units = ['B', 'KB', 'MB', 'GB'];
    var n = bytes, i = 0;
    while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
    return (i ? n.toFixed(1) : String(n)) + ' ' + units[i];
  }

  function assetUrl(asset) {
    var url = String(asset && asset.browser_download_url || '');
    return url.indexOf(directAssetPrefix) === 0 ? url : '';
  }

  function primaryAsset(assets) {
    var usable = assets.filter(function (a) {
      return assetUrl(a) && !/sha256|checksum|debug|symbols/i.test(a.name || '');
    });
    var priorities = [/\.deb$/i, /\.pkg\.tar\.zst$/i, /\.flatpak$/i, /\.rpm$/i, /\.appimage$/i, /\.exe$/i];
    for (var i = 0; i < priorities.length; i++) {
      var found = usable.find(function (a) { return priorities[i].test(a.name || ''); });
      if (found) return found;
    }
    return usable[0] || assets.find(function (a) { return assetUrl(a); }) || null;
  }

  function renderRelease(release) {
    var tag = release.tag_name || release.name || 'Untitled release';
    var assets = Array.isArray(release.assets) ? release.assets : [];
    var date = release.published_at ? new Date(release.published_at) : null;
    var dateText = date && !Number.isNaN(date.getTime())
      ? date.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' }) : '';
    var links = assets.map(function (asset) {
      var url = assetUrl(asset);
      if (!url) return '';
      var label = escapeHtml(asset.name || 'Download');
      var size = formatSize(Number(asset.size));
      return '<a class="btn release-download" href="' + escapeHtml(url) +
        '" rel="noopener noreferrer">' + label + (size ? '<small>' + escapeHtml(size) + '</small>' : '') + '</a>';
    }).join('');
    var primary = primaryAsset(assets);
    var primaryButton = primary ? '<a class="btn release-primary" href="' + escapeHtml(assetUrl(primary)) +
      '" rel="noopener noreferrer">Download ' + escapeHtml(primary.name || 'this version') + '</a>' : '';
    return '<article class="release-card">' +
      '<div class="release-heading"><div><h3>' + escapeHtml(release.name || tag) + '</h3>' +
      '<p class="release-meta">' + escapeHtml(tag) + (dateText ? ' · ' + escapeHtml(dateText) : '') +
      (release.prerelease ? ' · Pre-release' : '') + '</p></div></div>' +
      (primaryButton ? '<div class="release-primary-row">' + primaryButton + '</div>' : '') +
      (links ? '<div class="release-assets">' + links + '</div>' : '<p class="release-empty">No downloadable assets are attached to this release.</p>') +
      '</article>';
  }

  function fillList(id, releases) {
    var target = document.getElementById(id);
    if (!target) return;
    target.innerHTML = releases.length ? releases.map(renderRelease).join('') :
      '<p class="release-empty">There are no releases in this channel.</p>';
  }

  function renderStableVersions(releases) {
    var target = document.getElementById('release-stable');
    if (!target) return;
    var sorted = releases.slice().sort(compareVersions);
    if (!sorted.length) {
      target.innerHTML = '<p class="release-empty">There are no stable releases in this channel.</p>';
      return;
    }
    target.innerHTML = sorted.map(function (release) {
      var tag = release.tag_name || release.name || 'Release';
      return '<section class="release-version"><div class="release-version-heading"><h2>' +
        escapeHtml(release.name || versionLabel(release)) + '</h2><span>' +
        escapeHtml(tag) + '</span></div>' + renderRelease(release) + '</section>';
    }).join('');
  }

  function renderReleases(releases) {
    var published = releases.filter(function (r) { return r && !r.draft; });
    var legacy = published.filter(isLegacy).sort(compareVersions);
    var current = published.filter(function (r) { return !isLegacy(r); });
    renderStableVersions(current.filter(function (r) { return !r.prerelease; }));
    fillList('release-beta', current.filter(function (r) { return !!r.prerelease; }).sort(compareVersions));
    fillList('release-legacy', legacy);
    var status = document.getElementById('release-status');
    if (status) status.textContent = published.length + ' published releases · ' +
      current.filter(function (r) { return !r.prerelease; }).length + ' stable · ' +
      current.filter(function (r) { return !!r.prerelease; }).length + ' pre-release · ' +
      legacy.length + ' legacy';
  }

  function loadReleases() {
    var status = document.getElementById('release-status');
    if (!status) return;
    status.textContent = 'Loading releases…';
    var all = [], page = 1;
    function next() {
      return fetch(releaseApi + '&page=' + page, {
        headers: { Accept: 'application/vnd.github+json' },
        cache: 'no-store'
      }).then(function (response) {
        if (!response.ok) throw new Error('GitHub returned HTTP ' + response.status);
        return response.json();
      }).then(function (batch) {
        if (!Array.isArray(batch)) throw new Error('Unexpected release response');
        all = all.concat(batch);
        if (batch.length === 100 && page < 20) { page++; return next(); }
        renderReleases(all);
      });
    }
    next().catch(function () {
      status.innerHTML = 'Releases could not be loaded. Check your connection and <button class="text-link release-retry" type="button">try again</button>.';
      var retry = status.querySelector('.release-retry');
      if (retry) retry.addEventListener('click', loadReleases);
    });
  }

  // Only fetch release metadata on the Downloads page.
  loadReleases();

  // Mobile menu
  var menu = document.querySelector('.menu-btn'), tabs = document.querySelector('.tabs');
  if (menu && tabs) menu.addEventListener('click', function () {
    var open = tabs.classList.toggle('open');
    menu.setAttribute('aria-expanded', open);
  });

  // Copy buttons on code blocks marked data-copy
  document.querySelectorAll('pre[data-copy]').forEach(function (pre) {
    var code = pre.querySelector('code');
    if (!code) return;
    var b = document.createElement('button');
    b.className = 'copy'; b.type = 'button'; b.textContent = 'Copy';
    b.addEventListener('click', function () {
      var text = code.innerText.replace(/^\s*#.*$/gm, '').replace(/\n{2,}/g, '\n').trim();
      if (!navigator.clipboard || !navigator.clipboard.writeText) return;
      navigator.clipboard.writeText(text).then(function () {
        b.textContent = 'Copied';
        window.setTimeout(function () { b.textContent = 'Copy'; }, 1400);
      }).catch(function () { b.textContent = 'Copy unavailable'; });
    });
    pre.appendChild(b);
  });

  // Segmented buttons switching panels.
  document.querySelectorAll('.seg[data-panels]').forEach(function (seg) {
    var buttons = seg.querySelectorAll('button');
    function pick(id) {
      buttons.forEach(function (b) {
        var on = b.dataset.panel === id;
        b.setAttribute('aria-selected', on ? 'true' : 'false');
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
      p.addEventListener('click', function () {
        on(i);
        if (notes[i]) notes[i].scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      });
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
      var q = input ? (input.value || '').trim().toLowerCase() : '', shown = 0;
      grid.querySelectorAll('.ext').forEach(function (e) {
        var ok = (!cat || e.dataset.category === cat) && (!q || (e.dataset.text || '').indexOf(q) >= 0);
        e.hidden = !ok;
        if (ok) shown++;
      });
      if (empty) empty.hidden = shown > 0;
    }
    if (input) input.addEventListener('input', apply);
    chips.forEach(function (c) {
      c.addEventListener('click', function () {
        cat = c.getAttribute('aria-pressed') === 'true' ? '' : c.dataset.category;
        chips.forEach(function (x) {
          x.setAttribute('aria-pressed', x.dataset.category === cat && cat !== '' ? 'true' : 'false');
        });
        apply();
      });
    });
  }
})();
