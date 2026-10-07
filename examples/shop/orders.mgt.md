# Orders

An order is created at checkout, paid, then shipped. It can be cancelled until it ships.

## Uses

- `ProductId`, `Category` from [Catalog](catalog.mgt.md)
- `CustomerId` from [Customers](customers.mgt.md)

## Identifiers

IDs are whole numbers unless a **Type** column says otherwise. IDs issued by
outside systems are usually text.

| Identifier | Meaning | Type |
|---|---|---|
| `OrderId` | An order | `Int` |
| `PaymentId` | A captured payment at the payment provider | `Text` |
| `ShipmentId` | A booked shipment at a carrier | `Text` |

## Types

### `LineItem`

One product on an order, priced at the moment the order was placed.

| Field | Type | Constraint |
|---|---|---|
| `product` | `ProductId` | |
| `name` | `Text` | not empty |
| `category` | `Category` | |
| `unit_price` | `Money` | at least $0 |
| `qty` | `Int` | between 1 and 1000 |
| `weight_kg` | `Decimal` | at least 0 |

| Rule | Returns | Formula |
|---|---|---|
| `line_total()` | `Money` | `unit_price * qty` |

### `Order`

| Field | Type | Constraint |
|---|---|---|
| `id` | `OrderId` | |
| `customer` | `CustomerId` | |
| `items` | `List of LineItem` | at least 1 |
| `total` | `Money` | at least $0 |
| `carrier` | `Carrier` | |
| `status` | `OrderStatus` | |
| `payment` | `PaymentId` | optional |
| `shipment` | `ShipmentId` | optional |
| `placed_at` | `DateTime` | |

| Rule | Returns | Formula |
|---|---|---|
| `weight_kg()` | `Decimal` | `sum of item.weight_kg * item.qty for each item in items` |
| `is_open()` | `Bool` | `status is Placed or status is Paid or status is Shipped` |

### `Carrier`

- `FastShip` — next-day, pricier
- `ParcelCo` — 3–5 days, cheaper

### `OrderStatus`

- `Placed` — created at checkout, not yet paid
- `Paid` — payment captured
- `Shipped` — handed to the carrier
- `Delivered` — the carrier confirmed delivery
- `Cancelled` — stopped before shipping; any payment is refunded

| From | To |
|---|---|
| `Placed` | `Paid`, `Cancelled` |
| `Paid` | `Shipped`, `Cancelled` |
| `Shipped` | `Delivered` |

> [!IMPORTANT]
> `mgtc` checks every `becomes` against this table. An action that tries to move a
> `Shipped` order to `Cancelled` is a compile error, not a runtime surprise.

## Actions

Each action changes one order. If **Requires** is false, the action fails with the
message in **Otherwise** and nothing changes.

| Action | Requires | Otherwise | Then |
|---|---|---|---|
| `mark_paid(changes order: Order, payment: PaymentId)` | `order.status is Placed` | Only placed orders can be paid | `order.status becomes Paid`, `order.payment becomes payment` |
| `mark_shipped(changes order: Order, shipment: ShipmentId)` | `order.status is Paid` | The order must be paid before it ships | `order.status becomes Shipped`, `order.shipment becomes shipment` |
| `cancel(changes order: Order)` | `order.status is Placed or order.status is Paid` | Orders can't be cancelled once they have shipped | `order.status becomes Cancelled` |

## Examples

Fixtures are named sample values that any example in the package can use.

| Fixture | Value |
|---|---|
| `sample_item` | `LineItem(product: 1, name: "Dune", category: Books, unit_price: $12.00, qty: 2, weight_kg: 0.4)` |
| `sample_order` | `Order(id: 100, customer: 7, items: [sample_item], total: $24.00, carrier: ParcelCo, status: Placed, placed_at: 2026-10-01T09:00Z)` |

| Scenario | Given | When | Expect |
|---|---|---|---|
| a paid order can be cancelled | `order = sample_order with status = Paid` | `cancel(order)` | `order.status is Cancelled` |
| a shipped order can't be cancelled | `order = sample_order with status = Shipped` | `cancel(order)` | fails with "Orders can't be cancelled once they have shipped" |
| weight counts every unit | `order = sample_order` | | `order.weight_kg() is 0.8` |
