
use std::collections::hash_map::*;

use std::path::Path;
use std::path::PathBuf;
use glob::glob;

pub fn get_pak_map(include_dir_pathbuf_list:Vec<PathBuf>) -> HashMap<String,String> 
{
    let mut lib_to_package_map:HashMap<String,String> = HashMap::new() ;
    
    for include_dir_pathbuf in include_dir_pathbuf_list
    {
        let mut shared_libs:Vec<String> = vec!();

        let glob_ptn = format!("{}/**/*.so*",include_dir_pathbuf.display());
        //println!("glob_ptn:{}",glob_ptn);
        for entry in glob(&glob_ptn).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    shared_libs.push(path.display().to_string());
                },
                Err(e) => println!("{:?}", e),
            }
        }

        let pak_path = include_dir_pathbuf.display().to_string();

        match lib_to_package_map.get(&pak_path)
        {
            Some(pak) => {

            },
            None => {
                for lib in shared_libs
                {
                    lib_to_package_map.insert(lib.clone(),pak_path.clone());
                }
            }
        }
    }

    return lib_to_package_map;
}