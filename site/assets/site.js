// OyNIx website: small progressive enhancements. Every page works without it.
(function () {
  'use strict';

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

  // Segmented buttons switching panels: <div class="seg" data-panels="name">
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
