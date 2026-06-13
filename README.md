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

# 6. Lancer l'hôte des plateformes (un seul process) : démarre avec le preset
#    Kiev (6 plateformes périphérie 20 km + 3 défense de point ville 7 km).
#    On ajoute/retire/place ensuite des plateformes depuis le dashboard.
#    C'est aussi lui qui DÉCIDE des tirs (assignation hongroise) et les exécute.
#    CLASSIFICATION_RANGE_M (défaut 8000) = distance à laquelle on distingue
#    une vraie menace d'un leurre (donc à laquelle on autorise le tir).
cargo run -p vanguard-control

# 7. Le dashboard web (carte réelle centrée sur Kiev + panneau de contrôle)
cd webui && pnpm install && pnpm dev    # http://localhost:5173
```

Le panneau **SIMULATION CONTROL** du dashboard pilote la map en direct (**accélération
du temps** ×1–10, ratio de leurres, taille/cadence des vagues, rayon de zone, plafond),
permet d'**ajouter une
plateforme en cliquant sur la carte** (nom / portée / munitions), d'en retirer, et de
**réinitialiser** (`↺ RESET`) au scénario de base. Tout passe par NATS : l'UI publie sur
`control.map.config` / `control.platform.add` / `control.platform.remove` / `control.reset`.

### Boucle d'engagement (TEWA) + reconnaissance par l'intercepteur

Les **plateformes détectent** seulement (chaque contact est `Unknown` — elles ne
distinguent pas réel/leurre). `vanguard-control` assigne les contacts détectés par
l'**algorithme hongrois** (`pathfinding::kuhn_munkres`) — intercepteurs en vol + tubes
libres × contacts, avec **hystérésis** pour le re-tasking dynamique (jamais deux tirs sur
la même menace, jamais un bon tir gaspillé). Chaque tube lance un **vrai intercepteur**
qui vole vers un **point d'interception prédit (PIP)** (`vanguard_core::predicted_intercept`,
résolution quadratique sur la vitesse estimée).

C'est l'**autodirecteur de l'intercepteur qui reconnaît** réel vs leurre, en **phase
terminale** une fois à `RECOGNITION_RANGE` de la cible (défaut 4 km, env `RECOGNITION_RANGE_M`) :
menace réelle → impact (kill) ; **leurre → abort** (l'intercepteur vire vers la zone sûre et
le leurre est exclu des engagements suivants pour ne plus gaspiller de munition). La
reconnaissance est estampillée sur les rapports → le dashboard colore la piste
(ambre `UNKNOWN` → rouge `REAL` / gris `DECOY`) au moment où un intercepteur l'identifie.

Re-tasking **manuel** : cliquer un intercepteur puis une menace le redirige ; le bouton
**ABORT** l'envoie se crasher dans la **zone sûre** (`SAFE DROP ZONE`).

Chaque tir lance un **vrai intercepteur** qui vole vers un **point d'interception prédit
(PIP)** : `vanguard_core::predicted_intercept` résout l'équation d'interception
(quadratique) à partir de la vitesse estimée de la menace, l'intercepteur met le cap sur
ce point d'avance et impacte à la rencontre. Les positions des munitions en vol sont
publiées sur `control.interceptors` ; le dashboard les anime (**darts cyan + traînée**),
interpolées en fluide comme les menaces.

### Retour visuel & métriques

- **Burst d'impact** : un anneau qui s'étend à chaque kill (cyan) et à chaque impact réel
  au sol (rouge), placé à la position exacte.
- **Feed d'événements** (panneau latéral) : journal horodaté `NEUTRALIZED` / `⚠ IMPACT` /
  `decoy spent`, façon ticker C2.
- **Compteurs header** : `NEUTRALIZED` (menaces abattues) et **`IMPACTS`** (vrais drones qui
  ont touché Kyiv — la métrique de dégâts ; les leurres qui passent ne comptent pas).
- Le backend publie les kills sur `control.threat.destroyed` (avec position) et les fuites
  sur `control.leaker` (avec `is_decoy`, pour distinguer impact réel vs leurre inoffensif).

> Le binaire `vanguard-system-interceptor` (plateforme unique en CLI, option `--reach`)
> reste disponible si tu préfères lancer des plateformes en process séparés.

(`NATS_URL` est surchargeable par variable d'environnement, défaut `nats://127.0.0.1:4222`.)

## Utilisation actuelle

### Carte (vérité terrain)

Des **vagues d'essaims** (6-12 drones) arrivent toutes les ~45 s depuis un secteur
d'azimut aléatoire (anneau d'ingress à 50 km), mêlant vrais drones d'attaque et **leurres**
(~40 %). Chaque drone vise son **propre point d'impact aléatoire** dans une zone défendue
de 6 km de rayon autour du centre de Kiev (plus de point unique). Une plateforme ne peut
distinguer un vrai drone d'un leurre qu'une fois le contact entré dans sa **portée de
classification** (8 km par défaut, `CLASSIFICATION_RANGE_M`). La map publie la vérité
terrain (`Vec<Threat>`) sur `map.threats` chaque seconde ; les plateformes publient leurs
rapports radar (avec classification) sur `platform.<id>.report`.

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
