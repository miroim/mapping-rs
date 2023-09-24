use crate::types::SameSizeMat;
use nalgebra::{ArrayStorage, ComplexField, Const, Matrix, Point, RealField, Vector};
use num_traits::AsPrimitive;

#[cfg(not(feature = "std"))]
use core::{array, iter::Sum, ops::Add};
#[cfg(feature = "std")]
use std::{array, iter::Sum, ops::Add};

#[cfg(feature = "tracing")]
use tracing::instrument;

/// Calculates the mean(centeroid) of the point cloud.
///
/// # Arguments
/// * points: a slice of [`Point`], representing the point cloud.
///
/// # Returns
/// A [`Point`], representing the point cloud centeroid.
///
/// # Panics
/// In debug builds, this function will panic if `points` is an empty slice, to avoid dividing by 0.
#[inline]
#[cfg_attr(feature = "tracing", instrument("Calculate Mean Point", skip_all))]
pub(crate) fn calculate_mean<T, const N: usize>(points: &[Point<T, N>]) -> Point<T, N>
where
    T: Copy + Default + ComplexField,
    usize: AsPrimitive<T>,
{
    debug_assert!(!points.is_empty());

    let zeros: [T; N] = array::from_fn(|_| T::default());
    points.iter().fold(Point::<T, N>::from(zeros), |acc, it| {
        Point::from(acc.coords + it.coords)
    }) / points.len().as_()
}

/// Calculates the Mean Squared Error between two point clouds.
/// # Generics
/// * T: either an [`f32`] or [`f64`]
///
/// # Arguments
/// * `transformed_points_a`: a slice of [`Point`], representing the source point cloud, transformed by the current [`Isometry`](nalgebra::Isometry) matrix.
/// * `points_b`: a slice of [`Point`], representing the point cloud to match against.
///
/// # Returns
/// A [`T`], representing the sum of all squared distances between each point in `transformed_points_a` and its corresponding point in `points_b`.
#[inline]
#[cfg_attr(feature = "tracing", instrument("Calculate MSE", skip_all))]
pub(crate) fn calculate_mse<T, const N: usize>(
    transformed_points_a: &[Point<T, N>],
    points_b: &[Point<T, N>],
) -> T
where
    T: ComplexField + Copy + Sum,
{
    transformed_points_a
        .iter()
        .zip(points_b.iter())
        .map(|(transformed_a, point_b)| {
            // Again, there has GOT to be a distance squred function in nalgebra, just didn't dig enough to find it
            // Also, we are doing duplicate transforming of the points, perhaps merge the two
            (0..N)
                .map(|access_idx| {
                    (transformed_a.coords.data.0[0][access_idx]
                        - point_b.coords.data.0[0][access_idx])
                        .powi(2)
                })
                .sum::<T>()
        })
        .sum::<T>()
}

/// Calculates the outer product of two `N` length [`Vector`]s.
///
/// # Arguments
/// * `point_a`: A reference to the first [`Vector`].
/// * `point_b`: A reference to the second [`Vector`].
///
/// # Returns
/// A [`SameSizeMat`] of size `N` by `N`, containing the outer product.
#[inline]
#[cfg_attr(feature = "tracing", instrument("Calculate Outer Product", skip_all))]
pub(crate) fn outer_product<T, const N: usize>(
    point_a: &Vector<T, Const<N>, ArrayStorage<T, N, 1>>,
    point_b: &Vector<T, Const<N>, ArrayStorage<T, N, 1>>,
) -> SameSizeMat<T, N>
where
    T: ComplexField + Copy,
{
    Matrix::from_data(ArrayStorage(array::from_fn(|b_idx| {
        array::from_fn(|a_idx| point_a.data.0[0][a_idx] * point_b.data.0[0][b_idx])
    })))
}

/// Calculates the estimated transformation matrix between the two point clouds.
///
/// # Arguments
/// * `points_a`: a slice of [`Point`], representing the source point cloud.
/// * `points_b`: a slice of [`Point`], representing the target point cloud.
///
/// # Returns
/// A tuple of
/// * [`SameSizeMat<T, N>`], representing the covariance matrix of the outer products of the centered point clouds.
/// * [`Point`], representing the `points_a` centeroid.
/// * [`Point`], representing the `points_b` centeroid.
///
/// # Panics
/// See [`calculate_mean`]
#[inline]
#[cfg_attr(feature = "tracing", instrument("Estimate Transform", skip_all))]
pub(crate) fn transform_using_centeroids<T, const N: usize>(
    points_a: &[Point<T, N>],
    points_b: &[Point<T, N>],
) -> (SameSizeMat<T, N>, Point<T, N>, Point<T, N>)
where
    T: Copy + ComplexField + RealField + Default,
    usize: AsPrimitive<T>,
    SameSizeMat<T, N>: Add<Output = SameSizeMat<T, N>>,
{
    let (mean_a, mean_b) = (calculate_mean(points_a), calculate_mean(points_b));
    let rot_mat = points_a.iter().zip(points_b.iter()).fold(
        Matrix::from_array_storage(ArrayStorage([[T::default(); N]; N])),
        |rot_mat, (point_a, point_b)| {
            let a_distance_from_c = point_a - mean_a;
            let b_distance_from_c = point_b - mean_b;
            rot_mat + outer_product(&a_distance_from_c, &b_distance_from_c)
        },
    );

    (rot_mat, mean_a, mean_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{Point3, Vector3};

    #[test]
    fn test_calculate_mean() {
        // Define a set of points
        let points: [Point<f64, 3>; 3] = [
            Point::from([1.0, 2.0, 3.0]),
            Point::from([4.0, 5.0, 6.0]),
            Point::from([7.0, 8.0, 9.0]),
        ];

        // Calculate mean
        let mean = calculate_mean(&points);
        assert_eq!(
            mean,
            Point::from([4.0, 5.0, 6.0]),
            "The mean point was not calculated correctly."
        );
    }

    #[test]
    fn test_calculate_mse() {
        // Define two sets of points
        let transformed_points_a: [Point<f64, 3>; 3] = [
            Point::from([1.0, 2.0, 3.0]),
            Point::from([4.0, 4.0, 4.0]),
            Point::from([7.0, 7.0, 7.0]),
        ];

        let points_b: [Point<f64, 3>; 3] = [
            Point::from([1.0, 1.0, 1.0]),
            Point::from([4.0, 5.0, 6.0]),
            Point::from([8.0, 8.0, 8.0]),
        ];

        // Calculate MSE
        let mse = calculate_mse(&transformed_points_a, &points_b);

        assert_eq!(
            mse, 13.0,
            "The calculated MSE does not match the expected value."
        );
    }

    #[test]
    fn test_outer_product() {
        // Define two vectors
        let point_a = Vector3::new(1.0, 2.0, 3.0);
        let point_b = Vector3::new(4.0, 5.0, 6.0);

        // Calculate outer product
        let result = outer_product(&point_a, &point_b);
        assert_eq!(
            result,
            SameSizeMat::from_data(ArrayStorage([
                [4.0, 8.0, 12.0],
                [5.0, 10.0, 15.0],
                [6.0, 12.0, 18.0]
            ])),
            "The calculated outer product does not match the expected value."
        );
    }

    #[test]
    fn test_transform_using_centeroids() {
        // Define two sets of points
        let points_a: [Point<f64, 3>; 3] = [
            Point::from([6.0, 4.0, 20.0]),
            Point::from([100.0, 60.0, 3.0]),
            Point::from([5.0, 20.0, 10.0]),
        ];

        let points_b: [Point<f64, 3>; 3] = [
            Point::from([40.0, 22.0, 12.0]),
            Point::from([10.0, 14.0, 10.0]),
            Point::from([7.0, 30.0, 20.0]),
        ];

        // Compute transform using centroids
        let (rot_mat, mean_a, mean_b) = transform_using_centeroids(&points_a, &points_b);
        assert_eq!(
            mean_a,
            Point3::new(37.0, 28.0, 11.0),
            "The calculated mean of points_a does not match the expected value."
        );
        assert_eq!(
            mean_b,
            Point3::new(19.0, 22.0, 14.0),
            "The calculated mean of points_b does not match the expected value."
        );
        assert_eq!(
            rot_mat,
            Matrix::from_data(ArrayStorage([
                [-834.0, -696.0, 273.0],
                [-760.0, -320.0, 56.0],
                [-382.0, -128.0, 8.0]
            ])),
            "The calculated rotation matrix does not match the expected value."
        );
    }
}
