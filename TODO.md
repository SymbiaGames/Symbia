# 🗂️ Symbia – TODO / Roadmap
Dernière mise à jour: Mai 2026
Statut: ✅ **MVP Jouable & Multi-joueur LAN**

## ✅ FAIT (Implémenté et fonctionnel)

### 🌐 Réseau P2P (LAN)
- [x] **Intégration libp2p**: Découverte de pairs locale via mDNS
- [x] **Sync de blocs**: Broadcast `BlockUpdate` via GossipSub
- [x] **Résolution de conflits**: Priorité au timestamp le plus récent (Timestamp Wins)
- [x] **Thread-Safety**: Architecture `Arc<Mutex<P2PNode>>` compatible Bevy ECS

### 🎨 Rendu & Assets
- [x] **Atlas Externe**: Support texture 1024x1024 avec fallback procédural
- [x] **Pixel Art Net**: Filtrage `Nearest` (pas de flou bilinéaire)
- [x] **Gestion de Chunks**: Génération procédurale, streaming dynamique, bordures sans couture
- [x] **Optimisations Rendu**: Culling, Winding Order CCW

### 🎮 Gameplay & Interface
- [x] **Physique Complète**: Gravité, sauts, collisions AABB (axe par axe), friction
- [x] **Interactions Monde**: Casser/Poser des blocs (Clic Gauche/Droit)
- [x] **HUD**: Affichage FPS, Seed, Coordonnées joueur, Crosshair visé
- [x] **Feedback Visuel**: Système de particules lors de la destruction/pose de blocs

## 🚧 À VENIR (Prochaines étapes)

### 🌐 Réseau Avancé (WAN)
- [ ] **DHT Kademlia**: Découverte de pairs sur Internet (hors LAN) ⚠️ *À réintégrer proprement*
- [ ] **Chunk Streaming P2P**: Télécharger les chunks manquants depuis les pairs voisins
- [ ] **Sécurité**: Signature des messages (anti-triche timestamp)

### 🛠️ Améliorations & Polish
- [ ] **Greedy Meshing**: Réduire drastiquement le nombre de triangles (-90%)
- [ ] **Menu Principal**: Écran de titre et configuration
- [ ] **Inventaire**: Sélection de blocs (Touche 1-9)

## ⏳ IDÉES FUTURES (Phase 2+)
- [ ] Biomes variés (Désert, Neige...)
- [ ] Cycle Jour/Nuit
- [ ] Entités (Mobs/NPCs)
- [ ] Sauvegarde persistante (Local / IPFS)