# Geography

Where customers and warehouses are, and how far apart they are.

## Types

### `GeoPoint`

A point on the globe, in decimal degrees.

| Field | Type | Constraint |
|---|---|---|
| `lat` | `Decimal` | between -90 and 90 |
| `lon` | `Decimal` | between -180 and 180 |

### `Region`

- `NorthAmerica` — United States, Canada, Mexico
- `Europe` — EU, EEA and the UK
- `AsiaPacific` — everything else we ship to

## Distance

Great-circle distance needs trigonometry that's easier to write in Rust, so the
Rust block below implements it and Magnetite calls it like any other pure rule.

| Rule | Returns | Formula |
|---|---|---|
| `distance_km(from: GeoPoint, to: GeoPoint)` | `Decimal` | Rust: `distance_km` (below) |

> [!NOTE]
> `mgtc` generates the signature `fn distance_km(from: &GeoPoint, to: &GeoPoint) -> Decimal`.
> If the Rust below doesn't match it, the error points at this table row, not at generated code.

```rust
use rust_decimal::prelude::*;

const EARTH_RADIUS_KM: f64 = 6371.0;

/// Haversine distance, rounded to 0.1 km.
pub fn distance_km(from: &GeoPoint, to: &GeoPoint) -> Decimal {
    let lat1 = from.lat().to_f64().unwrap_or(0.0).to_radians();
    let lat2 = to.lat().to_f64().unwrap_or(0.0).to_radians();
    let d_lat = lat2 - lat1;
    let d_lon = (to.lon().to_f64().unwrap_or(0.0) - from.lon().to_f64().unwrap_or(0.0)).to_radians();

    let a = (d_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    Decimal::from_f64(EARTH_RADIUS_KM * c).unwrap_or_default().round_dp(1)
}
```

## Examples

| Fixture | Value |
|---|---|
| `berlin` | `GeoPoint(lat: 52.52, lon: 13.405)` |
| `paris` | `GeoPoint(lat: 48.8566, lon: 2.3522)` |

| Check | Expect |
|---|---|
| `distance_km(berlin, paris)` | between 875 and 880 |
| `distance_km(paris, paris)` | `0` |
