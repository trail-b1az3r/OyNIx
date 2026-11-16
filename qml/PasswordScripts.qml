pragma Singleton
import QtQuick

// Page scripts for saved passwords. They run in their own script world
// (`world`), which neither the website nor extensions can see; OyNIx
// decides which logins to offer from the tab's real address, never from
// anything the page says.
QtObject {
    readonly property int world: 7

    // Installed once per page: remembers what's typed into sign-in forms
    // (including two-step sign-ins, where the username comes first) and
    // marks when a form with a password is submitted.
    readonly property string capture: `(function () {
  if (window.__oynixPw) return true;
  var st = window.__oynixPw = { user: '', pass: '', submitted: false, needFill: true };
  function shown(el) { var r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0; }
  function isUserField(el) {
    var t = (el.type || 'text').toLowerCase();
    if (['text', 'email', 'tel', ''].indexOf(t) < 0) return false;
    var hint = ((el.autocomplete || '') + ' ' + (el.name || '') + ' ' + (el.id || '') + ' ' + (el.getAttribute('aria-label') || '')).toLowerCase();
    return t === 'email' || /user|email|login|account|identifier|phone/.test(hint);
  }
  function userFieldFor(pw) {
    var scope = pw.form || document;
    var inputs = Array.prototype.slice.call(scope.querySelectorAll('input'));
    for (var k = inputs.indexOf(pw) - 1; k >= 0; k--) {
      var t = (inputs[k].type || 'text').toLowerCase();
      if (['text', 'email', 'tel', ''].indexOf(t) >= 0 && shown(inputs[k])) return inputs[k];
    }
    return null;
  }
  function snapshot(pw) {
    if (!pw || !pw.value) return;
    // Sign-up and change-password forms: the last password field is the new one.
    var all = (pw.form || document).querySelectorAll('input[type=password]');
    var p = all.length > 1 ? all[all.length - 1] : pw;
    var u = userFieldFor(pw);
    if (u && u.value) st.user = u.value;
    st.pass = p.value;
  }
  function submitted(scope) {
    var pw = (scope && scope.querySelector) ? scope.querySelector('input[type=password]') : null;
    if (!pw) pw = document.querySelector('input[type=password]');
    if (pw && pw.value) { snapshot(pw); st.submitted = true; }
  }
  document.addEventListener('input', function (e) {
    var t = e.target;
    if (!t || t.tagName !== 'INPUT') return;
    if ((t.type || '').toLowerCase() === 'password') snapshot(t);
    else if (isUserField(t) && t.value) st.user = t.value;
  }, true);
  document.addEventListener('submit', function (e) { submitted(e.target); }, true);
  document.addEventListener('click', function (e) {
    var b = e.target && e.target.closest && e.target.closest('button, input[type=submit], input[type=button], [role=button]');
    if (b) submitted(b.form || b.closest('form'));
  }, true);
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Enter' && e.target && e.target.tagName === 'INPUT') submitted(e.target.form);
  }, true);
  // Sign-in forms that appear later (single-page apps, the second step of a sign-in).
  new MutationObserver(function () {
    if (document.querySelector('input[type=password]:not([data-oynix-seen])')) {
      document.querySelectorAll('input[type=password]').forEach(function (p) { p.setAttribute('data-oynix-seen', ''); });
      st.needFill = true;
    }
  }).observe(document.documentElement, { childList: true, subtree: true });
  return true;
})()`

    // Read (and reset) what capture saw.
    readonly property string poll: `(function () {
  var s = window.__oynixPw;
  if (!s) return null;
  var r = { user: s.user, pass: s.submitted ? s.pass : '', submitted: s.submitted, needFill: s.needFill };
  s.submitted = false; s.needFill = false;
  return r;
})()`

    // Fill the form with one of `logins` ([{id, username, password}]).
    // force: replace what's there (picked from the key menu). Returns the
    // id used, or "".
    function fill(logins, force) {
        return "(" + fillFn + ")(" + JSON.stringify(logins) + ", " + (force ? "true" : "false") + ")"
    }
    readonly property string fillFn: `function (logins, force) {
  function shown(el) { var r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0 && !el.disabled && !el.readOnly; }
  function set(el, v) {
    var d = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value');
    d.set.call(el, v);
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  }
  if (!logins.length) return '';
  var pws = Array.prototype.filter.call(document.querySelectorAll('input[type=password]'), shown);
  // Skip sign-up / change-password forms (more than one password field).
  if (pws.length > 1 && !force) return '';
  var pw = pws[0] || null;
  var scope = pw ? (pw.form || document) : document;
  var inputs = Array.prototype.slice.call(scope.querySelectorAll('input'));
  var user = null;
  if (pw) {
    for (var k = inputs.indexOf(pw) - 1; k >= 0; k--) {
      var t = (inputs[k].type || 'text').toLowerCase();
      if (['text', 'email', 'tel', ''].indexOf(t) >= 0 && shown(inputs[k])) { user = inputs[k]; break; }
    }
  } else {
    user = Array.prototype.filter.call(document.querySelectorAll('input[autocomplete~=username], input[type=email], input[autocomplete~=email]'), shown)[0] || null;
  }
  var pick = logins[0];
  if (user && user.value && !force) {
    for (var i = 0; i < logins.length; i++) if (logins[i].username === user.value) pick = logins[i];
  }
  if (user && pick.username && (force || !user.value)) set(user, pick.username);
  if (pw && (force || !pw.value)) set(pw, pick.password);
  return pw || user ? pick.id : '';
}`
}
