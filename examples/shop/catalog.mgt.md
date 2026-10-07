# Catalog

Products, warehouses, and the pricing rules that don't depend on who is buying.

## Uses

- `GeoPoint`, `Region` from [Geography](geo.mgt.md)

## Identifiers

| Identifier | Meaning |
|---|---|
| `ProductId` | A product in the catalog |
| `WarehouseId` | A warehouse that ships orders |

## Types

### `Product`

| Field | Type | Constraint |
|---|---|---|
| `id` | `ProductId` | |
| `name` | `Text` | not empty |
| `category` | `Category` | |
| `list_price` | `Money` | at least $0.01 |
| `weight_kg` | `Decimal` | between 0.01 and 500 |

### `Category`

- `Books` — reduced tax in some regions
- `Electronics`
- `Grocery` — reduced tax in most regions

### `Warehouse`

| Field | Type | Constraint |
|---|---|---|
| `id` | `WarehouseId` | |
| `name` | `Text` | not empty |
| `location` | `GeoPoint` | |
| `region` | `Region` | |

## Pricing rules

### rule `tax_rate(region: Region, category: Category) -> Percent`

Sales tax by region and category. Rows are checked top to bottom, and `_` matches
anything. `mgtc` checks that every region/category combination is covered and
warns about rows that can never match.

| region | category | → rate |
|---|---|---|
| `Europe` | `Books` | 5% |
| `Europe` | `Grocery` | 7% |
| `Europe` | `_` | 20% |
| `NorthAmerica` | `_` | 8% |
| `AsiaPacific` | `Grocery` | 0% |
| `AsiaPacific` | `_` | 10% |

### rule `bulk_discount(qty: Int) -> Percent`

Buying many of the same product lowers its unit price. A cell can hold a
comparison (`>= 100`) as well as a value.

| qty | → discount |
|---|---|
| `>= 100` | 15% |
| `>= 20` | 5% |
| `_` | 0% |

## Examples

| Check | Expect |
|---|---|
| `tax_rate(Europe, Books)` | 5% |
| `tax_rate(NorthAmerica, Grocery)` | 8% |
| `bulk_discount(19)` | 0% |
| `bulk_discount(20)` | 5% |
