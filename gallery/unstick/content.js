// Unstick: hide position:fixed / position:sticky bars at the top or bottom
// edge while scrolling down; show them again when scrolling up.
var CLASS = 'oynix-unstick-hidden';
var lastY = window.scrollY;
var bars = [];
var scanned = 0;

function isBar(el) {
  var cs = getComputedStyle(el);
  if (cs.position !== 'fixed' && cs.position !== 'sticky') return false;
  if (el.matches('dialog, [role=dialog], [aria-modal=true], video, iframe')) return false;
  var r = el.getBoundingClientRect();
  if (r.height < 24 || r.height > innerHeight * 0.5 || r.width < innerWidth * 0.5) return false;
  // Only bars glued to the top or bottom edge.
  return r.top <= 8 || r.bottom >= innerHeight - 8;
}

function scan() {
  scanned = Date.now();
  bars = [];
  var all = document.body ? document.body.querySelectorAll('*') : [];
  for (var i = 0; i < all.length && i < 6000; i++) {
    if (isBar(all[i]) || all[i].classList.contains(CLASS)) bars.push(all[i]);
  }
}

function onScroll() {
  var y = window.scrollY;
  var down = y > lastY + 4, up = y < lastY - 4;
  if (!down && !up) return;
  lastY = y;
  if (Date.now() - scanned > 1500) scan();
  for (var i = 0; i < bars.length; i++) {
    bars[i].classList.toggle(CLASS, down && y > 160);
  }
}

window.addEventListener('scroll', function () { requestAnimationFrame(onScroll); }, { passive: true });
