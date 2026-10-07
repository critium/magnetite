# Integrations

Every outside system the shop talks to. Workflows in [Checkout](checkout.mgt.md)
call these; rules never can.

The calls are declared in one of two ways:

- **HTTP services** are declared as a table. `mgtc` generates a typed HTTP client
  (reqwest + serde), including timeouts, retries, and status-code → error mapping.
- **Rust services** are declared as a `Call | Returns | Fails with` table and implemented in a `rust` block
  right below it, for anything a table can't describe, such as a database or a cache.

## Uses

- `Customer`, `CustomerId` from [Customers](customers.mgt.md)
- `Product`, `ProductId`, `Warehouse`, `WarehouseId` from [Catalog](catalog.mgt.md)
- `GeoPoint` from [Geography](geo.mgt.md)
- `Order`, `OrderId`, `LineItem`, `PaymentId`, `ShipmentId`, `Carrier` from [Orders](orders.mgt.md)

> [!NOTE]
> Imports must not form a cycle. `Carrier` is declared in Orders and imported here,
> because Orders can't import from Integrations.

## Shared types

### `ShippingRate`

| Field | Type | Constraint |
|---|---|---|
| `carrier` | `Carrier` | |
| `price` | `Money` | at least $0 |
| `days` | `Int` | between 1 and 30 |

### `Reservation`

| Field | Type | Constraint |
|---|---|---|
| `id` | `Text` | not empty |
| `warehouse` | `WarehouseId` | |
| `expires_at` | `DateTime` | |

### `Tracking`

| Field | Type | Constraint |
|---|---|---|
| `shipment` | `ShipmentId` | |
| `status` | `Text` | |
| `eta` | `Date` | optional |

## HTTP services

### service `CustomerDirectory`

**Base URL:** setting `crm_url`
**Timeout:** 2 seconds
**Retries:** 2

| Call | HTTP | Returns | Fails with |
|---|---|---|---|
| `load(id: CustomerId)` | `GET /customers/{id}` | `Customer` | `CustomerNotFound` on 404 |

### service `ProductCatalog`

**Base URL:** setting `catalog_url`
**Timeout:** 1 second
**Retries:** 2

| Call | HTTP | Returns | Fails with |
|---|---|---|---|
| `products(ids: List of ProductId)` | `POST /products/batch` | `List of Product` | `UnknownProduct` on 404 |

### service `Inventory`

**Base URL:** setting `inventory_url`
**Timeout:** 2 seconds

| Call | HTTP | Returns | Fails with |
|---|---|---|---|
| `warehouses()` | `GET /warehouses` | `List of Warehouse` | |
| `reserve(items: List of LineItem, warehouse: WarehouseId)` | `POST /reservations` | `Reservation` | `OutOfStock` on 409 |
| `release(reservation: Reservation)` | `DELETE /reservations/{reservation.id}` | nothing | |

### service `Payments`

**Base URL:** setting `payments_url`
**Timeout:** 5 seconds
**Retries:** 2, network errors only. Safe because every charge carries the order ID as an idempotency key.

| Call | HTTP | Returns | Fails with |
|---|---|---|---|
| `charge(customer: CustomerId, amount: Money, order: OrderId)` | `POST /charges` | `PaymentId` | `Declined` on 402 |
| `refund(payment: PaymentId)` | `POST /charges/{payment}/refund` | nothing | |

### service `FastShip`

**Base URL:** setting `fastship_url`
**Timeout:** 800 milliseconds

| Call | HTTP | Returns | Fails with |
|---|---|---|---|
| `rate(from: GeoPoint, to: GeoPoint, weight_kg: Decimal)` | `POST /rates` | `ShippingRate` | `NoService` on 422 |
| `book(order: OrderId, from: GeoPoint, to: GeoPoint, weight_kg: Decimal)` | `POST /shipments` | `ShipmentId` | |
| `track(shipment: ShipmentId)` | `GET /shipments/{shipment}` | `Tracking` | |

### service `ParcelCo`

**Base URL:** setting `parcelco_url`
**Timeout:** 800 milliseconds

| Call | HTTP | Returns | Fails with |
|---|---|---|---|
| `rate(from: GeoPoint, to: GeoPoint, weight_kg: Decimal)` | `POST /v2/quote` | `ShippingRate` | `NoService` on 422 |
| `book(order: OrderId, from: GeoPoint, to: GeoPoint, weight_kg: Decimal)` | `POST /v2/labels` | `ShipmentId` | |
| `track(shipment: ShipmentId)` | `GET /v2/labels/{shipment}/status` | `Tracking` | |

### service `Notifications`

**Base URL:** setting `notify_url`
**Timeout:** 1 second

| Call | HTTP | Returns | Fails with |
|---|---|---|---|
| `order_confirmation(order: Order, email: Text)` | `POST /emails/order-confirmation` | nothing | |
| `track_event(name: Text, order: OrderId)` | `POST /events` | nothing | |

## Rust services

### service `OrderStore`

Orders live in Postgres as JSON documents. This is easier to write directly in
Rust with sqlx than to describe in a table.

**Implemented in:** Rust (below)

| Call | Returns | Fails with |
|---|---|---|
| `next_id()` | `OrderId` | |
| `load(id: OrderId)` | `Order` | `OrderNotFound` |
| `save(order: Order)` | nothing | |
| `recent_for(customer: CustomerId, limit: Int)` | `List of Order` | |

```rust
use sqlx::types::Json;

// Implements the trait `mgtc` generates from `service OrderStore` above.
// `self.extra.db` is the pool defined in AppExtra in shop.mgt.md.
impl OrderStore for App {
    async fn next_id(&self) -> Result<OrderId, OrderStoreError> {
        let (id,): (i64,) = sqlx::query_as("select nextval('order_ids')")
            .fetch_one(&self.extra.db)
            .await
            .map_err(OrderStoreError::other)?;
        Ok(OrderId(id))
    }

    async fn load(&self, id: OrderId) -> Result<Order, OrderStoreError> {
        let row: Option<(Json<Order>,)> = sqlx::query_as("select body from orders where id = $1")
            .bind(id.0)
            .fetch_optional(&self.extra.db)
            .await
            .map_err(OrderStoreError::other)?;
        row.map(|(body,)| body.0).ok_or(OrderStoreError::OrderNotFound)
    }

    async fn save(&self, order: &Order) -> Result<(), OrderStoreError> {
        sqlx::query(
            "insert into orders (id, customer, body) values ($1, $2, $3)
             on conflict (id) do update set body = excluded.body",
        )
        .bind(order.id().0)
        .bind(order.customer().0)
        .bind(Json(order))
        .execute(&self.extra.db)
        .await
        .map_err(OrderStoreError::other)?;
        Ok(())
    }

    async fn recent_for(&self, customer: CustomerId, limit: i64) -> Result<Vec<Order>, OrderStoreError> {
        let rows: Vec<(Json<Order>,)> = sqlx::query_as(
            "select body from orders where customer = $1 order by id desc limit $2",
        )
        .bind(customer.0)
        .bind(limit)
        .fetch_all(&self.extra.db)
        .await
        .map_err(OrderStoreError::other)?;
        Ok(rows.into_iter().map(|(body,)| body.0).collect())
    }
}
```

### service `OrderCache`

An in-process cache in front of the store. A miss is a failure (`NotCached`), which
is what lets `first of` in [order_details](checkout.mgt.md#order-details) fall
through to the database.

**Implemented in:** Rust (below)

| Call | Returns | Fails with |
|---|---|---|
| `get(id: OrderId)` | `Order` | `NotCached` |
| `put(order: Order)` | nothing | |
| `forget(id: OrderId)` | nothing | |

```rust
impl OrderCache for App {
    async fn get(&self, id: OrderId) -> Result<Order, OrderCacheError> {
        self.extra.cache.get(&id).await.ok_or(OrderCacheError::NotCached)
    }

    async fn put(&self, order: &Order) -> Result<(), OrderCacheError> {
        self.extra.cache.insert(order.id(), order.clone()).await;
        Ok(())
    }

    async fn forget(&self, id: OrderId) -> Result<(), OrderCacheError> {
        self.extra.cache.invalidate(&id).await;
        Ok(())
    }
}
```

## Helpers

### workflow `track(carrier: Carrier, shipment: ShipmentId) -> Tracking`

A shipment is tracked through whichever carrier booked it. A decision table can pick
a service call as well as a value.

| carrier | → call |
|---|---|
| `FastShip` | `FastShip.track(shipment)` |
| `ParcelCo` | `ParcelCo.track(shipment)` |
