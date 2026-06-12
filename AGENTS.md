# AGENTS.md — Instructions pour les agents IA

## Projet

Système de défense aérienne temps réel (hackathon EDTH) : N intercepteurs publient leurs détections sur un broker **NATS**, un **orchestrateur** central fusionne les pistes, priorise les menaces et assigne les cibles. Voir `README.md` pour le contexte complet.

## Architecture — règle d'or

**Intercepteur → NATS → Orchestrateur → NATS → Intercepteur.** Les intercepteurs ne se parlent jamais directement et ne décident pas de leurs cibles : toute la logique d'assignation vit dans l'orchestrateur (`src/orchestrator.rs`). Ne pas introduire de logique de décision côté intercepteur.

## Structure du workspace

| Crate | Rôle |
|---|---|
| `edth_2026` (racine, `src/`) | Binaire orchestrateur : état global, fusion, assignation |
| `interceptor/` | Binaire plateforme d'interception : capteurs simulés, exécution des ordres |
| `common/` | Types partagés sérialisables (serde) échangés sur NATS |

- Les types qui transitent sur NATS (`InterceptorReport`, `InterceptorOrder`, `DetectedThreat`, `Position`) doivent vivre dans `common` avec `#[derive(Serialize, Deserialize)]`, pas être dupliqués dans chaque binaire.
- ⚠️ État actuel : `common` n'a pas encore de `src/lib.rs` — les modèles sont encore dans `src/models.rs` à la racine. La migration vers `common` est un TODO connu.

## Conventions de code

- **Minimalisme avant tout** : toujours la solution la plus simple avec le moins de lignes de code possible. Pas d'abstraction spéculative (trait, generic, module) tant qu'il n'y a pas au moins deux usages concrets ; pas de gestion de cas qui n'arrivent pas encore ; pas de dépendance en plus si la lib standard suffit. Contexte hackathon : un code court qui marche bat un code extensible.
- Rust édition 2024, async avec **tokio** (`#[tokio::main]`, `rt-multi-thread`).
- Messagerie via **async-nats** + **serde_json**. Sujets NATS à nommer en hiérarchie : `interceptor.<id>.report`, `orchestrator.orders.<id>` (ou équivalent cohérent — vérifier l'existant avant d'en créer).
- L'orchestrateur fonctionne en **tick** (cible : recalcul toutes les 1–2 s) : `OrchestratorState::tick(reports) -> Vec<(id, InterceptorOrder)>`. Conserver cette séparation pure (état + calcul) / IO (NATS) : la logique d'assignation doit rester testable sans broker.
- N'émettre un ordre que s'il **change** (déjà le cas dans `assign()`) — ne pas spammer le bus.
- Pas de `unwrap()` sur les chemins IO/réseau ; réserver `unwrap`/`expect` aux invariants internes.

## Commandes

```bash
cargo build                 # builder tout le workspace
cargo run                   # orchestrateur
cargo run -p interceptor    # un intercepteur
cargo test                  # tests
cargo clippy --workspace    # lint — corriger les warnings introduits
cargo fmt                   # formatage avant commit
```

NATS local : `docker run -p 4222:4222 nats:latest` (port par défaut 4222).

## Pistes algorithmiques prévues

Si on te demande d'améliorer l'assignation, les méthodes visées par le sujet sont : algorithme hongrois / max-flow (éventuellement via OR-Tools), filtrage de Kalman pour la fusion de pistes, et prise en compte des contraintes réelles (portée `sight_reach`, munitions `ammo_remaining`, temps de rechargement, probabilité d'engagement). L'heuristique actuelle (tous sur la menace de plus haut niveau) est un placeholder assumé.

## Git

- Messages de commit courts, à l'impératif, en anglais (cohérent avec l'historique existant).
- Pas de ligne `Co-Authored-By` dans les commits.
