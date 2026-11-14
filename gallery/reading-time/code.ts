// Reading Time: estimate from the main article's paragraphs.
const WPM = 230;

interface Article { el: Element; words: number }

function articleRoot(): Article | null {
  let best: Element | null = null, bestWords = 0;
  document.querySelectorAll('article, main, [role=main], .post, .entry-content, #content').forEach(function (el) {
    var n = words(el);
    if (n > bestWords) { best = el; bestWords = n; }
  });
  return best ? { el: best, words: bestWords } : null;
}

function words(el: Element): number {
  let n = 0;
  el.querySelectorAll('p, li, blockquote').forEach(function (p) {
    n += ((p as HTMLElement).innerText || '').split(/\s+/).filter(Boolean).length;
  });
  return n;
}

const root = articleRoot();
if (root && root.words >= 400) {
  var total = Math.max(1, Math.round(root.words / WPM));
  var pill = document.createElement('div');
  pill.className = 'oynix-reading-time';
  pill.title = root.words.toLocaleString() + ' words · click to hide';
  pill.textContent = total + ' min read';
  pill.addEventListener('click', function () { pill.remove(); });
  document.documentElement.appendChild(pill);

  var update = function () {
    var r = root.el.getBoundingClientRect();
    var read = Math.min(1, Math.max(0, (innerHeight - r.top) / r.height));
    var left = Math.ceil(total * (1 - read));
    pill.textContent = read < 0.05 ? total + ' min read' : left <= 0 ? 'Finished' : left + ' min left';
    pill.style.setProperty('--progress', (read * 100).toFixed(1) + '%');
  };
  window.addEventListener('scroll', function () { requestAnimationFrame(update); }, { passive: true });
  update();
}
