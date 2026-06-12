# EDTH Drone — Coordination distribuée d'intercepteurs

Système de défense aérienne temps réel : plusieurs plateformes d'interception partagent leurs détections, et un orchestrateur central fusionne les pistes, priorise les menaces et assigne les cibles de façon optimale — pour vaincre les attaques saturantes qu'une défense mono-intercepteur ne peut pas absorber.

## Le problème

Les menaces modernes arrivent de toutes les directions à la fois. Une attaque coordonnée (4 drones simultanés sur des vecteurs différents + essaims de leurres) dépasse la capacité d'un intercepteur seul, et une coordination manuelle prend 15–20 secondes par décision d'engagement — bien trop lent.

Le système doit :

- **Fusionner les capteurs distribués** (radar, optique) des 3 plateformes d'interception en une image de situation unifiée
- **Prioriser les menaces automatiquement** (vitesse, proximité, dangerosité)
- **Assigner chaque intercepteur à sa cible optimale** selon la portée, le temps de rechargement et la probabilité d'engagement
- **Suivre l'état du réseau** : munitions restantes et statut de chaque intercepteur
- **Recalculer les assignations toutes les 1–2 s** à mesure que les menaces se déplacent
- **Émettre des recommandations de tir** avec score de confiance pour chaque intercepteur

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Interceptor 1│     │ Interceptor 2│     │ Interceptor 3│
│ (capteurs +  │     │ (capteurs +  │     │ (capteurs +  │
│  effecteur)  │     │  effecteur)  │     │  effecteur)  │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │  rapports (position, menaces détectées, munitions)
       ▼                    ▼                    ▼
╔══════════════════════ NATS (broker pub/sub) ══════════════════════╗
╚════════════════════════════════╤═══════════════════════════════════╝
                                 ▼
                      ┌─────────────────────┐
                      │    Orchestrator     │
                      │ fusion des pistes   │
                      │ priorisation        │
                      │ assignation optimale│
                      └──────────┬──────────┘
                                 │  ordres (Intercept / MoveTo / Idle)
                                 ▼
                       retour aux intercepteurs
```

- Chaque **intercepteur** publie périodiquement un `InterceptorReport` (sa position, ses menaces détectées, ses munitions) sur NATS.
- L'**orchestrateur** s'abonne à ces rapports, maintient l'état global (`OrchestratorState`), et à chaque `tick` fusionne les détections puis recalcule les assignations.
- Les **ordres** (`InterceptorOrder::Intercept(threat_id)`, `MoveTo`, `Idle`) ne sont republiés que s'ils changent, pour minimiser le trafic.

## Structure du dépôt

```
edth_drone/
├── vanguard-core/                # lib : modèles partagés (Position, Threat,
│                                 #   PlatformInterceptor, Interceptor, états, rapports)
├── vanguard-map/                 # binaire : carte simulée — vérité terrain
│                                 #   (spawn de menaces, mouvement, détections radar)
├── vanguard-orchestrator/        # lib : OrchestratorState (fusion + assignation)
└── vanguard-system-interceptor/  # binaire : initialisation d'une plateforme (CLI clap)
```

Workspace Cargo (édition 2024). Une plateforme (`PlatformInterceptor`) porte un radar
(portée `range`) et un stock d'intercepteurs (`Interceptor`, les munitions, chacune avec
son UUID et son état `Idle / MovingTo / Intercepting / Destroyed`).

## Stack

- **Rust** (édition 2024)
- **clap** (CLI), **uuid** (identifiants v4), **rand** (aléatoire maîtrisé par seed)
- **NATS** ([`async-nats`](https://crates.io/crates/async-nats)) comme broker publish-subscribe, **serde / serde_json** pour les messages

## Environnement de test

Toutes les commandes pour partir de zéro :

```bash
# 1. Toolchain Rust (une seule fois par machine) — édition 2024 → rustc >= 1.85
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update

# 2. Récupérer et compiler le projet (les dépendances s'installent toutes seules)
git clone <url-du-repo> && cd edth_drone
cargo build

# 3. Vérifier que tout est sain
cargo test               # tests du workspace
cargo clippy --workspace # lint
cargo fmt --check        # formatage

# 4. Démarrer le broker NATS (terminal 1) — requis par la map et les plateformes
docker run -p 4222:4222 nats:latest

# 5. Lancer la carte (terminal 2) : vérité terrain, publie les menaces sur NATS
cargo run -p vanguard-map

# 6. Lancer des plateformes d'interception (un terminal chacune) :
#    chacune reste active, son radar s'abonne aux menaces et print ses détections
cargo run -p vanguard-system-interceptor -- --name alpha -n 4 -x -300 -y 250
cargo run -p vanguard-system-interceptor -- --name bravo -n 4 -x 300 -y 250
cargo run -p vanguard-system-interceptor -- --name charlie -n 4 -y -400
```

(`NATS_URL` est surchargeable par variable d'environnement, défaut `nats://127.0.0.1:4222`.)

## Utilisation actuelle

### Carte (vérité terrain)

Une menace apparaît toutes les 3 s en bordure de carte (5 000 m) et converge vers le
point défendu en (0, 0). La map **print uniquement les menaces** (spawn, position chaque
seconde, `LEAKER` à l'impact) et publie la liste sur le subject NATS `map.threats`
chaque seconde.

```bash
cargo run -p vanguard-map
```

```
map online — publishing threats on `map.threats` via nats://127.0.0.1:4222
[   0.0s] threat 351af624 spawned at (-4931, -830) — 57 m/s, level 4
[   1.0s] threat 351af624 at (-4875, -821)
[  ...s ] threat 351af624 reached defended point — LEAKER
```

La simulation est reproductible (seed fixe `SEED = 42` dans `vanguard-map/src/main.rs`,
comme les autres constantes : cadence de spawn, vitesses, portées).

### Plateforme d'interception

Process long-vivant : son radar s'abonne à `map.threats` et **c'est elle qui print ses
détections** — `RADAR CONTACT` à la première acquisition d'une menace, puis son image
radar à chaque mise à jour (`radar: <contacts>` ou `radar: no contact`).

```bash
cargo run -p vanguard-system-interceptor -- --name alpha -n 4 -x -300 -y 250
```

```
alpha (id 3472121a-…) online at (-300, 250) — radar range 1500 m, 4 interceptor(s) ready
alpha radar active — listening on `map.threats` via nats://127.0.0.1:4222
alpha radar: no contact
alpha RADAR CONTACT threat becf8dfa at (-172, -2167) — range 1492 m
alpha radar: becf8dfa at 1492 m
```

| Option | Rôle | Défaut |
|---|---|---|
| `--name` | nom de la plateforme | requis |
| `-n`, `--interceptors` | nombre d'intercepteurs (munitions) embarqués | 4 |
| `-x`, `-y` | position en mètres | 0.0 |

L'id de la plateforme et celui de chaque intercepteur sont des UUID v4 générés au
lancement.

## État d'avancement

- [x] Modèles de données (`vanguard-core` : menaces, plateformes, intercepteurs, états, rapports)
- [x] Carte simulée : spawn continu, mouvement vers le point défendu, signaux de détection radar, leakers
- [x] Initialisation d'une plateforme via CLI (UUID, munitions, position)
- [x] Boucle d'orchestration : fusion des rapports + assignation (heuristique : menace de plus haut niveau)
- [ ] Transport NATS entre plateformes et orchestrateur
- [ ] Rapports de détection (`InterceptorReport`) générés par la carte
- [ ] Assignation optimale (algorithme hongrois / max-flow, contraintes de portée et munitions)
- [ ] Scores de confiance sur les recommandations de tir
- [ ] Re-tasking dynamique en cours d'engagement
