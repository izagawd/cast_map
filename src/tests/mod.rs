//! Tests for the cast maps. The `slotmap_backed` and `gen_map_backed` modules
//! insert through coercions, so they also need the `coerce_unsized` feature.

#[cfg(feature = "gen_map")]
mod dense_gen_map_backed;
mod erased_values;
#[cfg(all(feature = "gen_map", feature = "coerce_unsized"))]
mod gen_map_backed;
#[cfg(all(feature = "slotmap", feature = "coerce_unsized"))]
mod slotmap_backed;
mod vec_backed;

#[derive(Debug, PartialEq)]
struct Dog {
    name: String,
}

#[derive(Debug, PartialEq)]
struct Cat {
    lives: u32,
}
