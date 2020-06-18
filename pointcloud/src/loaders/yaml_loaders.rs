use glob::{glob_with, MatchOptions};
use std::fs;
use yaml_rust::YamlLoader;

use crate::{DefaultLabeledCloud,DefaultCloud};

use super::*;

/// Given a yaml file on disk, it builds a point cloud. Minimal example below.
/// ```yaml
/// ---
/// data_path: DATAMEMMAP
/// labels_path: LABELS_CSV_OR_MEMMAP
/// count: NUMBER_OF_DATA_POINTS
/// data_dim: 784
/// label_csv_index: 2
/// ```
pub fn labeled_ram_from_yaml<P: AsRef<Path>, M: Metric>(
    path: P,
) -> PointCloudResult<DefaultLabeledCloud<M>> {
    let config = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Unable to read config file {:?}", &path.as_ref()));

    let params_files = &YamlLoader::load_from_str(&config).unwrap()[0];

    let data_paths = &get_file_list(
        params_files["data_path"]
            .as_str()
            .expect("Unable to read the 'data_path'"),
        path.as_ref(),
    );
    let labels_path = &get_file_list(
        params_files["labels_path"]
            .as_str()
            .expect("Unable to read the 'labels_path'"),
        path.as_ref(),
    );

    let data_dim = params_files["data_dim"]
        .as_i64()
        .expect("Unable to read the 'data_dim'") as usize;

    let labels_index = params_files["labels_index"]
        .as_i64()
        .expect("Unable to read the 'labels_index'") as usize;

    let mut label_set: Vec<SmallIntLabels> = labels_path.iter().map(|path| open_int_csv(path, labels_index)).collect::<PointCloudResult<Vec<SmallIntLabels>>>()?;
    let data_set = convert_glued_memmap_to_ram(open_memmaps(data_dim, data_paths)?);

    Ok(SimpleLabeledCloud::new(
        data_set,
        label_set.drain(0..).fold_first(|mut a,b| {a.merge(&b); a}).unwrap(),
    ))
}

/// Given a yaml file on disk, it builds a point cloud. Minimal example below.
/// ```yaml
/// ---
/// data_path: DATAMEMMAP
/// count: NUMBER_OF_DATA_POINTS
/// data_dim: 784
/// ```
pub fn ram_from_yaml<P: AsRef<Path>, M: Metric>(
    path: P,
) -> PointCloudResult<DefaultCloud<M>> {
    let config = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Unable to read config file {:?}", &path.as_ref()));

    let params_files = &YamlLoader::load_from_str(&config).unwrap()[0];

    let data_paths = &get_file_list(
        params_files["data_path"]
            .as_str()
            .expect("Unable to read the 'data_path'"),
        path.as_ref(),
    );

    let data_dim = params_files["data_dim"]
        .as_i64()
        .expect("Unable to read the 'data_dim'") as usize;

    let data_set = open_memmaps(data_dim, data_paths)?;
    Ok(convert_glued_memmap_to_ram(data_set))
}

fn get_file_list(files_reg: &str, yaml_path: &Path) -> Vec<PathBuf> {
    let options = MatchOptions {
        case_sensitive: false,
        ..Default::default()
    };
    let mut paths = Vec::new();
    let glob_paths;
    let files_reg_path = Path::new(files_reg);
    if files_reg_path.is_absolute() {
        glob_paths = match glob_with(&files_reg_path.to_str().unwrap(), options) {
            Ok(expr) => expr,
            Err(e) => panic!("Pattern reading error {:?}", e),
        };
    } else {
        glob_paths = match glob_with(
            &yaml_path
                .parent()
                .unwrap()
                .join(files_reg_path)
                .to_str()
                .unwrap(),
            options,
        ) {
            Ok(expr) => expr,
            Err(e) => panic!("Pattern reading error {:?}", e),
        };
    }

    for entry in glob_paths {
        let path = match entry {
            Ok(expr) => expr,
            Err(e) => panic!("Error reading path {:?}", e),
        };
        paths.push(path)
    }
    paths
}
