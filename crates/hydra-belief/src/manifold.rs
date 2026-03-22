//! Belief manifold — geometric representation of belief space.

use crate::constants::GEODESIC_STEP_SIZE;
use serde::{Deserialize, Serialize};

/// A position on the belief manifold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefPosition {
    /// Coordinates in belief space.
    pub coordinates: Vec<f64>,
}

impl BeliefPosition {
    /// Create a new position at the given coordinates.
    pub fn new(coordinates: Vec<f64>) -> Self {
        Self { coordinates }
    }

    /// Compute the Euclidean distance between two positions.
    pub fn distance(&self, other: &Self) -> f64 {
        let min_len = self.coordinates.len().min(other.coordinates.len());
        let sum_sq: f64 = (0..min_len)
            .map(|i| {
                let diff = self.coordinates[i] - other.coordinates[i];
                diff * diff
            })
            .sum();
        sum_sq.sqrt()
    }

    /// Take a geodesic step toward a target position.
    ///
    /// Moves a fraction of `GEODESIC_STEP_SIZE` toward the target.
    pub fn geodesic_step(&self, target: &Self) -> Self {
        let max_len = self.coordinates.len().max(target.coordinates.len());
        let mut new_coords = Vec::with_capacity(max_len);
        for i in 0..max_len {
            let a = self.coordinates.get(i).copied().unwrap_or(0.0);
            let b = target.coordinates.get(i).copied().unwrap_or(0.0);
            new_coords.push(a + GEODESIC_STEP_SIZE * (b - a));
        }
        Self {
            coordinates: new_coords,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_same_point() {
        let p = BeliefPosition::new(vec![1.0, 2.0, 3.0]);
        assert!((p.distance(&p)).abs() < f64::EPSILON);
    }

    #[test]
    fn distance_known_value() {
        let a = BeliefPosition::new(vec![0.0, 0.0]);
        let b = BeliefPosition::new(vec![3.0, 4.0]);
        assert!((a.distance(&b) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn geodesic_step_moves_toward_target() {
        let a = BeliefPosition::new(vec![0.0, 0.0]);
        let b = BeliefPosition::new(vec![10.0, 10.0]);
        let stepped = a.geodesic_step(&b);
        assert!(stepped.distance(&b) < a.distance(&b));
    }
}
