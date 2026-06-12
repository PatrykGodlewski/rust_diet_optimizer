# Diet Optimizer — Codebase Overview

This document explains how the application is structured, how a request flows through the system, and the main design decisions. Use it to prepare for technical questions about the project.

## What the application does

The **Diet Optimizer** is a CPU-bound HTTP microservice written in Rust. A client sends daily nutrition goals (calories, macros, optional dietary exclusions, meal count bounds). The service:

1. Validates the request.
2. Filters recipes that violate dietary exclusions.
3. Searches combinations of meals that best match the targets.
4. Returns the lowest-cost plan as JSON.

The problem is a **combinatorial optimization** task: choose `k` recipes from a catalog (where `min_meals ≤ k ≤ max_meals`) so that summed nutrition is as close as possible to the user's targets.

---

## Architecture style: layered + ports & adapters

The code follows a **clean / hexagonal architecture** with four main layers. Dependencies point **inward**: outer layers depend on inner abstractions, not the other way around.

```
┌─────────────────────────────────────────────────────────────┐
│  main.rs          — composition root (wires everything)     │
├─────────────────────────────────────────────────────────────┤
│  infrastructure/  — HTTP (Axum), in-memory repository       │
├─────────────────────────────────────────────────────────────┤
│  application/     — use case orchestration (DietService)    │
├─────────────────────────────────────────────────────────────┤
│  domain/          — business types, rules, cost function    │
│  optimization/    — algorithm implementing DietOptimizer    │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Responsibility | Key files |
|-------|----------------|-----------|
| **Domain** | Core business concepts and rules | `domain/recipe.rs`, `constraints.rs`, `cost.rs`, `plan.rs` |
| **Application** | Orchestrates the use case; defines ports (traits) | `application/diet_service.rs`, `ports.rs` |
| **Optimization** | Pluggable search algorithm | `optimization/branch_and_bound.rs` |
| **Infrastructure** | HTTP API, persistence | `infrastructure/http/*`, `repository/in_memory.rs` |
| **Main** | Dependency injection at startup | `main.rs` |

### Why this structure?

- **Domain logic stays independent** of Axum, JSON, or storage details — easier to test and reason about.
- **Algorithms are swappable** via the `DietOptimizer` trait (e.g. branch-and-bound today, simulated annealing tomorrow).
- **Storage is swappable** via `RecipeRepository` (in-memory now, database later).

---

## Startup and dependency wiring

`main.rs` is the **composition root** — the only place that knows concrete implementations:

```rust
let repository = Arc::new(InMemoryRecipeRepository::with_defaults());
let optimizer  = Arc::new(BranchAndBoundOptimizer::default());
let service    = Arc::new(DietOptimizationService::new(repository, optimizer));
let app        = create_router(service);
```

Flow of ownership:

- `AppState` = `Arc<DietOptimizationService<BranchAndBoundOptimizer>>`
- The HTTP router holds this state and passes it to handlers.
- Handlers call `state.optimize_diet(constraints)`.

Environment variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `8080` | Listen port |
| `RUST_LOG` | `info,...` | Tracing filter |

---

## Request lifecycle

```mermaid
sequenceDiagram
    participant Client
    participant Axum as HTTP (Axum)
    participant Handler as optimize_diet handler
    participant DTO as OptimizeDietRequest
    participant Service as DietOptimizationService
    participant Repo as InMemoryRecipeRepository
    participant Opt as BranchAndBoundOptimizer

    Client->>Axum: POST /api/v1/optimize-diet (JSON)
    Axum->>Handler: deserialize + inject AppState
    Handler->>DTO: validate() via validator crate
    Handler->>Service: optimize_diet(UserConstraints)
    Service->>Service: validate_meal_bounds(), resolved_macros()
    Service->>Repo: filter_eligible(exclusions)
    Repo-->>Service: Vec<Recipe>
    Service->>Opt: optimize(recipes, constraints)
    Opt->>Opt: search combinations, score with CostFunction
    Opt-->>Service: DietPlan
    Service-->>Handler: DietPlan
    Handler-->>Client: 200 JSON { plan: ... }
```

### Step-by-step

1. **HTTP layer** (`infrastructure/http/handlers.rs`)
   - Deserializes JSON into `OptimizeDietRequest`.
   - Runs field validation (`validator` crate: calorie range, meal bounds).
   - Checks `min_meals ≤ max_meals`.
   - Converts DTO → `UserConstraints` (domain type).

2. **Application layer** (`application/diet_service.rs`)
   - Validates meal bounds and resolves macro targets again at domain level.
   - Asks repository for eligible recipes.
   - Delegates optimization to `DietOptimizer` implementation.

3. **Optimization layer** (`optimization/branch_and_bound.rs`)
   - Enumerates all combinations of size `min_meals..=max_meals`.
   - Scores each combination with `CostFunction`.
   - Returns the plan with the lowest score.

4. **Response**
   - `DietPlan` is serialized to JSON via `OptimizeDietResponse`.

### Error mapping

Domain and validation errors become HTTP responses in `error_response.rs`:

| Condition | HTTP status | Code |
|-----------|-------------|------|
| Invalid JSON fields | 400 | `VALIDATION_ERROR` |
| Bad macro sum, meal bounds | 400 | `INVALID_CONSTRAINTS` |
| No recipes after filtering / no feasible plan | 422 | `OPTIMIZATION_FAILED` |

---

## Domain model

### Recipe & nutrition

```text
Recipe
├── id, name
├── nutrition: NutritionalProfile (calories, carbs_g, protein_g, fat_g)
└── tags: Vec<String>   // e.g. "vegan", "dairy", "lunch"
```

`Recipe::has_excluded_tag()` checks if any recipe tag matches an exclusion (case-insensitive). If a user excludes `"dairy"`, any recipe tagged `dairy` is removed.

### User constraints

```text
UserConstraints
├── target_calories: f64
├── macro_targets: MacroTargets (enum)
├── dietary_exclusions: Vec<String>
├── min_meals, max_meals: usize
```

**MacroTargets** supports two input formats:

| Variant | Input | Resolution |
|---------|-------|------------|
| `Grams` | Direct gram values | Used as-is (must be ≥ 0) |
| `Percentages` | carbs/protein/fat % of calories | Converted to grams: carbs & protein ÷ 4 kcal/g, fat ÷ 9 kcal/g |

Percentages must sum to **100** (±1 tolerance). This logic lives in `domain/constraints.rs`.

### Diet plan (output)

```text
DietPlan
├── meals: Vec<PlannedMeal>      // selected recipes
├── total_nutrition: NutritionalProfile   // sum of meals
└── cost_score: f64              // lower = better fit
```

`DietPlan::from_recipes()` builds the plan and aggregates nutrition.

---

## Cost function (fitness scoring)

Defined in `domain/cost.rs`. For each nutrient, compute **relative error**:

```text
relative_error(actual, target) = (actual - target) / target   (if target ≠ 0)
```

Total cost is a **weighted sum of squared relative errors**:

```text
cost = w_cal · err_cal² + w_carbs · err_carbs² + w_protein · err_protein² + w_fat · err_fat²
```

Default weights:

| Nutrient | Weight |
|----------|--------|
| Calories | 1.0 |
| Carbs    | 2.0 |
| Protein  | 2.0 |
| Fat      | 2.0 |

**Why squared errors?** Penalizes large deviations more than small ones. **Why relative?** A 50 kcal miss on a 2000 kcal target is treated proportionally, not the same as a 50 g carb miss.

**Why higher macro weights?** Macros are often stricter goals than total calories in diet planning.

A perfect match yields `cost_score = 0.0`.

---

## Optimization algorithm

Implementation: `BranchAndBoundOptimizer` in `optimization/branch_and_bound.rs`.

### Problem formulation

- **Input**: `n` eligible recipes, meal count `k` where `min_meals ≤ k ≤ max_meals`.
- **Decision**: choose `k` distinct recipes (combinations, not permutations — order does not matter).
- **Objective**: minimize `CostFunction` score of summed nutrition.

### Search strategy

For each meal count `k` in the allowed range:

1. Generate combinations using index vectors `[0, 1, 2, …]` advanced by `next_combination()` (standard lexicographic combination iterator).
2. Sum nutrition for the current combination.
3. Compute cost; track global best.

**Parallelism**: Rayon runs different meal counts (`k = 3`, `k = 4`, …) in parallel via `into_par_iter()`. A shared `AtomicU64` stores the best cost seen so far (using `f64::to_bits()` for atomic compare).

### Branch-and-bound note

The struct is named *branch-and-bound*, but the current implementation performs **full enumeration** over all combinations for each meal count, with parallel partitioning by `k`. The `partial_lower_bound()` method on `CostFunction` exists for future pruning but is not yet used to skip branches early. For the default catalog (~10 recipes, k ≤ 5), exhaustive search is fast enough.

**Complexity**: For meal count `k` and `n` recipes, there are `C(n, k)` combinations. Searching all k in range is roughly `Σ C(n,k)` — exponential in `n`, which is acceptable for small catalogs but would need smarter pruning or heuristics at scale.

Unit tests verify the optimizer matches **brute-force** optimum on small inputs.

---

## Ports (traits) — extension points

### `RecipeRepository` (`application/ports.rs`)

```rust
fn all_recipes(&self) -> &[Recipe];
fn filter_eligible(&self, exclusions: &[String]) -> Result<Vec<Recipe>, DomainError>;
```

Current impl: `InMemoryRecipeRepository` with 10 hard-coded recipes in `in_memory.rs`.

### `DietOptimizer` (`domain/optimizer.rs`)

```rust
fn optimize(&self, recipes: &[Recipe], constraints: &UserConstraints) -> Result<DietPlan, DomainError>;
```

Current impl: `BranchAndBoundOptimizer`.

These traits are the **boundaries** you would mention when asked “how would you extend this?” — swap repository for PostgreSQL, swap optimizer for a metaheuristic, etc.

---

## HTTP API layer

| Route | Method | Handler |
|-------|--------|---------|
| `/health` | GET | Liveness probe |
| `/api/v1/optimize-diet` | POST | Main optimization endpoint |

Files:

| File | Role |
|------|------|
| `router.rs` | Route definitions + HTTP tracing middleware |
| `dto.rs` | Request/response structs, Serde + validation |
| `handlers.rs` | Async handler functions |
| `error_response.rs` | Maps errors → HTTP status + JSON body |
| `state.rs` | Type alias for shared app state |

The HTTP layer knows about domain types for responses (`DietPlan`) but converts requests through DTOs to keep validation annotations separate.

---

## Built-in recipe catalog

`InMemoryRecipeRepository::with_defaults()` loads 10 recipes (oatmeal, chicken, salmon, etc.) with tags for dietary filtering and meal type. There is **no persistence** — the catalog is fixed at startup.

This is a deliberate simplification for a microservice demo. Production would externalize recipes to a database or external API.

---

## Testing strategy

| Level | Location | What it covers |
|-------|----------|----------------|
| **Unit** | `domain/cost.rs`, `constraints.rs`, `diet_service.rs`, `branch_and_bound.rs` | Cost math, macro resolution, exclusion filtering, optimizer correctness vs brute force |
| **Integration** | `tests/integration_test.rs` | Full HTTP stack: health, happy path, validation errors, exclusions, 422 cases |

Integration tests build the Axum router in-process (no real network) using `tower::ServiceExt::oneshot`.

Example files in `examples/` provide JSON payloads for manual/curl testing.

---

## Technology choices (likely professor questions)

| Choice | Reason |
|--------|--------|
| **Rust** | Memory safety, performance for CPU-bound search, strong typing for domain model |
| **Axum** | Modern async HTTP framework, good Tower ecosystem |
| **Rayon** | Data parallelism for searching different meal counts |
| **Serde** | JSON serialization for API |
| **validator** | Declarative request validation |
| **thiserror** | Structured domain error types |
| **Arc + traits** | Shared ownership of services; swappable implementations |

---

## Common questions & short answers

**Q: What problem are you solving?**  
A: Daily meal plan selection — pick a subset of recipes whose combined nutrition best matches user calorie and macro targets, respecting dietary exclusions and meal count limits.

**Q: Is this linear programming?**  
A: No. Recipes are discrete choices (you pick whole meals, not fractional portions). It is combinatorial optimization over subsets.

**Q: Why not always pick the lowest-calorie meals?**  
A: The cost function balances calories *and* all three macros. A low-calorie plan can still score poorly if protein or carbs are far from target.

**Q: How do dietary exclusions work?**  
A: Tag-based filtering. Each recipe has tags; if any tag matches an exclusion string (case-insensitive), the recipe is removed before optimization.

**Q: What happens if no plan exists?**  
A: `NoEligibleRecipes` if filtering removes everything; `NoFeasiblePlan` if the search finds no combination (rare with defaults). Both map to HTTP 422.

**Q: How would you scale this?**  
A: Larger catalogs need better algorithms (proper branch-and-bound pruning, integer programming, or heuristics), caching, possibly pre-filtering by calorie range, and moving recipe storage to a database. Could also offload heavy optimization to a worker queue.

**Q: Why separate domain and infrastructure?**  
A: Keeps business rules testable without HTTP; allows changing transport or storage without touching optimization logic.

**Q: What is `cost_score` in the response?**  
A: The weighted squared relative error — the optimizer's objective value. Lower means closer to targets. It is not monetary cost.

---

## File map (quick reference)

```text
src/
├── main.rs                          # Entry point, tracing, server bind
├── lib.rs                           # Module exports
├── domain/
│   ├── recipe.rs                    # Recipe, NutritionalProfile
│   ├── constraints.rs               # UserConstraints, MacroTargets
│   ├── cost.rs                      # CostFunction
│   ├── plan.rs                      # DietPlan, PlannedMeal
│   ├── optimizer.rs                 # DietOptimizer trait
│   └── error.rs                     # DomainError enum
├── application/
│   ├── diet_service.rs              # optimize_diet use case
│   └── ports.rs                     # RecipeRepository trait
├── optimization/
│   └── branch_and_bound.rs          # Combinatorial search + Rayon
└── infrastructure/
    ├── http/                        # Axum router, handlers, DTOs
    └── repository/in_memory.rs      # Default recipe catalog

tests/integration_test.rs            # End-to-end HTTP tests
examples/                            # Sample JSON requests
```

---

## Rust & project configuration

This section explains how Rust works in general, how this project is configured, and which Rust features the code uses.

### Rust in one minute

Rust is a **compiled**, **statically typed** language focused on performance and memory safety without a garbage collector.

| Concept | Meaning |
|---------|---------|
| **Compile time** | `cargo build` turns `.rs` source into a native binary. Many bugs (type errors, data races) are caught before run. |
| **Ownership** | Each value has one owner. References (`&T`) borrow without taking ownership. Prevents use-after-free and double-free. |
| **Borrow checker** | Compiler rule that enforces safe memory access at compile time. |
| **Traits** | Like interfaces — shared behavior (`DietOptimizer`, `RecipeRepository`). |
| **Enums** | Sum types — `Result<T, E>` for success/error, `MacroTargets` for variants. |
| **No null** | `Option<T>` = `Some(value)` or `None` instead of null pointers. |
| **Error handling** | Functions return `Result<T, E>`; callers use `?` to propagate errors. |

Rust fits this project because the optimizer is **CPU-bound** and benefits from native speed, and the type system keeps domain models explicit (`UserConstraints`, `DietPlan`, etc.).

### Project layout (Cargo convention)

```text
rust_diet_optimizer/
├── Cargo.toml          # Project manifest (name, deps, targets)
├── Cargo.lock          # Exact dependency versions (committed for apps)
├── src/
│   ├── main.rs         # Binary entry point (HTTP server)
│   └── lib.rs          # Library root (business logic — importable & testable)
├── tests/              # Integration tests (each .rs file = separate crate)
├── examples/           # Sample JSON + demo script (not Rust examples/)
├── target/             # Build output (gitignored) — debug & release binaries
├── Dockerfile          # Multi-stage container build
├── README.md           # Run instructions & API
└── ARCHITECTURE.md     # This file
```

**Cargo** is Rust's build tool and package manager (like npm + webpack, or Maven). You run `cargo build`, `cargo run`, `cargo test`.

### `Cargo.toml` explained

```toml
[package]
name = "diet_optimizer"       # Crate name; binary becomes target/debug/diet_optimizer
version = "0.1.0"
edition = "2021"              # Rust language edition (syntax & std library baseline)
rust-version = "1.86"         # Minimum rustc version required
description = "..."
license = "MIT"
```

| Field | Purpose |
|-------|---------|
| `edition` | Which Rust language version rules apply (2018, 2021, …). |
| `rust-version` | Documents minimum compiler; `cargo` may warn if rustc is too old. Matches Docker image `rust:1.86`. |

**Dependencies** — crates from [crates.io](https://crates.io):

| Crate | Role in this project |
|-------|----------------------|
| `axum` | Async HTTP server framework |
| `tokio` | Async runtime (runs `.await` futures, TCP listener) |
| `tower` / `tower-http` | Middleware (HTTP tracing) |
| `serde` / `serde_json` | JSON serialize/deserialize |
| `validator` | Validate request fields (`#[validate(range(...))]`) |
| `thiserror` | Derive `Display` + `Error` for `DomainError` |
| `anyhow` | Convenient error type in `main()` (`anyhow::Result<()>`) |
| `rayon` | Parallel iterators for optimizer |
| `tracing` / `tracing-subscriber` | Structured logging |

**Dev-dependencies** — only for tests, not shipped in the binary:

- `http-body-util`, `tower` — read HTTP response bodies in integration tests.

**Two targets in one repo:**

```toml
[lib]
name = "diet_optimizer"
path = "src/lib.rs"           # Library crate — most code lives here

[[bin]]
name = "diet_optimizer"
path = "src/main.rs"          # Executable — thin wrapper that starts the server
```

Why both?

- **`lib.rs`** holds domain, application, optimization, infrastructure — testable without starting HTTP.
- **`main.rs`** only wires dependencies and runs `axum::serve`.
- Integration tests import `diet_optimizer::...` as a library.

**Release profile** (production optimizations):

```toml
[profile.release]
lto = true           # Link-time optimization — smaller, faster binary
codegen-units = 1    # Single codegen unit — better optimization, slower compile
strip = true         # Strip debug symbols — smaller binary
```

Debug builds (`cargo run`, `cargo test`) skip these for faster compile. Docker uses `cargo build --release`.

### `Cargo.lock`

Locks exact versions of every transitive dependency. For **applications** (like this service), commit `Cargo.lock` so builds are reproducible. Libraries often omit it; apps should keep it.

### How code is organized: modules

Rust modules map to files/folders via `mod` declarations:

```rust
// src/lib.rs
pub mod application;   // → src/application/mod.rs
pub mod domain;        // → src/domain/mod.rs
```

`pub` = visible outside the crate. `use` imports names. Private items stay module-internal unless exported.

This project uses **folder modules**: `domain/mod.rs` re-exports submodules (`recipe`, `cost`, …).

### Rust patterns used in this codebase

#### 1. `Result` and the `?` operator

```rust
pub fn optimize_diet(&self, constraints: UserConstraints) -> Result<DietPlan, DomainError> {
    constraints.validate_meal_bounds()?;   // if Err, return early
    let eligible = self.repository.filter_eligible(&constraints.dietary_exclusions)?;
    self.optimizer.optimize(&eligible, &constraints)
}
```

`?` propagates errors up the call stack — no exceptions; errors are explicit in types.

#### 2. Traits (ports)

```rust
pub trait DietOptimizer: Send + Sync {
    fn optimize(&self, recipes: &[Recipe], constraints: &UserConstraints)
        -> Result<DietPlan, DomainError>;
}
```

`Send + Sync` means safe to share across threads (`Arc` + Rayon parallelism).

#### 3. `Arc` — shared ownership

```rust
let repository = Arc::new(InMemoryRecipeRepository::with_defaults());
let service = Arc::new(DietOptimizationService::new(repository, optimizer));
```

HTTP handlers clone cheap `Arc` pointers instead of copying large structs. Multiple request tasks share one service instance.

#### 4. Async / Tokio

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
}
```

- **`async fn`** — function that returns a **Future** (lazy task).
- **`.await`** — yield until I/O completes (non-blocking).
- **`#[tokio::main]`** — macro that starts the Tokio runtime and runs `main`.

HTTP I/O is async; the optimizer itself is **sync CPU work** run inside async handlers (fine for moderate load; heavy scale might use `spawn_blocking`).

#### 5. Derive macros

Common in this repo:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe { ... }

#[derive(Debug, Error, PartialEq)]
pub enum DomainError { ... }

#[derive(Debug, Deserialize, Validate)]
pub struct OptimizeDietRequest { ... }
```

Macros generate boilerplate at compile time (JSON traits, validation, error messages).

#### 6. Generics

```rust
pub struct DietOptimizationService<O: DietOptimizer> { ... }
```

The service works with any optimizer implementing the trait; `main.rs` picks `BranchAndBoundOptimizer`.

#### 7. Unit tests co-located with code

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolves_percentages_to_grams() { ... }
}
```

`#[cfg(test)]` — compiled only when running `cargo test`.

Integration tests in `tests/` are separate crates that link against your library.

### Build & run commands

| Command | What it does |
|---------|----------------|
| `cargo run` | Build debug + run `main.rs` binary |
| `cargo build --release` | Optimized binary → `target/release/diet_optimizer` |
| `cargo test` | Run unit + integration tests |
| `cargo test --test integration_test` | Integration tests only |
| `cargo check` | Type-check without full codegen (fast feedback) |
| `cargo clippy` | Lint for idiomatic Rust |
| `RUST_LOG=debug cargo run` | Run with verbose logging |

### Docker build (how it uses Cargo)

```dockerfile
FROM rust:1.86-bookworm AS builder    # Full toolchain
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release             # Statically linked release binary

FROM debian:bookworm-slim AS runtime  # Small runtime — no Rust compiler
COPY --from=builder .../diet_optimizer /usr/local/bin/
```

Multi-stage: builder image is large; final image only contains the compiled binary + minimal OS libs.

### What gets compiled

```text
cargo build
    │
    ├── libdiet_optimizer.rlib    ← library from src/lib.rs
    └── diet_optimizer (binary)   ← links lib + main.rs

cargo test
    ├── unit tests inside src/**/*.rs
    └── integration_test (binary) ← tests/integration_test.rs
```

### Professor-style Rust questions

**Q: Why Rust instead of Python/Node for this?**  
A: Combinatorial search is CPU-heavy; Rust gives native speed and safe concurrency (Rayon) without GIL limits.

**Q: What is ownership in your project?**  
A: e.g. `UserConstraints` is moved into `optimize_diet`; recipes are borrowed as `&[Recipe]` during search to avoid copying the whole catalog repeatedly.

**Q: How do you handle errors?**  
A: Domain layer uses `DomainError` + `Result`. HTTP layer maps to status codes. `main` uses `anyhow` for startup errors with context strings.

**Q: Is the server multithreaded?**  
A: Yes. Tokio runs a multi-thread runtime (`rt-multi-thread` feature). Rayon parallelizes optimizer work across meal counts.

**Q: What is `edition = "2021"`?**  
A: Sets language defaults (e.g. disjoint capture in closures, into_iter for arrays). Not the same as `rust-version`.

---

## Related docs

- [README.md](README.md) — how to run the service, API reference, curl examples
