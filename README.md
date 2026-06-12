# Diet Optimizer

CPU-bound microservice that builds personalized daily meal plans from a recipe catalog. Given calorie and macro targets (plus optional dietary exclusions), it searches meal combinations with branch-and-bound and returns the plan with the lowest fitness cost.

## Requirements

- [Rust](https://rustup.rs/) **1.86+** (see `rust-version` in `Cargo.toml`)
- Optional: Docker for containerized deployment

## Quick start

```bash
# Build and run locally (default: http://0.0.0.0:8080)
cargo run

# Or run the release binary
cargo build --release
./target/release/diet_optimizer
```

Check that the service is up:

```bash
curl http://localhost:8080/health
```

Optimize a diet plan:

```bash
curl -s -X POST http://localhost:8080/api/v1/optimize-diet \
  -H 'Content-Type: application/json' \
  -d @examples/request-percentages.json | jq
```

See [examples/](examples/) for more sample requests and a full demo script.

## Configuration

| Variable   | Default     | Description                          |
|------------|-------------|--------------------------------------|
| `HOST`     | `0.0.0.0`   | Bind address                         |
| `PORT`     | `8080`      | Listen port                          |
| `RUST_LOG` | `info,...`  | Log filter (`debug` for more detail) |

Example:

```bash
RUST_LOG=debug PORT=3000 cargo run
```

## Docker

```bash
docker build -t diet-optimizer .
docker run --rm -p 8080:8080 diet-optimizer
```

The image exposes port **8080** and runs as a non-root user.

## API

### `GET /health`

Liveness probe.

**Response** `200 OK`:

```json
{
  "status": "ok",
  "service": "diet-optimizer"
}
```

### `POST /api/v1/optimize-diet`

Find the best combination of recipes for one day.

**Request body:**

| Field                 | Type     | Required | Default | Description                                      |
|-----------------------|----------|----------|---------|--------------------------------------------------|
| `target_calories`     | number   | yes      | —       | Daily calorie target (1–10000)                   |
| `macro_targets`       | object   | yes      | —       | Macro goals (grams or percentages)               |
| `dietary_exclusions`  | string[] | no       | `[]`    | Recipe tags to exclude (e.g. `dairy`, `vegan`) |
| `min_meals`           | integer  | no       | `3`     | Minimum meals in the plan (1–10)                   |
| `max_meals`           | integer  | no       | `5`     | Maximum meals in the plan (1–10)                 |

**Macro targets** — use one of:

```json
{
  "type": "percentages",
  "carbs_pct": 50.0,
  "protein_pct": 25.0,
  "fat_pct": 25.0
}
```

Percentages must sum to **100** (±1 tolerance). Converted using 4 kcal/g carbs & protein, 9 kcal/g fat.

```json
{
  "type": "grams",
  "carbs_g": 250.0,
  "protein_g": 150.0,
  "fat_g": 67.0
}
```

**Success response** `200 OK`:

```json
{
  "plan": {
    "meals": [
      {
        "recipe_id": "oatmeal-berry",
        "recipe_name": "Berry Oatmeal",
        "nutrition": {
          "calories": 380.0,
          "carbs_g": 58.0,
          "protein_g": 14.0,
          "fat_g": 9.0
        }
      }
    ],
    "total_nutrition": {
      "calories": 2000.0,
      "carbs_g": 250.0,
      "protein_g": 125.0,
      "fat_g": 55.6
    },
    "cost_score": 0.42
  }
}
```

- `meals` — selected recipes (count between `min_meals` and `max_meals`)
- `total_nutrition` — sum of meal macros
- `cost_score` — weighted squared relative error vs targets; **lower is better**

**Error responses:**

| Status | Code                   | When                                      |
|--------|------------------------|-------------------------------------------|
| `400`  | `VALIDATION_ERROR`     | Invalid JSON fields (e.g. negative calories) |
| `400`  | `INVALID_CONSTRAINTS`  | Bad macro sum, `min_meals > max_meals`, etc. |
| `422`  | `OPTIMIZATION_FAILED`  | No eligible recipes or no feasible plan   |

## Built-in recipe catalog

The service ships with an in-memory catalog of 10 recipes (breakfast, lunch, dinner, snacks). Tags include `vegan`, `vegetarian`, `dairy`, `gluten-free`, and meal-time labels. Use `dietary_exclusions` to filter by tag name (case-insensitive substring match on recipe tags).

## How optimization works

1. Resolve macro targets to grams.
2. Filter recipes that match none of the exclusion tags.
3. Search combinations of `min_meals..=max_meals` recipes (no duplicates, order-independent).
4. Score each plan with a weighted squared relative-error cost function.
5. Return the lowest-cost plan (branch-and-bound with Rayon parallelism).

## Development

```bash
# Unit + integration tests
cargo test

# Integration tests only
cargo test --test integration_test

# Lint
cargo clippy -- -D warnings
```

### Project layout

```
src/
  domain/          # Recipes, constraints, cost function, plan types
  application/     # DietOptimizationService (use case)
  optimization/    # Branch-and-bound optimizer
  infrastructure/  # HTTP (Axum) and in-memory recipe repository
examples/          # Sample JSON requests and curl demo
tests/             # HTTP integration tests
```

## Example requests

**Balanced macros (percentages), exclude dairy:**

```bash
curl -s -X POST http://localhost:8080/api/v1/optimize-diet \
  -H 'Content-Type: application/json' \
  -d @examples/request-percentages.json
```

**Explicit gram targets, high-protein day:**

```bash
curl -s -X POST http://localhost:8080/api/v1/optimize-diet \
  -H 'Content-Type: application/json' \
  -d @examples/request-grams.json
```

**Vegan-only (exclude non-vegan tags):**

```bash
curl -s -X POST http://localhost:8080/api/v1/optimize-diet \
  -H 'Content-Type: application/json' \
  -d @examples/request-vegan.json
```

Run all examples against a running server:

```bash
./examples/demo.sh
```

## License

MIT
