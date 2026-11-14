// Clean Copy: copy location.href minus known tracking parameters.
const TRACKING: RegExp = /^(utm_[a-z_]+|fbclid|gclid|gclsrc|dclid|gbraid|wbraid|msclkid|yclid|twclid|ttclid|li_fat_id|mc_cid|mc_eid|_hsenc|_hsmi|mkt_tok|igshid|igsh|si|ref_src|ref_url|s_cid|vero_id|oly_anon_id|oly_enc_id|__s|ncid|spm|share_id)$/i;

function clean(href: string): string {
  const u = new URL(href);
  Array.from(u.searchParams.keys()).forEach(function (k) { if (TRACKING.test(k)) u.searchParams.delete(k); });
  if (/^#?(:~:|xtor=)/.test(u.hash)) u.hash = '';
  return u.toString().replace(/\?$/, '');
}

function toast(text: string): void {
  const t = document.createElement('div');
  t.className = 'oynix-clean-copy';
  t.textContent = text;
  document.documentElement.appendChild(t);
  setTimeout(function () { t.classList.add('gone'); }, 1800);
  setTimeout(function () { t.remove(); }, 2200);
}

document.addEventListener('keydown', function (e) {
  if (!(e.ctrlKey && e.shiftKey && (e.key === 'C' || e.key === 'c'))) return;
  e.preventDefault();
  e.stopPropagation();
  var url = clean(location.href);
  navigator.clipboard.writeText(url).then(function () {
    var removed = location.href.length - url.length;
    toast((removed > 0 ? 'Copied without tracking · ' : 'Copied · ') + url);
  }, function () { toast('Couldn’t copy: the page has no clipboard access'); });
}, true);
