// Speed Keys: [ ] \ adjust playbackRate of the most relevant video.
var STEP = 0.25, MIN = 0.25, MAX = 4;
var label = null, hideTimer = 0;

function video() {
  var vs = Array.prototype.slice.call(document.querySelectorAll('video'));
  // Prefer a playing video, then the largest one on screen.
  vs.sort(function (a, b) {
    var pa = !a.paused, pb = !b.paused;
    if (pa !== pb) return pb - pa;
    return b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight;
  });
  return vs[0] || null;
}

function show(v) {
  if (!label) {
    label = document.createElement('div');
    label.className = 'oynix-speed-keys';
  }
  (document.fullscreenElement || document.documentElement).appendChild(label);
  var r = v.getBoundingClientRect();
  label.style.left = (r.left + 16) + 'px';
  label.style.top = (Math.max(r.top, 0) + 16) + 'px';
  label.textContent = v.playbackRate.toFixed(2).replace(/0$/, '') + '×';
  label.classList.add('shown');
  clearTimeout(hideTimer);
  hideTimer = setTimeout(function () { label.classList.remove('shown'); }, 900);
}

document.addEventListener('keydown', function (e) {
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  var t = e.target;
  if (t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) return;
  if (e.key !== '[' && e.key !== ']' && e.key !== '\\') return;
  var v = video();
  if (!v) return;
  if (e.key === '\\') v.playbackRate = 1;
  else v.playbackRate = Math.min(MAX, Math.max(MIN, Math.round((v.playbackRate + (e.key === ']' ? STEP : -STEP)) / STEP) * STEP));
  e.preventDefault();
  e.stopPropagation();
  show(v);
}, true);
