// SPDX-License-Identifier: MIT
/*
 * Copyright (c) [2023 - Present] Emily Matheys <emilymatt96@gmail.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use crate::{
    array,
    ops::RangeInclusive,
    types::{PolygonExtents, SameSizeMat},
};
use nalgebra::{Const, DimMin, Point, RealField, Scalar};
use num_traits::{Bounded, NumOps};

/// Various utility functions regarding point clouds of 2 or 3 dimensions.
pub mod point_cloud;

#[cfg_attr(
    feature = "tracing",
    tracing::instrument("Calculate Distance Squared", skip_all, level = "trace")
)]
pub(crate) fn distance_squared<T, const N: usize>(point_a: &Point<T, N>, point_b: &Point<T, N>) -> T
where
    T: Copy + Default + NumOps + Scalar,
{
    point_a
        .iter()
        .zip(point_b.iter())
        .map(|(&x, &y)| {
            let diff = x - y;
            diff * diff
        })
        .fold(T::default(), |acc, x| acc + x)
}

/// This function calculates the extents of the polygon, i.e., the minimum and maximum values for each coordinate dimension.
///
/// # Generics
/// * `T`: one of [`f32`] or [`f64`].
/// * `N`: a constant generic of type [`usize`].
///
/// # Arguments
/// * `polygon`: a slice of [`Point`].
///
/// # Returns
/// See [`PolygonExtents`]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument("Calculate Polygon Extents", skip_all, level = "info")
)]
pub fn calculate_polygon_extents<T, const N: usize>(polygon: &[Point<T, N>]) -> PolygonExtents<T, N>
where
    T: Bounded + Copy + RealField,
{
    let mut extents_accumulator: [RangeInclusive<T>; N] =
        array::from_fn(|_| <T as Bounded>::max_value()..=<T as Bounded>::min_value());

    for vertex in polygon.iter() {
        for (extent_for_dimension, vertex_coord) in
            extents_accumulator.iter_mut().zip(vertex.coords.iter())
        {
            *extent_for_dimension = extent_for_dimension.start().min(*vertex_coord)
                ..=extent_for_dimension.end().max(*vertex_coord);
        }
    }

    extents_accumulator
}

#[cfg_attr(
    feature = "tracing",
    tracing::instrument("Verify Matrix Determinant", skip_all, level = "info")
)]
pub(crate) fn verify_rotation_matrix_determinant<T, const N: usize>(
    mut u: SameSizeMat<T, N>,
    v_t: SameSizeMat<T, N>,
) -> SameSizeMat<T, N>
where
    T: Copy + RealField,
    Const<N>: DimMin<Const<N>, Output = Const<N>>,
{
    if (u * v_t).determinant() < T::zero() {
        u.column_mut(N - 1)
            .iter_mut()
            .for_each(|element| *element *= T::one().neg()); // Reverse the last column
    }
    u * v_t
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::Vec;
    use nalgebra::{Matrix2, Point, Point2, Point3};

    #[test]
    fn test_calculate_polygon_extents() {
        // Given:
        // A set of polygon vertices
        let polygon = Vec::from([
            Point2::new(1.0, 1.0),
            Point2::new(1.0, 4.0),
            Point2::new(5.0, 4.0),
            Point2::new(5.0, 1.0),
        ]);

        // When:
        // Calculating the extents
        let extents = calculate_polygon_extents(&polygon);
        assert_eq!(
            extents,
            [RangeInclusive::new(1.0, 5.0), RangeInclusive::new(1.0, 4.0)]
        );
    }

    #[test]
    fn test_calculate_polygon_extents_empty_polygon() {
        // An empty polygon
        let polygon: Vec<Point<f64, 2>> = Vec::new();

        // Calculating the extents
        let extents = calculate_polygon_extents(&polygon);

        // Expect the extents to be [max_value..=min_value] for x and y respectively
        assert_eq!(
            extents,
            [
                RangeInclusive::new(f64::MAX, f64::MIN),
                RangeInclusive::new(f64::MAX, f64::MIN)
            ]
        );
    }

    #[test]
    fn test_distance_squared() {
        let point_a = Point3::new(1.0, 2.0, 3.0);
        let point_b = Point3::new(4.0, 5.0, 6.0);
        assert_eq!(distance_squared(&point_a, &point_b), 27.0)
    }

    #[test]
    fn test_verify_rotation_matrix_determinant() {
        let mat_a = Matrix2::new(2.0, 3.0, 2.0, 1.0);
        let mat_b = Matrix2::new(-1.0, 0.0, 0.0, -1.0);

        let regular_dot = mat_a * mat_b;
        assert!(regular_dot.determinant() < 0.0);

        let func_dot = verify_rotation_matrix_determinant(mat_a, mat_b);

        // Verify second column is actually reversed by this function
        assert!(func_dot.determinant() >= 0.0);
        assert_eq!(func_dot.m12, -regular_dot.m12);
        assert_eq!(func_dot.m22, -regular_dot.m22);
    }
}
