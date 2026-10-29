/* Hodegos — application. Phase 1 : vérifie la WebSocket par un ping. */
(function () {
  'use strict';

  var badge = document.getElementById('connexion');

  function show(text, variant) {
    badge.textContent = text;
    badge.className = 'hd-badge' + (variant ? ' hd-badge--' + variant : '');
  }

  var socket = new WebSocket('ws://' + location.host + '/ws');

  socket.addEventListener('open', function () {
    socket.send(JSON.stringify({ v: 1, type: 'ping', id: 'c-1' }));
  });

  socket.addEventListener('message', function (event) {
    var message;
    try {
      message = JSON.parse(event.data);
    } catch (e) {
      return;
    }
    if (message.type === 'pong') show('Connecté', 'success');
    else if (message.type === 'error') show('Erreur', 'danger');
  });

  socket.addEventListener('close', function () {
    show('Déconnecté', 'danger');
  });
})();
