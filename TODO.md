# 🗂️ Symbia – TODO / Roadmap

> Dernière mise à jour: Mai 2026  
> Statut: ✅ MVP jouable | 🚧 En cours | ⏳ À venir

---

## ✅ FAIT (MVP jouable)

### 🎨 Rendu
- [x] **Winding order CCW** → Faces correctement orientées, culling actif
- [x] **Cross-chunk updates** → Plus de seams visibles aux bordures
- [x] **Textures procédurales** → Atlas 64×64 généré en code (Grass, Dirt, Stone, etc.)
- [x] **Éclairage optimisé** → `illuminance: 8000` + lumière ambiante + matériau brillant
- [x] **Near plane ajusté** → `0.01` pour éviter le clipping proche

### 🎮 Gameplay
- [x] **Minage/Placement** → Clic G = casser, Clic D = poser (adjacent)
- [x] **Collision AABB** → Boîte 0.6×1.7m, résolution axe par axe (glissement)
- [x] **Gravité + Saut** → Chute réaliste, Espace = saut (uniquement au sol)
- [x] **Contrôles FPS** → WASD + souris + Échap toggle

### 🖱️ Input / UX
- [x] **Souris capturée** → `CursorGrabMode::Locked` + toggle Échap fiable
- [x] **Feedback console** → Logs pour minage/placement/capture souris

### 🌍 Monde infini
- [x] **Chunk streaming** → Chargement/déchargement autour du joueur (spawn:2, unload:4)
- [x] **Génération procédurale** → Seed-based, cohérent entre sessions

### 🔧 DevOps
- [x] **.gitignore Rust/Bevy** → `target/`, IDE, OS files ignorés
- [x] **Repo GitHub propre** → Push réussi, historique clean, 19 fichiers seulement
- [x] **Cargo.lock versionné** → Builds reproductibles

### 🎨 Atlas PNG externe (PRIORITÉ ACTUELLE)
- [x] **Charger un fichier `assets/textures/atlas.png`** au lieu de l'atlas procédural
- [x] **Fallback procédural** si le PNG est manquant (pour le dev)
- [x] **Documentation** : format attendu (64×64, 4×4 tuiles 16×16, RGBA)
---

## 🚧 EN COURS / PROCHAINES ÉTAPES

### 🎨 Atlas PNG externe (PRIORITÉ ACTUELLE)
- [ ] **Hot-reload** (optionnel) : recharger l'atlas si le fichier change

### 🌐 P2P Handshake (Cœur de Symbia)
- [ ] **libp2p integration** → Découverte de pairs, échange de seed
- [ ] **Sync de blocs** → Broadcast `BlockChange` aux voisins
- [ ] **Conflit resolution** → Timestamp ou autorité simple

### 🪂 Polish UX
- [ ] **Particules de feedback** → Effets visuels quand un bloc est cassé/posé
- [ ] **Crosshair minimal** → Petit point au centre de l'écran
- [ ] **HUD coordinates** → Afficher seed, FPS, coords du bloc visé (debug)

### 🧱 Optimisations
- [ ] **Greedy Meshing** → Fusionner faces adjacentes → -90% draw calls
- [ ] **Frustum Culling** → Ne pas render les chunks hors champ de vue
- [ ] **Chunk LOD** → Mesh simplifié pour chunks lointains

---

## ⏳ IDÉES FUTURES (Phase 2+)

- [ ] **Biomes** → Forêt, désert, montagne selon seed/position
- [ ] **Crafting system** → Combiner des blocs pour créer de nouveaux types
- [ ] **Jour/Nuit cycle** → Lumière dynamique + ciel changeant
- [ ] **Mobs / NPCs** → Entités simples avec IA de base
- [ ] **Sauvegarde IPFS** → Persistance décentralisée des chunks modifiés
- [ ] **DAO governance** → Token de vote pour les features du jeu

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

# Formater + Linter
cargo fmt --all
cargo clippy --workspace -- -D warnings

# Générer la doc
cargo doc --open --workspace