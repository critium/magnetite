# Shop API

A small online shop: quote a cart, check out, look up and cancel orders, and see a
customer summary. This file is the **entry point**. It wires the routes to the
workflows in [Checkout](checkout.mgt.md) and adds a little plain Rust.

> [!NOTE]
> `mgtc build examples/shop` turns this folder into a Rust crate with an axum server.
> `mgtc test examples/shop` runs every `example` block. `mgtc doc` refreshes the
> diagrams and renders this folder as a documentation site.

## Files

| File | What's in it |
|---|---|
| [shop.mgt.md](shop.mgt.md) | Settings, routes, error responses, Rust extensions (this file) |
| [checkout.mgt.md](checkout.mgt.md) | The workflows behind each route, plus their pricing rules |
| [orders.mgt.md](orders.mgt.md) | `Order`, its states and transitions, and the actions that change it |
| [customers.mgt.md](customers.mgt.md) | Customer kinds and discounts |
| [catalog.mgt.md](catalog.mgt.md) | Products, warehouses, tax decision table, bulk discount |
| [integrations.mgt.md](integrations.mgt.md) | Every outside service: HTTP clients, and Rust ones for Postgres and the cache |
| [geo.mgt.md](geo.mgt.md) | Coordinates and a distance rule implemented in Rust |

## Uses

- `quote`, `checkout`, `order_details`, `cancel_order`, `customer_summary` from [Checkout](checkout.mgt.md)

## Settings

Loaded from environment variables at startup. A setting with no default is required,
and the server refuses to start without it, naming the missing variable.

| Setting | Env | Type | Default |
|---|---|---|---|
| `port` | `PORT` | `Int` | 8080 |
| `database_url` | `DATABASE_URL` | `Text` | |
| `crm_url` | `CRM_URL` | `Url` | |
| `catalog_url` | `CATALOG_URL` | `Url` | |
| `inventory_url` | `INVENTORY_URL` | `Url` | |
| `payments_url` | `PAYMENTS_URL` | `Url` | |
| `fastship_url` | `FASTSHIP_URL` | `Url` | https://api.fastship.example |
| `parcelco_url` | `PARCELCO_URL` | `Url` | https://api.parcelco.example |
| `notify_url` | `NOTIFY_URL` | `Url` | |
| `cache_size` | `ORDER_CACHE_SIZE` | `Int` | 10000 |

## Routes

Path parameters (`{id}`) bind to the workflow parameter with the same name, and the
JSON body binds to the remaining parameter. Every route also accepts `?explain=true`.

| Method | Path | Workflow | Success |
|---|---|---|---|
| `POST` | `/quotes` | [quote](checkout.mgt.md#quote) | `200` |
| `POST` | `/orders` | [checkout](checkout.mgt.md#checkout) | `201` |
| `GET` | `/orders/{id}` | [order_details](checkout.mgt.md#order-details) | `200` |
| `POST` | `/orders/{id}/cancel` | [cancel_order](checkout.mgt.md#cancel-an-order) | `200` |
| `GET` | `/customers/{id}/summary` | [customer_summary](checkout.mgt.md#customer-summary) | `200` |
| `GET` | `/health` | Rust: `health` (below) | `200` |

## Error responses

Every error a routed workflow can raise must appear here, or `mgtc` refuses to build.
An unmapped error can never turn into an accidental `500`.

| Error | Status | Message |
|---|---|---|
| `CustomerNotFound` | `404` | No customer with that ID |
| `OrderNotFound` | `404` | No order with that ID |
| `UnknownProduct` | `422` | The cart contains a product we don't sell |
| `OutOfStock` | `409` | Some items are out of stock |
| `Declined` | `402` | The card was declined |
| `NoWarehouseAvailable` | `503` | No warehouse can ship this order |
| `NoCarrierAvailable` | `503` | No carrier can deliver this order right now |
| `InvalidTransition` | `409` | *(the message from the failing `requires`)* |
| `Timeout` | `504` | A partner service took too long |

## Rust extensions

Plain Rust that the generated server uses. Rust blocks can see every generated type,
rule and service in the package.

### Dependencies

| Crate | Version | Features |
|---|---|---|
| `sqlx` | 0.8 | `postgres`, `runtime-tokio`, `json` |
| `moka` | 0.12 | `future` |
| `tower-http` | 0.6 | `trace`, `cors` |

### App state

The generated `App` already holds the settings and every HTTP client. `AppExtra` adds
what the Rust services in [Integrations](integrations.mgt.md) need.

```rust
use moka::future::Cache;
use sqlx::PgPool;

pub struct AppExtra {
    pub db: PgPool,
    pub cache: Cache<OrderId, Order>,
}

pub async fn init_extra(settings: &Settings) -> anyhow::Result<AppExtra> {
    Ok(AppExtra {
        db: PgPool::connect(&settings.database_url).await?,
        cache: Cache::new(settings.cache_size as u64),
    })
}
```

### Health check and middleware

```rust
use axum::{extract::State, http::StatusCode, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// `GET /health`, mounted by the Routes table above.
pub async fn health(State(app): State<App>) -> StatusCode {
    match sqlx::query("select 1").execute(&app.extra.db).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// Called once after the generated router is built.
pub fn customize(router: Router<App>) -> Router<App> {
    router
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
```

### Calling the generated code from ordinary Rust

The generated crate is a normal library, so other Rust code (a batch job, a CLI, an
existing service) can reuse the same rules and workflows without HTTP. This block is
`rust ignore`: it's shown for illustration and not compiled into the shop.

```rust ignore
use shop::{catalog::{tax_rate, Category}, geo::Region, App, Settings};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Pure rules are plain functions
    let rate = tax_rate(Region::Europe, Category::Books);      // 5%

    // Workflows need an App, which provides the services
    let app = App::from_settings(Settings::from_env()?).await?;
    let quote = shop::checkout::quote(&app, &request).await?;
    println!("{} via {:?}", quote.total(), quote.carrier());
    Ok(())
}
```
