# ADR 0001 — Interface web servie en local par un backend Rust

Statut : accepté · 29 octobre 2026

## Contexte

Hodegos est un IDE personnel, développé et utilisé sous WSL2 uniquement. Les
options étudiées : extension VS Code, application Tauri (WebKitGTK sous WSLg),
interface native rendue par GPU (Zed/GPUI, Vulkan sous WSLg), boîte à outils Rust
(egui, iced, Slint), interface web servie par un backend local.

## Décision

Un binaire Rust dans WSL2 sert une interface HTML/CSS/JS sans framework, ouverte
dans le navigateur de Windows, et dialogue avec elle par WebSocket sur `127.0.0.1`.

## Alternatives écartées

- Extension VS Code : l'ergonomie du mentorat est le produit ; une extension la
  contraint aux surfaces prévues par VS Code.
- Tauri : dépend de WSLg et des pilotes graphiques de WebKitGTK sous Linux.
- GPUI ou fork de Zed : Vulkan sous WSLg est instable, base de code très lourde.
- egui, iced, Slint : il faudrait écrire un éditeur de texte complet.

## Conséquences

- Le navigateur Windows rend l'interface avec son GPU, sans WSLg.
- Il faut un protocole WebSocket (docs/protocole.md) et une sécurité de serveur
  local (jeton, Origin, Host, 127.0.0.1).
- Le cœur ne connaît pas le navigateur : une coquille native pourra remplacer
  `hodegos-server` plus tard sans toucher aux autres crates.
