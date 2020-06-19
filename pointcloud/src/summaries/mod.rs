//! Summaries for some label types

use crate::pc_errors::PointCloudResult;
use std::cmp::Eq;
use std::default::Default;
use std::iter::Iterator;
use hashbrown::HashMap;

use smallvec::SmallVec;

use crate::base_traits::*;

/// A summary for a small number of categories.
#[derive(Clone, Debug)]
pub struct SmallCatSummary<T: Eq + Clone> {
    items: SmallVec<[(T, usize); 4]>,
    nones: usize,
    errors: usize,
}

impl<T: Eq + Clone> Default for SmallCatSummary<T> {
    fn default() -> Self {
        SmallCatSummary {
            items: SmallVec::new(),
            nones: 0,
            errors: 0,
        }
    }
}

impl<T: Eq + Clone> Summary<T> for SmallCatSummary<T> {
    fn add(&mut self, v: PointCloudResult<Option<&T>>) {
        if let Ok(v) = v {
            if let Some(val) = v {
                let mut added_to_existing = false;
                for (stored_val, totals) in self.items.iter_mut() {
                    if val == stored_val {
                        *totals += 1;
                        added_to_existing = true;
                        break;
                    }
                }
                if added_to_existing {
                    self.items.push((val.clone(), 1));
                }
            } else {
                self.nones += 1;
            }
        } else {
            self.errors += 1;
        }
    }

    fn combine(&mut self, other: SmallCatSummary<T>) {
        for (val, count) in other.items.iter() {
            let mut added_to_existing = false;
            for (stored_val, totals) in self.items.iter_mut() {
                if val == stored_val {
                    *totals += count;
                    added_to_existing = true;
                    break;
                }
            }
            if !added_to_existing {
                self.items.push((val.clone(), *count));
            }
        }
    }

    fn count(&self) -> usize {
        self.items.iter().map(|(_a,b)| b).fold(0,|a,c| a + c)
    }

    fn nones(&self) -> usize {
        self.nones
    }

    fn errors(&self) -> usize {
        self.nones
    }
}

/// Summary of vectors
#[derive(Clone, Debug, Default)]
pub struct VecSummary {
    // First moment is stored in the first half, the second in the second
    moment1: Vec<f32>,
    moment2: Vec<f32>,
    count: usize,
    nones: usize,
    errors: usize,
}

impl Summary<[f32]> for VecSummary {
    fn add(&mut self, v: PointCloudResult<Option<&[f32]>>) {
        if let Ok(vv) = v {
            if let Some(val) = vv {
                if !self.moment1.is_empty() {
                    if self.moment1.len() == val.len() {
                        self.moment1
                            .iter_mut()
                            .zip(val)
                            .for_each(|(m, x)| *m += x);
                        self.moment2
                            .iter_mut()
                            .zip(val)
                            .for_each(|(m, x)| *m += x * x);
                        self.count += 1;
                    } else {
                        self.errors += 1;
                    }
                } else {
                    self.moment1.extend(val);
                    self.moment2.extend(val.iter().map(|x| x * x))
                }
            } else {
                self.nones += 1;
            }
        } else {
            self.errors += 1;
        }
    }
    fn combine(&mut self, other: VecSummary) {
        self.moment1
            .iter_mut()
            .zip(other.moment1)
            .for_each(|(x, y)| *x += y);
        self.moment2
            .iter_mut()
            .zip(other.moment2)
            .for_each(|(x, y)| *x += y);
        self.count += other.count;
        self.nones += other.nones;
        self.errors += other.errors;
    }

    fn count(&self) -> usize {
        self.count
    }

    fn nones(&self) -> usize {
        self.nones
    }

    fn errors(&self) -> usize {
        self.nones
    }
}

/// A summary for a small number of categories.
#[derive(Clone, Debug)]
pub struct StringSummary {
    items: HashMap<String,usize>,
    nones: usize,
    errors: usize,
}

impl Default for StringSummary {
    fn default() -> Self {
        StringSummary {
            items: HashMap::new(),
            nones: 0,
            errors: 0,
        }
    }
}

impl Summary<String> for StringSummary {
    fn add(&mut self, v: PointCloudResult<Option<&String>>) {
        if let Ok(v) = v {
            if let Some(val) = v {
                *self.items.entry(val.to_string()).or_insert(0) += 1;
            } else {
                self.nones += 1;
            }
        } else {
            self.errors += 1;
        }
    }

    fn combine(&mut self, other: StringSummary) {
        self.nones += other.nones;
        self.errors += other.errors;
        for (val, count) in other.items.iter() {
            *self.items.entry(val.to_string()).or_insert(0) += count;
        }
    }

    fn count(&self) -> usize {
        self.items.values().fold(0,|a,c| a + c)
    }

    fn nones(&self) -> usize {
        self.nones
    }

    fn errors(&self) -> usize {
        self.nones
    }
}
