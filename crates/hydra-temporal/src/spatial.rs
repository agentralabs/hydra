//! Grid-based spatial partition index for manifold coordinates.

use crate::btree::{ManifoldCoord, MemoryId};
use crate::constants::{SPATIAL_PARTITION_GRID_SIZE, SPATIAL_PARTITION_MAX_PER_CELL};
use crate::errors::TemporalError;
use std::collections::HashMap;

/// Maps a floating-point coordinate component to a grid cell index.
fn coord_to_cell(val: f64) -> usize {
    // Clamp to [0, GRID_SIZE - 1] after flooring
    let idx = val.floor() as i64;
    idx.clamp(0, (SPATIAL_PARTITION_GRID_SIZE - 1) as i64) as usize
}

/// A grid-based spatial index for fast proximity queries.
///
/// The grid is `SPATIAL_PARTITION_GRID_SIZE` cells on each axis.
/// Each cell stores a list of memory IDs located within it.
pub struct SpatialPartitionIndex {
    cells: HashMap<(usize, usize, usize), Vec<MemoryId>>,
}

impl Default for SpatialPartitionIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SpatialPartitionIndex {
    /// Create a new, empty spatial partition index.
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    /// Insert a memory at the given manifold coordinate.
    pub fn insert(
        &mut self,
        memory_id: MemoryId,
        coord: &ManifoldCoord,
    ) -> Result<(), TemporalError> {
        let x = coord_to_cell(coord.x);
        let y = coord_to_cell(coord.y);
        let z = coord_to_cell(coord.z);
        let cell = self.cells.entry((x, y, z)).or_default();
        if cell.len() >= SPATIAL_PARTITION_MAX_PER_CELL {
            return Err(TemporalError::SpatialPartitionFull { x, y, z });
        }
        cell.push(memory_id);
        Ok(())
    }

    /// Get all memory IDs at the exact grid cell for the given coordinate.
    pub fn memories_at(&self, coord: &ManifoldCoord) -> Vec<&MemoryId> {
        let x = coord_to_cell(coord.x);
        let y = coord_to_cell(coord.y);
        let z = coord_to_cell(coord.z);
        self.cells
            .get(&(x, y, z))
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Get all memory IDs within `radius` grid cells of the given coordinate.
    ///
    /// This searches a cube of cells and returns all memories in those cells.
    pub fn memories_near(&self, coord: &ManifoldCoord, radius: usize) -> Vec<&MemoryId> {
        let cx = coord_to_cell(coord.x);
        let cy = coord_to_cell(coord.y);
        let cz = coord_to_cell(coord.z);

        let lo_x = cx.saturating_sub(radius);
        let hi_x = (cx + radius).min(SPATIAL_PARTITION_GRID_SIZE - 1);
        let lo_y = cy.saturating_sub(radius);
        let hi_y = (cy + radius).min(SPATIAL_PARTITION_GRID_SIZE - 1);
        let lo_z = cz.saturating_sub(radius);
        let hi_z = (cz + radius).min(SPATIAL_PARTITION_GRID_SIZE - 1);

        let mut results = Vec::new();
        for x in lo_x..=hi_x {
            for y in lo_y..=hi_y {
                for z in lo_z..=hi_z {
                    if let Some(ids) = self.cells.get(&(x, y, z)) {
                        results.extend(ids.iter());
                    }
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_query() {
        let mut idx = SpatialPartitionIndex::new();
        let coord = ManifoldCoord::new(5.5, 10.2, 3.9);
        idx.insert(MemoryId::from_value("m1"), &coord).unwrap();
        let found = idx.memories_at(&coord);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].as_str(), "m1");
    }

    #[test]
    fn near_query_finds_neighbors() {
        let mut idx = SpatialPartitionIndex::new();
        idx.insert(
            MemoryId::from_value("m1"),
            &ManifoldCoord::new(5.0, 5.0, 5.0),
        )
        .unwrap();
        idx.insert(
            MemoryId::from_value("m2"),
            &ManifoldCoord::new(6.0, 5.0, 5.0),
        )
        .unwrap();
        idx.insert(
            MemoryId::from_value("m3"),
            &ManifoldCoord::new(20.0, 20.0, 20.0),
        )
        .unwrap();
        let near = idx.memories_near(&ManifoldCoord::new(5.5, 5.0, 5.0), 1);
        assert_eq!(near.len(), 2);
    }
}
