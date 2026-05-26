# 🗂️ Symbia – TODO / Roadmap

> Dernière mise à jour: Mai 2026  
> Statut: ✅ MVP jouable | 🚧 En développement | ⏳ À venir

---

## 🔥 Haute priorité (à faire avant la démo)

### 🪂 Gameplay de base
- [ ] **Gravité simple** → Le joueur tombe s'il n'y a pas de bloc sous ses pieds
- [ ] **Saut** → Appuyer sur Espace pour sauter (avec cooldown)
- [ ] **Indicateur de sélection** → Affiche un contour/wireframe du bloc visé

### 🧱 Minage/Placement
- [ ] **Sélection de type de bloc** → Cycle entre Grass/Dirt/Stone avec la molette ou touches 1/2/3
- [ ] **Portée de minage/placement** → Configurable (actuellement 6 blocs)
- [ ] **Feedback visuel** → Particules ou son quand un bloc est cassé/posé

---

## ⚙️ Moyenne priorité (optimisations)

### 🚀 Performance
- [ ] **Greedy Meshing** → Fusionner les faces adjacentes dans un chunk → réduire draw calls de ~90%
- [ ] **Frustum Culling** → Ne pas envoyer au GPU les chunks hors champ de vue
- [ ] **LOD (Level of Detail)** → Chunks lointains = mesh simplifié

### 🌐 Réseau / P2P (Cœur de Symbia)
- [ ] **P2P Handshake** → Deux clients échangent la seed → même monde généré localement
- [ ] **Sync de blocs** → Quand un joueur mine/pose, envoyer `BlockChange` via libp2p aux voisins
- [ ] **Conflit resolution** → Si deux joueurs modifient le même bloc → timestamp ou autorité

### 🗺️ Monde infini
- [ ] **Cross-chunk updates** → Quand un bloc en bordure est modifié, régénérer aussi les chunks voisins
- [ ] **Sauvegarde IPFS** → Archiver les chunks modifiés dans IPFS/Arweave pour persistance décentralisée
- [ ] **Biomes** → Adapter la génération (forêt, désert, montagne) selon la seed/position

---

## 🧼 Low priority / Tech debt

### 🧹 Code qualité
- [ ] **Tests unitaires** → `cargo test` pour `ChunkData::index()`, `build_chunk_mesh()`, etc.
- [ ] **Clippy fixes** → `cargo clippy --fix` pour nettoyer les warnings Rust
- [ ] **Documentation** → `cargo doc --open` avec commentaires `///` sur les APIs publiques

### 🎨 Visuel
- [ ] **Textures basiques** → Remplacer les couleurs unies par des atlas de textures 16x16
- [ ] **Skybox / Fog** → Ajouter un ciel dégradé et du brouillard pour la profondeur
- [ ] **Ombres douces** → Activer `shadows_enabled: true` + configurer la lumière directionnelle

### 🎮 UX
- [ ] **Menu pause** → Échap ouvre un menu avec options (quitter, changer seed, etc.)
- [ ] **HUD minimal** → Afficher la seed, les FPS, les coordonnées du bloc visé
- [ ] **Contrôles configurables** → Permettre de rebinder WASD/mouse dans un config file

---

## 🧪 Idées futures (Phase 2+)

- [ ] **Crafting system** → Combiner des blocs pour créer de nouveaux types
- [ ] **Mobs / NPCs** → Entités simples avec IA de base (wander, follow player)
- [ ] **Jour/Nuit cycle** → Lumière dynamique qui change avec le temps
- [ ] **DAO governance** → Token de gouvernance pour voter sur les features du jeu
- [ ] **Marketplace NFT** → Échanger des "claims de terrain" ou des builds uniques

---

## 🛠️ Commands utiles

```bash
# Vérifier la compilation
cargo check --workspace

# Lancer le client
cargo run -p symbia-client

# Lancer avec une seed différente
$env:SYMBIA_SEED="777"; cargo run -p symbia-client  # PowerShell
SYMBIA_SEED=777 cargo run -p symbia-client          # Linux/macOS

# Tester les unit tests (quand ajoutés)
cargo test --workspace

# Formater le code
cargo fmt --all

# Linter avec recommandations
cargo clippy --workspace -- -D warnings

# Générer la doc
cargo doc --open --workspace