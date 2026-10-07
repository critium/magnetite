//! Checks a parsed Magnetite package (step 3 of the prototype).
//!
//! Planned passes: name resolution across `## Uses`, type checking with units,
//! refinement literals, hierarchy building with `override` and exhaustiveness,
//! `changes` aliasing, purity, and the move → borrow → clone ownership analysis.
