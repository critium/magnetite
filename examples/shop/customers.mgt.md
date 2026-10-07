# Customers

A customer is regular, premium, or corporate. The kind of customer decides their discount.

## Uses

- `GeoPoint`, `Region` from [Geography](geo.mgt.md)

## Identifiers

| Identifier | Meaning |
|---|---|
| `CustomerId` | A customer account |

## Types

### `Customer` *(sealed)*

Every customer has these fields. The three kinds below add their own.

| Field | Type | Constraint |
|---|---|---|
| `id` | `CustomerId` | |
| `name` | `Text` | not empty |
| `email` | `Text` | matching /^[^@\s]+@[^@\s]+$/ |
| `region` | `Region` | |
| `location` | `GeoPoint` | |

| Rule | Returns | Formula |
|---|---|---|
| `discount()` | `Percent` | `0%` |

### `Regular` extends `Customer`

No extra fields. Regular customers keep the default 0% discount.

### `Premium` extends `Customer`

| Field | Type | Constraint |
|---|---|---|
| `tier` | `Tier` | |

#### override rule `discount() -> Percent`

| tier | → discount |
|---|---|
| `Gold` | 10% |
| `Platinum` | 15% |

### `Tier`

- `Gold`
- `Platinum`

### `Corporate` extends `Customer`

Corporate accounts negotiate their own discount, up to 30%.

| Field | Type | Constraint |
|---|---|---|
| `account_manager` | `Text` | not empty |
| `negotiated` | `Percent` | between 0% and 30% |

| Rule | Returns | Formula |
|---|---|---|
| override `discount()` | `Percent` | `negotiated` |

## Examples

| Fixture | Value |
|---|---|
| `ada` | `Premium(id: 7, name: "Ada", email: "ada@example.com", region: Europe, location: GeoPoint(lat: 51.5, lon: -0.12), tier: Platinum)` |

| Check | Expect |
|---|---|
| `ada.discount()` | 15% |
