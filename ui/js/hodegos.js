/* Hodegos — petits utilitaires JavaScript sans dépendance (window.Hodegos). Les composants sont du HTML + bundle.css. */
window.Hodegos = (function () {
  'use strict';

  /* Thème : "light" ou "dark", posé sur <html data-theme>. */
  function setTheme(id) {
    document.documentElement.setAttribute('data-theme', id);
  }

  /* Onglets accessibles : boutons [role=tab] dans un [role=tablist], panneaux liés par aria-controls. */
  function initTabs(tablist) {
    var tabs = Array.prototype.slice.call(tablist.querySelectorAll('[role="tab"]'));
    function select(tab) {
      tabs.forEach(function (t) {
        var on = t === tab;
        t.setAttribute('aria-selected', on ? 'true' : 'false');
        t.tabIndex = on ? 0 : -1;
        var panelId = t.getAttribute('aria-controls');
        var panel = panelId ? document.getElementById(panelId) : null;
        if (panel) panel.hidden = !on;
      });
    }
    tabs.forEach(function (t, i) {
      t.addEventListener('click', function () { select(t); });
      t.addEventListener('keydown', function (e) {
        if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
        var next = tabs[(i + (e.key === 'ArrowRight' ? 1 : tabs.length - 1)) % tabs.length];
        select(next);
        next.focus();
      });
    });
    var current = tabs.filter(function (t) { return t.getAttribute('aria-selected') === 'true'; })[0] || tabs[0];
    if (current) select(current);
  }

  /* Choix exclusif (point de départ, réponse à une question, proposition d'un mentor) : boutons .hd-btn--choice avec aria-pressed. */
  function initChoices(group, onChange) {
    var buttons = Array.prototype.slice.call(group.querySelectorAll('.hd-btn--choice'));
    buttons.forEach(function (b) {
      b.addEventListener('click', function () {
        buttons.forEach(function (o) { o.setAttribute('aria-pressed', o === b ? 'true' : 'false'); });
        if (onChange) onChange(b.getAttribute('data-value') || b.textContent.trim());
      });
    });
  }

  /* Paliers d'aide : renvoie le palier suivant ("indice" → "question" → "piste" → "exemple"), ou null au dernier. */
  var LADDER = ['indice', 'question', 'piste', 'exemple'];
  function nextHint(level) {
    var i = LADDER.indexOf(level);
    return i >= 0 && i < LADDER.length - 1 ? LADDER[i + 1] : null;
  }

  /* Sélection d'un passage : renvoie le texte cité et l'identifiant stable le plus proche (data-anchor-id). */
  function getAnchor(root) {
    var sel = window.getSelection();
    if (!sel || sel.isCollapsed || !root.contains(sel.anchorNode)) return null;
    var node = sel.anchorNode.nodeType === 1 ? sel.anchorNode : sel.anchorNode.parentElement;
    var holder = node && node.closest('[data-anchor-id]');
    return { quote: sel.toString().trim(), anchorId: holder ? holder.getAttribute('data-anchor-id') : null };
  }

  return { version: '0.1.0', setTheme: setTheme, initTabs: initTabs, initChoices: initChoices, nextHint: nextHint, hintLadder: LADDER.slice(), getAnchor: getAnchor };
})();
