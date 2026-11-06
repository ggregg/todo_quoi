# ADR-0005 — Tauri 2 + Vue 3

- Statut : Acceptée · Date : 2026-11-06

## Contexte
Application de bureau Windows + macOS, puis Linux et peut-être iOS, avec un éditeur de notes riche
(cases, radios, tableaux, images, audio). Le cœur doit être en Rust ; le mainteneur est expert Vue/TS.
Options étudiées : Tauri, Qt 6 (cxx-qt), Slint, Dioxus, egui/iced.

## Décision
**Tauri 2** (cœur Rust, webview système) + **Vue 3 / TypeScript** + éditeur **TipTap**.

## Conséquences
- ➕ L'éditeur riche existe (TipTap) ; les blocs maison sont de simples extensions (POC I0-1).
- ➕ Audio et images traités par la webview sans dépendance (POC I0-2).
- ➕ Binaires légers, pas de Chromium embarqué ; iOS supporté par Tauri 2.
- ➖ Rendu non natif (webview) ; Qt aurait été natif mais l'éditeur 3 à 4 fois plus coûteux.
- ➖ Deux webviews différentes (WebView2, WKWebView) : chaque fonction média est vérifiée sur les deux.
