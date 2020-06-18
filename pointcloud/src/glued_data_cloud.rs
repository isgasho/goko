//! Simple gluing structs that abstracts away multi cloud access

use crate::errors::{PointCloudError, PointCloudResult};

use crate::{PointIndex, PointRef};

use crate::base_traits::*;

use fxhash::FxBuildHasher;
use hashbrown::HashMap;

/// For large numbers of underlying point clouds
#[derive(Debug)]
pub struct HashGluedCloud<D: PointCloud> {
    addresses: HashMap<PointIndex, (usize, PointIndex), FxBuildHasher>,
    data_sources: Vec<D>,
}

impl<D: PointCloud> HashGluedCloud<D> {
    /// Creates a new one, preserves the order in the supplied vec.
    pub fn new(data_sources: Vec<D>) -> HashGluedCloud<D> {
        let mut addresses = HashMap::with_hasher(FxBuildHasher::default());
        let mut pi: PointIndex = 0;
        for (i, source) in data_sources.iter().enumerate() {
            for j in 0..source.len() {
                addresses.insert(pi, (i, j as PointIndex));
                pi += 1;
            }
        }
        HashGluedCloud {
            addresses,
            data_sources,
        }
    }

    /// Extracts the underlying point clouds
    pub fn take_data_sources(self) -> Vec<D> {
        self.data_sources
    }

    #[inline]
    fn get_address(&self, pn: PointIndex) -> PointCloudResult<(usize, PointIndex)> {
        match self.addresses.get(&pn) {
            Some((i, j)) => Ok((*i, *j)),
            None => Err(PointCloudError::DataAccessError {
                index: pn,
                reason: "address not found".to_string(),
            }),
        }
    }
}

impl<D: PointCloud> PointCloud for HashGluedCloud<D> {
    type Metric = D::Metric;
    /// Returns a slice corresponding to the point in question. Used for rarely referenced points,
    /// like outliers or leaves.
    fn point(&self, pn: PointIndex) -> PointCloudResult<PointRef> {
        let (i, j) = self.get_address(pn)?;
        self.data_sources[i].point(j)
    }

    /// Total number of points in the point cloud
    fn len(&self) -> usize {
        self.data_sources.iter().fold(0, |acc, mm| acc + mm.len())
    }

    /// Total number of points in the point cloud
    fn is_empty(&self) -> bool {
        if self.data_sources.is_empty() {
            false
        } else {
            self.data_sources.iter().all(|m| m.is_empty())
        }
    }

    /// The names of the data are currently a shallow wrapper around a usize.
    fn reference_indexes(&self) -> Vec<PointIndex> {
        self.addresses.keys().cloned().collect()
    }

    /// Dimension of the data in the point cloud
    fn dim(&self) -> usize {
        self.data_sources[0].dim()
    }
}

impl<D: LabeledCloud> LabeledCloud for HashGluedCloud<D> {
    type Label = D::Label;
    type LabelSummary = D::LabelSummary;

    fn label(&self, pn: PointIndex) -> PointCloudResult<Option<&Self::Label>> {
        let (i, j) = self.get_address(pn)?;
        self.data_sources[i].label(j)
    }
    fn label_summary(&self, pns: &[PointIndex]) -> PointCloudResult<Self::LabelSummary> {
        let mut summary = Self::LabelSummary::default();
        for pn in pns {
            let (i, j) = self.get_address(*pn)?;
            summary.add(self.data_sources[i].label(j));
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_sources::tests::*;
    use crate::data_sources::*;
    use crate::distances::*;
    use crate::label_sources::*;
    use crate::{Point, PointRef};

    pub fn build_glue_random_labeled_test(
        partitions: usize,
        count: usize,
        data_dim: usize,
        labels_dim: usize,
    ) -> HashGluedCloud<SimpleLabeledCloud<DataRam<L2>, VecLabels>> {
        HashGluedCloud::new(
            (0..partitions)
                .map(|_i| build_ram_random_labeled_test(count, data_dim, labels_dim))
                .collect(),
        )
    }

    pub fn build_glue_random_test(
        partitions: usize,
        count: usize,
        data_dim: usize,
    ) -> HashGluedCloud<DataRam<L2>> {
        HashGluedCloud::new(
            (0..partitions)
                .map(|_i| build_ram_random_test(count, data_dim))
                .collect(),
        )
    }

    pub fn build_glue_fixed_labeled_test(
        partitions: usize,
        count: usize,
        data_dim: usize,
        labels_dim: usize,
    ) -> HashGluedCloud<SimpleLabeledCloud<DataRam<L2>, VecLabels>> {
        HashGluedCloud::new(
            (0..partitions)
                .map(|_i| build_ram_fixed_labeled_test(count, data_dim, labels_dim))
                .collect(),
        )
    }

    pub fn build_glue_fixed_test(
        partitions: usize,
        count: usize,
        data_dim: usize,
    ) -> HashGluedCloud<DataRam<L2>> {
        HashGluedCloud::new(
            (0..partitions)
                .map(|_i| build_ram_fixed_test(count, data_dim))
                .collect(),
        )
    }

    #[test]
    fn address_correct() {
        let pc = build_glue_fixed_test(5, 2, 3);
        println!("{:?}", pc);

        for i in &[1, 3, 5, 7, 9] {
            let address = pc.get_address(*i).unwrap();
            assert_eq!(address, ((i / 2) as usize, i % 2));
        }
    }

    #[test]
    fn point_correct() {
        let pc = build_glue_fixed_test(5, 2, 3);
        println!("{:?}", pc);

        for i in &[1, 3, 5, 7, 9] {
            let point = pc.point(*i).unwrap();
            match point {
                PointRef::Dense(val) => {
                    for d in val {
                        assert_approx_eq!(1.0, d);
                    }
                }
                PointRef::Sparse(_, _) => panic!("Should return a sparse datum"),
            };
        }
    }

    #[test]
    fn distance_correct() {
        let pc = build_glue_fixed_test(5, 2, 3);
        println!("{:?}", pc);

        let indexes = [1, 3, 5, 7, 9];
        let point = Point::Dense(vec![0.0; 5]);
        let point_ref = point.to_ref();

        let dists = pc.distances_to_point(&point_ref, &indexes).unwrap();
        for d in dists {
            assert_approx_eq!(3.0f32.sqrt(), d);
        }
        let dists = pc.distances_to_point_index(0, &indexes).unwrap();
        for d in dists {
            assert_approx_eq!(3.0f32.sqrt(), d);
        }
    }
}
