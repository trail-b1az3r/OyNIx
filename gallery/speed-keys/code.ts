// Speed Keys: [ ] \ adjust playbackRate of the most relevant video.
const STEP = 0.25, MIN = 0.25, MAX = 4;
let label: HTMLDivElement | null = null;
let hideTimer = 0;

function video(): HTMLVideoElement | null {
  const vs = Array.from(document.querySelectorAll('video'));
  // Prefer a playing video, then the largest one on screen.
  vs.sort(function (a, b) {
    const pa = !a.paused, pb = !b.paused;
    if (pa !== pb) return Number(pb) - Number(pa);
    return b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight;
  });
  return vs[0] || null;
}

function show(v: HTMLVideoElement): void {
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
  hideTimer = window.setTimeout(function () { label!.classList.remove('shown'); }, 900);
}

document.addEventListener('keydown', function (e) {
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  const t = e.target as HTMLElement | null;
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
