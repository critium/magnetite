# Plan: Magnetite — a readable domain language that compiles to Rust

> **Magnetite** (Fe₃O₄) is the other iron oxide, a mineral cousin of rust, so the name avoids Rust trademark issues. Naming: crates `magnetite-*`, compiler `mgtc`, files `*.mgt.md`, code fence tag `mgt`. Two names are already taken on crates.io: `magnetite` (an unrelated project; use `magnetite-lang` if a top-level crate is ever needed) and `magc`. This is not a formal trademark search.

## Context

**Goal.** A web service that is as fast and small as Rust, with business logic written in a language anyone on the team can read, even someone who doesn't know Rust.

**Ideas we rejected along the way:**
- *Java host with Rust kernels.* This only speeds up the hot paths. The JVM still sets the memory footprint and startup time.
- *Rust HTTP with Java domain code.* Rich web data doesn't cross the boundary cleanly, and the deployment story is poor.
- *"Engineers just write Rust."* This is workable, but Rust's readability for domain authors depends entirely on team discipline.

**Chosen direction.** A small domain-specific language. Source files are **Markdown documents**: prose and headings are documentation, and fenced `mgt` blocks contain the code. Code inside those blocks reads close to plain English. The compiler gives **compile-time guarantees stronger than Rust/Java defaults**: refinement types, exhaustive state handling, units and money, and pure rules. It generates a plain, readable **Rust crate**. It can also generate the HTTP layer: a Routes table becomes an axum router, and HTTP service tables become reqwest clients (see Endpoints and services). Hand-written Rust plugs in anywhere through ` ```rust ` blocks inside the same Markdown files, or through ordinary Rust code that calls the generated crate.

## Prior art

| Project | What it is | Overlap | How Magnetite differs |
|---|---|---|---|
| **Lemma** (`lemma-engine`, Rust, pre-1.0) | Declarative business-rules language that stakeholders can read, with units and percent types and opt-in explanation trees | Readable rules, units, purity, Rust underneath | Lemma is an **interpreted rules engine**. Magnetite compiles AOT to a Rust crate and models entities, inheritance, actions, workflows and HTTP. We borrow Lemma's idea of explanation trees (see Explain). |
| **Catala** (Inria, research) | Literate language for turning legislation into code, designed for review by lawyers; OCaml/Python backends | Prose and code in one document, readable by domain experts | Catala targets law and uses default logic. Magnetite targets services and compiles to Rust. |
| **Amoskeag** | Pure, statically validated rules/formula DSL written in Rust, with Ruby bindings | Pure rules | Only an embedded evaluator |
| **Drools, DMN/FEEL** (JVM) | The popular business-rules engines | Rules and decision tables that business users edit | Heavy JVM runtime, XML/tool-based authoring. Magnetite's Markdown decision tables cover the same need. |
| **Roc, Koka, Hylo, Swift** | Languages with value semantics and functional-but-in-place updates | Our ownership and purity model | General-purpose languages with their own compilers, not readable DSLs that target Rust |
| **Dafny, Ada/SPARK** | Verification, range types | Refinement types | Not readable to business people |
| **Gherkin/Cucumber** | Readable specs for tests | Readability | Untyped; describes tests, not logic |
| **Temporal, Ballerina** | Workflow orchestration | `workflow`, `together`, compensation | Runtime platforms. Magnetite checks workflows at compile time and generates plain tokio code. |

**Positioning:** no popular project combines a literate Markdown source, full domain modeling, AOT compilation to idiomatic Rust, and value semantics that hide the borrow checker. The real competitors are "write disciplined Rust" and Drools/DMN, so Magnetite has to read better than Rust and run lighter than Drools.

**Non-goals for v1:**
- general-purpose programming
- I/O inside `rule`s or `action`s (only `workflow`s talk to the outside world, through declared `service`s)
- explicit threads, locks, `async` or lifetimes in source code
- user-defined generic types (passing pure functions as arguments is allowed)
- other compilation targets
- Java interop

## Design principles
1. **Readable by a non-Rust engineer or a domain expert in an afternoon.** The language stays small.
2. **If it type-checks, the generated Rust compiles.** Users never see a `rustc` error, and CI enforces this.
3. **All diagnostics point at the `.mgt.md` source line,** with plain-English messages.
4. **The generated Rust is idiomatic and readable**, so it's easy to debug and audit, and is never treated as an unreadable blob.
5. **Entities refer to each other by ID, never by reference.** This avoids ownership problems and matches how DB-backed apps work.
6. **Sealed by default and explicit by default:** `override`, `open` and `optional` must all be written out.
7. **A pure core with effects at the edges:** `rule`s are pure, `action`s are pure as far as callers can observe, and only `workflow`s do I/O.
8. **Value semantics:** every variable behaves like an independent value. The compiler chooses borrow, move or clone, so users never see the borrow checker.

## Source format
- Files are `*.mgt.md`, which are valid Markdown and render on GitHub.
- **Markdown-native syntax.** As much of the language as possible is plain Markdown: tables declare records, decision tables, transitions, routes and settings; bulleted lists declare enums; links declare imports. That way GitHub, VS Code and every other Markdown viewer render the code with structure and color, even before a Magnetite grammar exists. The full set of recognized forms is under Markdown-native syntax below.
- Logic that doesn't fit a table goes in fenced ` ```mgt ` blocks. These use indentation (2 spaces) with no braces or semicolons, and comments start with `--`.
- ` ```rust ` blocks are compiled into the generated crate (interop). ` ```mermaid `, ` ```json ` and unrecognized tables are documentation only.
- Headings and prose become doc comments on the declarations that follow.
- A module is one file. Imports are links in a `## Uses` section, e.g. `- [Customers](customers.mgt.md)`. The `use customers` line in the early examples below is shorthand for the same thing.

The early examples below use the code-block form. The fuller example in `examples/shop/` uses the Markdown-native form.

Example (`examples/orders/domain/customers.mgt.md`):

````markdown
# Customers

A customer is regular, premium, or corporate. Discounts depend on the kind.

```mgt
id type CustomerId

type Customer (sealed)
  id:     CustomerId
  name:   Text, not empty
  joined: Date

  rule discount() -> Percent
    0%

type Regular extends Customer

type Premium extends Customer
  tier: Gold | Platinum

  override rule discount() -> Percent
    when tier is Gold     then 10%
    when tier is Platinum then 15%

type Corporate extends Customer
  account_manager: EmployeeId
  negotiated:      Percent, between 0% and 30%

  override rule discount() -> Percent
    negotiated
```
````

And `orders.mgt.md`:

````markdown
# Orders

```mgt
use customers

id type OrderId

type LineItem
  price: Money, at least 0
  qty:   Int, between 1 and 1000

type Order
  id:       OrderId
  customer: CustomerId
  items:    List of LineItem, at least 1
  status:   Draft | Paid | Shipped | Cancelled
  note:     Text, optional

  transitions status
    Draft -> Paid | Cancelled
    Paid  -> Shipped | Cancelled

rule total(order: Order) -> Money
  sum of item.price * item.qty for each item in order.items

rule price(order: Order, customer: Customer) -> Money
  total(order) less customer.discount()
```

## Shipping

An order can only ship once it is paid.

```mgt
action ship(order: Order)
  requires order.status is Paid  else "Order must be paid before shipping"
  then order.status becomes Shipped

example "gold customers get 10% off"
  given item     = LineItem(price: $10.00, qty: 3)
  given order    = Order(id: 1, customer: 7, items: [item], status: Draft)
  given customer = Premium(id: 7, name: "Ada", joined: 2024-01-01, tier: Gold)
  expect price(order, customer) is $27.00
```
````

## Language v1

### Types
| Category | Constructs |
|---|---|
| Primitives | `Int` (i64, checked arithmetic), `Decimal`, `Bool`, `Text`, `Date`, `DateTime`, `Money`, `Percent` |
| Identity | `id type OrderId`: a distinct newtype, so an `OrderId` can't be passed where a `CustomerId` is expected |
| Records | `type Name` followed by indented fields |
| Inline enums | `status: Draft \| Paid \| Shipped` |
| Collections | `List of T`, `Set of T`, `Map of K to V` |
| Optional | `field: T, optional` becomes `Option<T>`, and the compiler requires a check before use (no null) |
| Refinements | `at least`, `at most`, `between X and Y`, `not empty`, `matching /regex/`, and `at least N` on lists |

### Inheritance
- `type T (sealed)` is the default for hierarchies: every subtype must be declared in the same module. This gives **exhaustiveness checking**, so a new subtype produces errors at every rule that doesn't handle it.
- `type T (open)` is an explicit opt-in that allows subtypes in other modules. Exhaustiveness checking is lost, and rule calls use dynamic dispatch.
- `extends` inherits the parent's fields and rules. Overrides need `override`, and `super.rule()` calls the parent version.
- `rule r() -> T (abstract)` means every concrete subtype must implement the rule.
- Mixins: `trait Auditable` with fields and rules, used as `type Order has Auditable`. Only single `extends` is allowed, with no multiple class inheritance.
- **Narrowing**: after `if c is Premium` or inside `when c is Premium`, `c.tier` is usable without a cast.
- Hierarchies are limited to a depth of 3. Going deeper is a compile error with an explanation.

### Behavior
- **`rule`**: a pure function with no I/O and no mutation. Its body is a single expression: `when … then …`, `sum of … for each …`, `count of`, `any`/`all … in …`, arithmetic, and comparisons.
- **`action`**: changes one entity. `requires` clauses are checked in order and generate typed errors with the given message. `then x becomes y` performs the assignment. If a status field has a `transitions` block, every `becomes` is checked against it **statically** when it can be, and at runtime otherwise.
- **`example`**: a test written in the domain language that compiles to a Rust `#[test]`.
- **`workflow`**: the only construct that does I/O. It calls `service`s and may run them concurrently. See Concurrency.

### Static checks (guarantees)
- Type checking with local inference and no implicit conversions.
- Units: `Money + Int` is an error, while `Money * Int` and `Money less Percent` are allowed.
- Refinements are enforced at construction: the generated types have validated constructors, so invalid values can't exist. Literals are checked at compile time.
- Exhaustive `when` over enums and sealed hierarchies.
- Optional fields must be checked before they're used.
- State transitions are validated against `transitions`, and unreachable states produce a warning.
- Rules are pure. Integer arithmetic is checked, and overflow becomes a domain error instead of wrapping.
- **Aliasing:** the same entity can't be passed to two `changes` parameters. If distinctness can't be proven statically, the compiler adds a runtime ID check that raises `SameEntity`.
- **No changing a list while looping over it.** The error message suggests the `becomes … where …` rewrite.
- A `rule (…) -> …` parameter accepts only pure rules, never actions or workflows.
- **Collection refinements are used:** `first`/`largest`/`smallest` on a list declared `at least 1` return `T` instead of an optional.
- **Concurrency:** branches of `together` can't change the same value, and `in parallel` is only allowed on pure expressions.
- **Failures:** every `may fail with X` from a service must be handled or propagated explicitly. A workflow's error set is part of its type.

## Collections

Every list operation reads as an English phrase built around `for each`.

| Magnetite | Meaning | Generated Rust |
|---|---|---|
| `item.price * item.qty for each item in order.items` | map | `.iter().map(..).collect()` |
| `item for each item in order.items where item.qty > 10` | filter | `.iter().filter(..).cloned().collect()` |
| `item for each order in orders, for each item in order.items` | flat map (main form) | `.iter().flat_map(..)` |
| `all order.items for each order in orders` | flatten (shorthand) | `.iter().flat_map(..)` |
| `sum of x.price for each x in xs` | sum | `.map(..).sum()` |
| `count of x in xs where …` | count | `.filter(..).count()` |
| `any x in xs where …` / `all x in xs where …` | any/all | `.any(..)` / `.all(..)` |
| `first x in xs where …` | find → optional (or `T` if non-empty and unfiltered) | `.find(..)` |
| `largest … for each …` / `smallest …` | max/min | `.max()` / `.min()` |
| `xs sorted by f` / `… descending` | sort | `sort_by_key` |
| `xs grouped by x.customer` | group → `Map of K to List of V` | `BTreeMap` fold |
| `distinct x.customer for each x in orders` | unique | `BTreeSet` |
| `first 10 of xs` | take | `.take(10)` |

- `where` clauses can appear at every level of a nested `for each`. Types are narrowed inside them, so `c.tier for each c in customers where c is Premium` type-checks.
- **Flat-map mistakes are caught.** If a `List of List of T` appears where a `List of T` is expected, the error suggests `all … for each …`.
- Long pipelines are written as named `let` steps, never as one long chain.

```mgt
rule top_customers(orders: List of Order) -> List of CustomerId
  let paid    = order for each order in orders where order.status is Paid
  let totals  = (order.customer, total(order)) for each order in paid
  let ranked  = totals sorted by second descending
  first 10 of (pair.first for each pair in ranked)
```

## Ownership and passing

The user-facing model has three rules:
1. **Values, not references.** `let a = b` gives `a` its own value, and changing `a` never affects `b`.
2. **Mutation is declared.** Only `action`s change things, and only the parameters marked `changes`.
3. **Entities refer to each other by ID**, so no struct holds a reference and lifetimes are never needed.

```mgt
action apply_coupon(changes order: Order, coupon: Coupon, customer: Customer)
  requires coupon.valid_for(customer) else "Coupon not valid for this customer"
  then order.discount becomes coupon.amount
```

How the compiler maps this to Rust (ownership analysis in `magnetite-check`, which tries move, then borrow, then clone):

| DSL situation | Generated Rust |
|---|---|
| `rule` parameter | `&T` |
| `changes x` | `&mut T` |
| Struct fields | owned (`String`, `Vec<T>`, IDs) |
| `let a = b`, neither changed afterwards | `&b` |
| `let a = b`, last use of `b` | move |
| `let a = b`, both used and one changed | `b.clone()` |
| Small types (`Int`, `Money`, `Percent`, IDs, enums) | `Copy` |
| `for each` in a rule | `.iter()` |
| `for each` over a field of a `changes` parameter | `.iter_mut()` |
| `together` branches | shared `&` borrows only; `changes` actions run after the join |

How the cases that trip people up in hand-written Rust are handled:
- **The same entity passed to two `changes` parameters:** a compile error, or a runtime `SameEntity` check when the compiler can't tell.
- **Removing from a list while iterating:** a compile error that suggests the filter rewrite.
- **Holding a read while writing:** can't happen, because reads are values. The compiler moves or clones as needed.
- **Async:** workflows use `join`/`select` and never `spawn`, so nothing needs `'static` or `Arc`.

`mgtc build --explain-copies` lists every generated `.clone()` with its source line. A later optimization may switch large immutable data (`Text`, big lists) to `Arc`-backed persistent types, which makes clones O(1) with no source changes.

## Purity and functional updates

| Construct | Pure? |
|---|---|
| `rule` | Yes, always: no mutation, no I/O, deterministic |
| `action` | Yes, in what callers can observe: it's local mutation of a value nobody else can see |
| `workflow` | No; this is the only place that does I/O |

**Two spellings of the same update.** `changes` (in place) and `with` (functional) mean the same thing:

```mgt
ship(order)                                           -- action, in-place style
let shipped_order = order with status = Shipped       -- functional style
let repriced = order with items = (item with price = item.price less 5%) for each item in order.items
```

`with` compiles to an **in-place update** when the original isn't used afterwards (functional-but-in-place, as in Roc and Koka). Otherwise it clones first.

**Functions as values.** A parameter can have a rule type, `rule (A) -> B`, which accepts pure rules only:

```mgt
rule half_off(item: LineItem) -> LineItem
  item with price = item.price less 50%

rule apply_all(order: Order, adjust: rule (LineItem) -> LineItem) -> Order
  order with items = adjust(item) for each item in order.items
```

This generates `adjust: impl Fn(&LineItem) -> LineItem`. Inline lambdas (`item -> item.price`) are deferred; named rules stay the readable default.

Purity also enables `in parallel`, mock-free tests for rules, and possibly `rule … (cached)` memoization later.

## Concurrency

There are two models, and neither exposes threads, futures or locks.

**1. Data parallelism inside pure rules: `in parallel`, generated with rayon.**

```mgt
rule risk_scores(portfolios: List of Portfolio) -> List of Score
  score(p) for each p in portfolios, in parallel
```

```rust
pub fn risk_scores(portfolios: &[Portfolio]) -> Vec<Score> {
    portfolios.par_iter().map(|p| score(p)).collect()
}
```

It is always safe, because rules are pure and therefore always `Sync`. Order is preserved, so results equal the sequential version. The compiler warns when the list is small or the per-item work is trivial.

**2. Async I/O: `service` and `workflow`, generated with tokio.**

Services are declared in the domain file and implemented in hand-written Rust (sqlx, reqwest and so on) as async traits:

```mgt
service Payments
  charge(customer: CustomerId, amount: Money) -> Receipt, may fail with Declined

service Inventory
  reserve(items: List of LineItem) -> Reservation, may fail with OutOfStock
  release(reservation: Reservation)

service Customers
  load(id: CustomerId) -> Customer
```

Workflows coordinate them:

```mgt
workflow checkout(changes order: Order) -> Receipt
  together
    let customer    = Customers.load(order.customer)
    let reservation = Inventory.reserve(order.items)

  let amount  = price(order, customer)
  let receipt = Payments.charge(customer.id, amount) within 5 seconds

  pay(order)
  receipt

  if it fails
    Inventory.release(reservation)
```

| Magnetite | Generated Rust |
|---|---|
| `service S` | `pub trait S { async fn …(&self, …) -> Result<T, E>; }` |
| `workflow w(…)` | `pub async fn w<Svc: S1 + S2>(svc: &Svc, …) -> Result<T, WError>` |
| `together` block | `tokio::try_join!` |
| `first of` block | `tokio::select!` (the loser is cancelled) |
| `… within 5 seconds` | `tokio::time::timeout` → `WError::Timeout { step }` |
| `for each …, together, at most 10 at a time` | `futures::stream::iter(..).buffered(10)` |
| `if it fails` | compensation block run on the error path |
| `may fail with X` | variant `X` added to the workflow's error enum |

`example` blocks can stub services, e.g. `given Payments.charge returns Declined`, which generates a fake trait implementation.

Called from axum:

```rust
let mut order = app.repo.load(id).await?;
let receipt = orders_domain::checkout(&app, &mut order).await?;
app.repo.save(&order).await?;
```

More workflow forms (used in `examples/shop/checkout.mgt.md`):

| Form | Meaning |
|---|---|
| `first of` | The **first successful** result wins and the rest are cancelled. Fails only if all branches fail (e.g. a hedged cache-or-database read). |
| `together, ignoring failures` | Run all branches and collect only the successes into a `List`. Used for best-effort calls such as asking several carriers for a rate. |
| `each within 800 milliseconds` | A timeout applied to each branch |
| `x or fail with E` | Turns an optional (or an empty `the … with smallest …` result) into an error |
| `let` bindings inside `if it fails` | Every `let` from the main body is **optional** there (the step may not have run), so compensation is written `if payment is set then Payments.refund(payment)` |

## Markdown-native syntax

The compiler recognizes these Markdown structures. Everything else is documentation.

**The Markdown form is the default.** ` ```mgt ` blocks are an escape hatch for logic that reads better as continuous code, such as multi-list joins or long aggregations. Both forms parse to the same AST. The shop example has 2 ` ```mgt ` blocks across 7 files, down from 21 in its first draft.

| Markdown | Declares | Rule for recognizing it |
|---|---|---|
| Table `Rule \| Returns \| Formula`, with the formula as a code span (or `Rust: name` for an `extern rule`) | One-line rules. Under a type heading they become members; prefix `override` in the Rule cell to override. | Exact header row |
| Heading `` ### rule `name(params) -> T` `` + table `Field \| Value` | A rule that builds a record field by field, like a spreadsheet. A row can use the fields above it. | Exact header row |
| Table `Action \| Requires \| Otherwise \| Then` | Actions. **Otherwise** is the plain-text failure message, and **Then** is a comma-separated list of `x becomes y` code spans. | Exact header row |
| Heading `` #### workflow `name(params) -> T` `` + **numbered list** | A workflow. Numbered items run **in sequence**. An item with **Together** / **Together, ignoring failures, each within …** / **First of** runs its nested bulleted items **in parallel**. Other keywords: **When** `x` **is** (nested `Variant:` items), **Requires** `cond`, otherwise fail with `E`, and **Returns** `expr`. A paragraph **If it fails** + bulleted list is the compensation. Text after `—` is a comment. | Heading keyword `workflow` + ordered list |
| `` `name =` `` before a **Together, ignoring failures** / **First of** keyword | Binds the collected list, or the winning result | Position |
| Decision table cell holding `>= 100`, `< 5`, `between 1 and 9` | Comparison condition instead of an exact match | Starts with a comparison operator |
| Decision table whose `→` column holds service calls, under a `workflow` heading | Dispatch to a call (e.g. `track` by carrier) | Heading keyword `workflow` |
| Table `Call \| Returns \| Fails with` + `**Implemented in:** Rust (below)` | Service implemented in a ` ```rust ` block | Exact header row |
| Table `Fixture \| Value` | Named sample values for examples | Exact header row |
| Table `Check \| Expect` | Assertions on pure expressions | Exact header row |
| Heading `` ### Stubs: `name` `` + table `Call \| Returns` | A named set of service stubs | Exact header row |
| Table `Scenario \| Given \| Expect` (optionally a `When` column) | Examples. **Given** can name stubs and add overrides (`X fails with E`, `X takes 2 seconds`). **Expect** is a comma-separated list of assertions (`… is …`, `fails with …`, `was called once`). | Exact header row |
| `## Uses` list with links: `- [Orders](orders.mgt.md)` or ``- `Order`, `LineItem` from [Orders](orders.mgt.md)`` | Imports | Section heading `Uses`, list items containing a link to a `.mgt.md` file. Links are clickable on GitHub. |
| Heading `` ### `Order` `` + table with header `Field \| Type \| Constraint` | Record type | Exact header row |
| Heading `` ### `Premium` extends `Customer` `` + field table | Subtype | `extends` in the heading |
| Heading `` ### `Customer` *(sealed)* `` | Sealed hierarchy root | `*(sealed)*` / `*(open)*` in the heading |
| Heading `` ### `OrderStatus` `` + list where every item starts with a code span: ``- `Paid` — payment captured`` | Enum. The text after the dash is the variant's doc. | Every item starts with a code span |
| Table `Identifier \| Meaning`, optionally with a `Type` column (`Int` by default, `Text` for IDs issued by outside systems) | ID types | Exact header row |
| Table `From \| To` under an enum heading | Allowed state transitions | Exact header row |
| Heading `` ### rule `tax_rate(region: Region, category: Category) -> Percent` `` + table whose last column header starts with `→` | **Decision table.** Rows are checked top to bottom, and `_` matches anything. The compiler checks that every combination is covered and flags unreachable rows. | `→` column |
| ` ```mgt ` block under a type heading | Members (rules, actions) of that type | Position |
| `**Label:** value` lines under a heading | Properties, e.g. `**Base URL:** setting `crm_url``, `**Timeout:** 2 seconds`, `**Retries:** 2` | Bold label ending in a colon at the start of a paragraph |
| Table `Setting \| Env \| Type \| Default` | Typed configuration loaded from environment variables | Exact header row |
| Table `Method \| Path \| Workflow \| Success` | HTTP routes | Exact header row |
| Table `Error \| Status \| Message` | Error → HTTP status mapping | Exact header row |
| Table `Call \| HTTP \| Returns \| Fails with` under `` ### service `X` `` | HTTP client service | Exact header row |
| Table `Crate \| Version \| Features` | Cargo dependencies for Rust blocks | Exact header row |
| ` ```rust ` block | Rust compiled into this module of the generated crate (`rust ignore` for illustration only) | Fence tag |
| ` ```mermaid ` after `<!-- mgtc:diagram name -->` | A diagram **generated** from the named workflow or type. `mgtc doc` rewrites it, and `mgtc check` fails if it's stale, so it can't drift from the code. It is never parsed as source. | HTML comment marker |
| Alerts (`> [!NOTE]`), other tables, prose, images | Documentation | — |

**Coloring.**
- Tables, headings, code spans, links, alerts, and the ` ```rust `/` ```json `/` ```mermaid ` blocks are all colored by every Markdown renderer today.
- ` ```mgt ` blocks get coloring from:
  - a TextMate grammar shipped in a VS Code extension (M5), which also works with Shiki-based doc sites
  - GitHub Linguist registration later. Linguist only accepts languages already used in a meaningful number of public repositories.
- Putting declarations in tables keeps most of each file colored even without that grammar.

## Endpoints and services

- **Routes table → axum router.**
  - Path parameters bind to workflow parameters by name, and the JSON body binds to the remaining parameter.
  - A route may name a Rust function (`Rust: health`) in place of a workflow.
  - The compiler checks that the Error table maps **every** error each routed workflow can raise. An unmapped error is a compile error, never an accidental 500.
- **HTTP service table → reqwest client** that implements the service trait.
  - Base URL, timeout and retries come from `**Label:**` properties.
  - JSON goes through serde.
  - `Fails with X on 409` maps a status code to a typed error.
- **Services implemented in Rust:** declare the service in an ` ```mgt ` block and implement its generated trait in a ` ```rust ` block in the same file (e.g. an sqlx-backed store, a moka cache).
- **`extern rule`:** a pure function implemented in a ` ```rust ` block (e.g. haversine distance). The compiler generates the expected signature. A rustc error inside a Rust block is reported against that block's lines in the `.mgt.md` file. This is the one place rustc errors can legitimately appear.
- **App state:** the generated `App` holds the settings, the HTTP clients, and an optional `AppExtra` struct defined in a Rust block with an `init_extra(&Settings)` constructor. A `customize(Router) -> Router` hook in a Rust block adds middleware.

## Explain

Borrowed from Lemma: any pure `rule`, including decision tables, can produce an **explanation tree** showing why it returned what it did.
- Codegen emits two versions of each rule: `price_quote(..)` (no overhead) and `price_quote_explained(..) -> Explained<Quote>`, which records each sub-rule called, its inputs, and the decision-table row that matched.
- HTTP: any route accepts `?explain=true` and returns `{ "result": …, "explanation": … }`. Workflows include the trees of the rules they call.
- CLI: `mgtc explain examples/shop price_quote --input quote.json`.
- Tests: an `example` can assert on the explanation, e.g. `expect explanation of tax_rate matched row 3`.

## Rust code generation

| Magnetite | Generated Rust |
|---|---|
| `id type OrderId` | `pub struct OrderId(pub i64)` with serde, Eq, Hash |
| `type Order` (record) | `pub struct Order { … }` with private refined fields and getters |
| refined field `qty: Int, between 1 and 1000` | `pub struct Qty(i64)` with `Qty::new(v) -> Result<Qty, DomainError>` |
| inline enum | `pub enum OrderStatus { … }` |
| `field: T, optional` | `Option<T>` |
| sealed hierarchy | `CustomerBase` struct, one struct per subtype embedding `base`, `pub enum Customer { Regular(..), … }`, and `impl Customer { fn base(), fn name(), … }` accessors |
| `override rule` on a sealed hierarchy | a method on the enum that does `match self` and calls the subtype's method or the inherited default |
| open hierarchy | `pub trait Customer`, `impl Customer for Premium`, `Box<dyn Customer>` |
| `trait` mixin | Rust trait plus generated fields and impls in each type that uses it |
| `rule` | `pub fn` / `impl` method taking `&self` |
| `action` | `pub fn ship(&mut self) -> Result<(), OrderError>` |
| `requires … else "msg"` | variant `OrderError::ShipRequiresPaid` with a `Display` that returns the message |
| `example` | `#[test] fn gold_customers_get_10_off()` |
| `changes x: T` / rule param | `&mut T` / `&T` (see Ownership) |
| `x with f = v` | in-place update when the original isn't used afterwards, `clone()` + update otherwise |
| `rule (A) -> B` parameter | `impl Fn(&A) -> B` |
| `for each …` | iterator chains (see Collections) |
| `in parallel` | rayon `par_iter()` |
| `service` / `workflow` | async trait / generic `async fn` (see Concurrency) |
| Markdown prose | `///` doc comments |

Generated code depends on one small crate, `magnetite-runtime`, which provides `Money` (i64 minor units plus a currency fixed per package in v1), `Percent`, `Decimal` (via `rust_decimal`), date types (`time`), and `DomainError` helpers. Every generated item gets a `// source: orders.mgt.md:42` comment.

Integration with a Rust service: the generated crate is written to `target/magnetite/<pkg>` by a `build.rs` helper, or checked in by running `mgtc build`. The axum handler code looks like this:

```rust
let mut order: Order = repo.load(id).await?;
order.ship()?;                          // generated
repo.save(&order).await?;
```

## Compiler architecture (written in Rust)

```
just/                              (repo root)
  Cargo.toml                       workspace
  crates/
    magnetite-syntax/              Markdown extraction (pulldown-cmark), indentation-aware lexer,
                                   hand-written recursive-descent parser → AST with source spans
    magnetite-check/               name resolution, type checking, hierarchy building, refinements,
                                   exhaustiveness, transitions, purity
    magnetite-codegen-rust/        AST+types → Rust tokens (quote) → formatted source (prettyplease)
    magnetite-runtime/             Money, Percent, Decimal, DomainError: what generated code uses
    magnetite-build/               build.rs helper: compile *.mgt.md into OUT_DIR / target/magnetite
    mgtc/                          CLI: `mgtc check`, `mgtc build`, `mgtc doc`, `mgtc test`
  examples/orders/
    domain/customers.mgt.md, orders.mgt.md
    service/                       axum app: POST /orders, POST /orders/:id/ship, GET /orders/:id/price
  docs/LANGUAGE.md                 language reference, written as a .mgt.md file that compiles
  tests/
```

- Diagnostics use `ariadne` or `miette`, with source snippets, labels and a suggested fix.
- The parser is hand-written instead of generated, so error messages and recovery are better.
- A later `magnetite-lsp` crate (`tower-lsp`) will add hover with prose docs, go-to-definition and diagnostics.

## Milestones
1. **M0 — Spec by example.** Write `docs/LANGUAGE.md` and the Orders/Customers examples first. Review readability with someone who isn't a Rust engineer before writing compiler code.
2. **M1 — Core.** Markdown extraction, lexer, parser, records, primitives, id types, rules, and Rust codegen. Value semantics and the ownership analysis (move → borrow → clone) are core, so they land here. Basic collections: map, filter, nested `for each`, sum, count, any/all. The examples compile to Rust that builds.
3. **M2 — Refinements.** Validated constructors, compile-time checks on literals, units/money arithmetic rules, and refinement-aware `first`/`largest`/`smallest`.
4. **M3 — Inheritance.** Inline enums, sealed hierarchies, `extends`, `override`, `super`, `abstract`, narrowing (including inside `where`) and exhaustiveness.
5. **M4 — Behavior.** Actions with `changes`, `with` functional updates (in place when possible), `requires`, `transitions` checks, aliasing and mutate-while-iterating checks, rule-typed parameters, `in parallel`, the rest of the collection operations (sort, group, distinct, take), and `example` → `#[test]`.
6. **M5 — Integration.** `magnetite-build`, serde derives, the example axum service, `mgtc doc` (HTML with type-hierarchy and state diagrams), `--explain-copies`, and diagnostics polish.
7. **M6 — Extensions.** Mixins, open hierarchies, the LSP, and an escape hatch for calling named Rust functions.
8. **M7 — Workflows.** `service` → async traits, `workflow` → async fns, `together`, `first of`, `within`, bounded fan-out, `if it fails` compensation, error sets in types, and stubbed services in `example`s. This is the largest addition; it begins only once M1–M5 are stable.
9. **M8 — Markdown-native forms and HTTP.** Table-declared records/enums/IDs/transitions and `## Uses` links are parsed from M1 onward. This milestone adds decision tables (with completeness checks), Settings, the Routes and Error tables → axum, HTTP service tables → reqwest, Rust blocks (`extern rule`, service impls, `AppExtra`, `customize`), `?explain=true` and `mgtc explain`, generated Mermaid diagrams, and the VS Code grammar. The `examples/shop` service builds and runs at the end of this milestone.

**Later:**
- SMT-backed proofs for refinements (e.g. proving `qty * price` can't overflow)
- multi-currency Money
- `Arc`-backed persistent collections
- `(cached)` rules
- inline lambdas
- other backends

## Verification
- **Snapshot tests** (`insta`): each `.mgt.md` fixture produces expected Rust output, and each bad fixture produces expected diagnostic text.
- **The "never leak rustc errors" invariant:** every fixture that passes `mgtc check` has its generated crate built with `cargo check` in CI. Any rustc failure is a compiler bug.
- **Behavior:** `mgtc test examples/orders` runs every `example` block as a Rust test.
- **Generated code quality:** the generated Rust passes `cargo clippy -D warnings`.
- **Ownership:**
  - snapshot tests confirm that `with` and last-use moves produce **no** `.clone()`
  - the `--explain-copies` output is snapshot-tested
  - aliasing and mutate-while-iterating fixtures produce the expected DSL errors, never rustc borrow errors
- **Purity and parallelism:** a property test checks that `in parallel` gives the same result as the sequential version on random inputs. Calling a service from a rule, or passing an action to a `rule (…)` parameter, is a compile error.
- **Workflows:**
  - `example`s with stubbed services cover success, a `Declined` failure (the compensation runs), and a timeout (`Timeout { step }`)
  - a `together` test with two delayed stubs confirms they run concurrently: total time ≈ the max delay, not the sum
- **End to end:** run `cargo run -p orders-service`, then `curl` creates an order, pays, ships, and checks the price. Shipping an unpaid order returns 409 with the message from `requires`.
- **Footprint check:** RSS and p99 latency of the example service compared with a hand-written Rust equivalent. The target is within a few percent.
- **Readability check:** a domain reader who isn't a Rust engineer explains what `orders.mgt.md` does without help.

## Open questions
- A formal trademark check on "Magnetite" before any public release.
- Indentation-only syntax versus optional `end` keywords. Indentation is chosen for v1.
- Money: one currency per package in v1. Multi-currency needs a decision about exchange rates.
- Persistence: should the language describe DB mappings, or leave that entirely to hand-written sqlx code? The v1 answer is to leave it to sqlx.

## TODO
- **Mermaid diagrams as both documentation and functional source.** Today they are generated from workflows and types, and checked for staleness. Later options:
  - accept `stateDiagram-v2` as the source for state transitions, in place of the `From | To` table
  - decide whether sequence diagrams can be functional (as source, or as a contract checked against the workflow), given that they can't express arguments, rules, timeouts or compensation
