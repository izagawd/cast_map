//! Tests for the cast maps. The `slotmap` and `gen_map` modules insert through
//! coercions, so they also need the `coerce_unsized` feature.

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
