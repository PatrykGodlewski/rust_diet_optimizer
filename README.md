# Diet-Optimizer

High-performance, CPU-bound Rust microservice that generates personalized daily diet plans via combinatorial optimization (branch-and-bound search).

## Architecture

Hexagonal (ports & adapters):

- `src/domain/` — entities, cost function, optimizer port (no HTTP)
- `src/optimization/` — branch-and-bound implementation
- `src/application/` — use cases and repository port
- `src/infrastructure/` — Axum API and in-memory recipe catalog

## Prerequisites

On Debian/Ubuntu/WSL:

```bash
sudo apt install build-essential pkg-config
```

## Run locally

```bash
cargo run
```

Server listens on `0.0.0.0:8080` by default (`HOST`, `PORT` env vars).

## API

### `POST /api/v1/optimize-diet`

```json
{
  "target_calories": 2000.0,
  "macro_targets": {
    "type": "percentages",
    "carbs_pct": 50.0,
    "protein_pct": 25.0,
    "fat_pct": 25.0
  },
  "dietary_exclusions": ["dairy", "gluten"],
  "min_meals": 3,
  "max_meals": 5
}
```

Macro targets also accept `"type": "grams"` with `carbs_g`, `protein_g`, `fat_g`.

### `GET /health`

Liveness check.

## Docker

```bash
docker build -t diet-optimizer .
docker run -p 8080:8080 diet-optimizer
```

## Tests

```bash
cargo test
```
