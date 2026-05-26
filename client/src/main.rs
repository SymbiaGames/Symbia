// ─────────────────────────────────────────────────────────────
// POINT D'ENTRÉE EXÉCUTABLE DU CLIENT SYMBIA
// Fichier : client/src/main.rs
// Rôle : Appeler run_client() depuis lib.rs pour démarrer le jeu
// ─────────────────────────────────────────────────────────────

// Importe la fonction publique "run_client" définie dans lib.rs du même crate
// "symbia_client" = nom du crate (défini dans client/Cargo.toml : name = "symbia-client")
// En Rust, les noms de crate utilisent des tirets (-), mais les imports utilisent des underscores (_)
use symbia_client::run_client;

// Fonction main classique, comme en C/C++
// Point d'entrée de l'exécutable compilé
fn main() {
    // Message de démarrage dans la console
    println!("🌿 Démarrage de Symbia Client v0.1.0");
    
    // Appelle la fonction d'initialisation du moteur Bevy
    // Cette fonction contient la boucle de jeu et ne retourne qu'à la fermeture
    run_client();
    
    // Ce code ne s'exécute que quand le joueur ferme la fenêtre
    println!("👋 Symbia Client fermé proprement");
}
