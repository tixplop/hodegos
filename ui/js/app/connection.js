/* Connexion WebSocket au serveur : demandes numérotées (id / re), messages spontanés, reconnexion. */
const RECONNECT_MS = 2000;

/** Erreur renvoyée par le serveur (`error`) ou connexion perdue (`disconnected`). */
export class ServerError extends Error {
  constructor({ code, message, version = null }) {
    super(message);
    this.code = code;
    this.version = version;
  }
}

/**
 * Ouvre la connexion et la rouvre si elle tombe.
 * `onPush(message)` reçoit les messages qui ne répondent à aucune demande ;
 * `onState('open' | 'closed')` suit l'état de la connexion.
 */
export function connect({ onPush, onState }) {
  let socket = null;
  let counter = 0;
  const pending = new Map();

  function open() {
    socket = new WebSocket(`ws://${location.host}/ws`);
    socket.addEventListener('open', () => onState('open'));
    socket.addEventListener('message', (event) => {
      let message;
      try {
        message = JSON.parse(event.data);
      } catch {
        return;
      }
      const waiting = message.re ? pending.get(message.re) : undefined;
      if (!waiting) {
        onPush(message);
        return;
      }
      pending.delete(message.re);
      if (message.type === 'error') waiting.reject(new ServerError(message.payload));
      else waiting.resolve(message);
    });
    socket.addEventListener('close', () => {
      for (const waiting of pending.values()) {
        waiting.reject(new ServerError({ code: 'disconnected', message: 'Connexion perdue' }));
      }
      pending.clear();
      onState('closed');
      setTimeout(open, RECONNECT_MS);
    });
  }

  /** Envoie une demande ; la promesse se résout avec la réponse complète. */
  function request(type, payload) {
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      return Promise.reject(new ServerError({ code: 'disconnected', message: 'Pas de connexion au serveur' }));
    }
    counter += 1;
    const id = `c-${counter}`;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      socket.send(JSON.stringify({ v: 1, type, id, payload }));
    });
  }

  /** Envoie un message sans attendre de réponse ; ignoré si la connexion est fermée. */
  function send(type, payload) {
    if (socket && socket.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify({ v: 1, type, payload }));
    }
  }

  open();
  return { request, send };
}
