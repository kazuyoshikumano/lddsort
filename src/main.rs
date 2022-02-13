
use std::collections::hash_map::*;
use std::fs;
use std::fs::File;
use std::path;

use std::path::Path;
use std::path::PathBuf;

use std::process::{Command};

extern crate clap;

use clap::{App, Arg};

mod ldd;
mod pak;

use ldd::Shared_lib;

fn main() {
    let target_pkg_path_argname = "target_pkg_path";
    let debug_argname = "debug_mode";

    let app = App::new("lddsort")
        .version("0.1.0")
        .author("kumanokazuyoshi <kazuyoshi.kumano@outlook.jp>")
        .about("CLI to sort Shared Libraries(lib***.so) according to their depedencies (provided by ldd command)")
        .arg(Arg::with_name(target_pkg_path_argname)
            .help("Path to shared libraries")
            .multiple_occurrences(true)
            .required(true)
        )
        .arg(Arg::with_name(debug_argname)
            .help("Display detail log")
            .short('d')
        );
    
    let matches = app.get_matches();

    let is_debug_mode = matches.is_present(debug_argname);

    let mut include_dir_pathbuf_list:Vec<PathBuf> = vec!();

    if let Some(include_matches) = matches.values_of(target_pkg_path_argname) {

        let includes: Vec<&str> = include_matches.collect();

        for include in includes
        {
            let raw_include_dir = PathBuf::from(include);
            
            if !raw_include_dir.exists()
            {
                println!("No such directory exists:{}",include);
                return;
            }

            match fs::canonicalize(&raw_include_dir)
            {
                Ok(include_dir) => {
                    if is_debug_mode
                    {
                        println!("searching shared libraries in {:?}",include_dir);
                    }
                    include_dir_pathbuf_list.push(include_dir);

                },
                Err(e) => println!("{:?}", e),
            }
        }
    }else
    {
        return;
    }

    let lib_to_package_map:HashMap<String,String> = pak::get_pak_map(include_dir_pathbuf_list);

    if lib_to_package_map.len() == 0
    {
        println!("No shared library found.");
        return;
    }
    
    if is_debug_mode
    {
        println!("getting dependency...");
    }

    let mut shared_libs:Vec<Shared_lib>        = ldd::get_lib_ldd(&lib_to_package_map,"",is_debug_mode);
    let unused_shared_libs:Vec<Shared_lib>     = ldd::get_lib_ldd(&lib_to_package_map,"-u",is_debug_mode);

    if is_debug_mode
    {
        println!("");
        println!("lib_to_package_map:{:?}",lib_to_package_map);
        println!("shared_libs:{:?}",shared_libs);
        println!("unused_shared_libs:{:?}",unused_shared_libs);
    
        println!("");
        println!("resolving...");
    }

    let mut resolved_so_map:HashMap<String,usize> = Default::default();
    let mut pre_count:usize = 0;
    let mut try_count = 0;
    loop
    {
        for i in 0..shared_libs.len()
        {
            //println!("resolving_lib : {}",shared_libs[i].name);
            let has_dependency_resolved = resolve(&shared_libs[i],&lib_to_package_map,&mut resolved_so_map,&unused_shared_libs);

            if has_dependency_resolved
            {
                shared_libs.remove(i);
                //println!("resolved_lib : {}",resolved_lib.name);
                break;
            }
        }

        if is_debug_mode
        {
            println!(" {0:>5} | resolved {1:>5} | remained {2:>5} ",try_count, resolved_so_map.iter().count(),shared_libs.len());
        }
        if shared_libs.len() == 0
        {
            break;
        }
        else
        {
            if pre_count == shared_libs.len()
            {
                break;
            }
            pre_count = shared_libs.len();
        }

        try_count = try_count + 1;
    }

    if is_debug_mode
    {
        println!("");
        println!("Check unsolved libs...");
        for i in 0..shared_libs.len()
        {
            println!("remained : {} {}",shared_libs[i].path,shared_libs[i].dependency.len());
            let insight = insight(&shared_libs[i],&lib_to_package_map,&mut resolved_so_map,&unused_shared_libs);
            let unused_deps = insight.0;
            let resolved_deps = insight.1;
            let not_in_package = insight.2;
            let remain_deps = insight.3;
            println!("  unused_deps    {0:>5} | : {1:?}",unused_deps.len(),unused_deps);
            println!("  resolved_deps  {0:>5} | : {1:?}",resolved_deps.len(),resolved_deps);
            println!("  not_in_package {0:>5} | : {1:?}",not_in_package.len(),not_in_package);
            println!("  remained_deps  {0:>5} | : {1:?}",remain_deps.len(),remain_deps);
        }
    }
    

    let mut removed_circle_dependency_map:HashMap<String,Vec<String>> = Default::default();

    if shared_libs.len() > 0
    {
        if is_debug_mode
        {
            println!("");
            println!("dependency remained. solving circle dependendy...");
        }

        for i in 0..shared_libs.len()
        {
            get_removing_circle_dependency(i,&mut shared_libs,&mut removed_circle_dependency_map);
        }

        for i in 0..shared_libs.len()
        {
            remove_circle_dependency(i,&mut shared_libs);
        }
        
        loop
        {
            for i in 0..shared_libs.len()
            {
                //println!("resolving_lib : {}",shared_libs[i].name);
                let has_dependency_resolved = resolve(&shared_libs[i],&lib_to_package_map,&mut resolved_so_map,&unused_shared_libs);
    
                if has_dependency_resolved
                {
                    shared_libs.remove(i);
                    //println!("resolved_lib : {}",resolved_lib.name);
                    break;
                }
            }
    
            if is_debug_mode
            {
                println!(" {0:>5} | resolved {1:>5} | remained {2:>5} ",try_count, resolved_so_map.iter().count(),shared_libs.len());
            }

            if shared_libs.len() == 0
            {
                break;
            }
            else
            {
                if pre_count == shared_libs.len()
                {
                    break;
                }
                pre_count = shared_libs.len();
            }
            try_count = try_count + 1;
        }

        if is_debug_mode
        {
            println!("");
            println!("Check unsolved libs...");
            
            for i in 0..shared_libs.len()
            {
                println!("remained : {} {}",shared_libs[i].path,shared_libs[i].dependency.len());
                let insight = insight(&shared_libs[i],&lib_to_package_map,&mut resolved_so_map,&unused_shared_libs);
                let unused_deps = insight.0;
                let resolved_deps = insight.1;
                let not_in_package = insight.2;
                let remain_deps = insight.3;
                println!("  unused_deps    {0:>5} | : {1:?}",unused_deps.len(),unused_deps);
                println!("  resolved_deps  {0:>5} | : {1:?}",resolved_deps.len(),resolved_deps);
                println!("  not_in_package {0:>5} | : {1:?}",not_in_package.len(),not_in_package);
                println!("  remained_deps  {0:>5} | : {1:?}",remain_deps.len(),remain_deps);
            }
        }
    }

    let mut resolved_so_list : Vec<(String,usize)> = resolved_so_map.into_iter().map(|(key,val)| (key,val)).collect();

    resolved_so_list.sort_by_key(|e|e.1);

    let title_num   = "order";
    let title_name  = "name";
    let title_lib   = "package";
    let title_dep   = "circular ref";
    let title_path  = "path";

    let mut max_len_num     = title_num.len();
    let mut max_len_name    = title_name.len();
    let mut max_len_lib     = title_lib.len();
    let mut max_len_dep     = title_dep.len();
    let mut max_len_path    = title_path.len();

    let mut removed_name_map:HashMap<String,String> = Default::default();
    let mut pak_name_map:HashMap<String,String> = Default::default();
    let mut dependency_map:HashMap<String,String> = Default::default();

    for resolved_so in &resolved_so_list
    {
        if resolved_so.1.to_string().len() > max_len_num
        {
            max_len_num = resolved_so.1.to_string().len();
        }

        if resolved_so.0.len() > max_len_path
        {
            max_len_path = resolved_so.0.len();
        }

        let name:String = match PathBuf::from(&resolved_so.0).file_name()
        {
            Some(fname) => fname.to_str().unwrap().to_string(),
            None        => resolved_so.0.clone(),
        };

        if name.len() > max_len_name
        {
            max_len_name = name.len();
        }

        match lib_to_package_map.get(&resolved_so.0)
        {
            Some(pak_path) => {
                let pak_name = match PathBuf::from(&pak_path).file_name()
                {
                     Some(fname) => fname.to_str().unwrap().to_string(),
                     None => pak_path.clone(),
                };
        
                if pak_name.len() > max_len_lib
                {
                    max_len_lib = pak_name.len();
                }

                pak_name_map.insert(resolved_so.0.clone(), pak_name);
            },
            None          => (),
        }

        match removed_circle_dependency_map.get(&resolved_so.0)
        {
            Some(dependency) => {
                let mut dep_str = match dependency.first()
                {
                    Some(fst) => {
                        let dep_name:String = match PathBuf::from(&fst).file_name()
                        {
                            Some(fname) => fname.to_str().unwrap().to_string(),
                            None        => fst.clone(),
                        };
                        format!("{}",dep_name)
                    },
                    None => continue,
                };
                
                for i in 1..dependency.len()
                {
                    let dep_name:String = match PathBuf::from(&dependency[i]).file_name()
                    {
                        Some(fname) => fname.to_str().unwrap().to_string(),
                        None        => dependency[i].clone(),
                    };

                    dep_str = format!("{},{}",dep_str,dep_name);
                }

                if dep_str.len() > max_len_dep
                {
                    max_len_dep = dep_str.len();
                }
                
                dependency_map.insert(resolved_so.0.clone(), dep_str);

            },
            None          => (),
        }

        removed_name_map.insert(resolved_so.0.clone(), name);

    }

    if is_debug_mode
    {
        println!("");
        println!("result:");
    }
    
    if dependency_map.len() > 0
    {
        println!(" {0:>max_len_num$}   {1:<max_len_name$}   {2:<max_len_lib$}   {3:<max_len_dep$}   {4:<max_len_path$}", title_num,title_name,title_lib,title_dep,title_path,max_len_num=max_len_num,max_len_name=max_len_name,max_len_lib=max_len_lib,max_len_dep=max_len_dep,max_len_path=max_len_path);
    }
    else
    {
        println!(" {0:>max_len_num$}   {1:<max_len_name$}   {2:<max_len_lib$}   {3:<max_len_path$}", title_num,title_name,title_lib,title_path,max_len_num=max_len_num,max_len_name=max_len_name,max_len_lib=max_len_lib,max_len_path=max_len_path);
    }

    for resolved_so in &resolved_so_list
    {
        let order = &resolved_so.1;
        let path = &resolved_so.0;


        let name = match removed_name_map.get(&resolved_so.0)
        {
            Some(val) => val.clone(),
            None      => Default::default(),
        };

        let pak_name = match pak_name_map.get(&resolved_so.0)
        {
            Some(val) => val.clone(),
            None      => Default::default(),
        };

        let dependency = match dependency_map.get(&resolved_so.0)
        {
            Some(val) => val.clone(),
            None      => Default::default(),
        };

        if dependency_map.len() > 0
        {
            println!(" {0:>max_len_num$} | {1:<max_len_name$} | {2:<max_len_lib$} | {3:<max_len_dep$} | {4:<max_len_path$}", order,name,pak_name,dependency,path,max_len_num=max_len_num,max_len_name=max_len_name,max_len_lib=max_len_lib,max_len_dep=max_len_dep,max_len_path=max_len_path);
        }
        else
        {
            println!(" {0:>max_len_num$} | {1:<max_len_name$} | {2:<max_len_lib$} | {3:<max_len_path$}", order,name,pak_name,path,max_len_num=max_len_num,max_len_name=max_len_name,max_len_lib=max_len_lib,max_len_path=max_len_path);
        }
    
    }

}


fn resolve(target_lib:&Shared_lib,lib_to_package_map:&HashMap<String,String>,resolved_so_map:&mut HashMap<String,usize>,unused_shared_libs:&Vec<Shared_lib>) -> bool
{
    let mut has_dependency_resolved = true;
    for dep in &target_lib.dependency
    {

        if is_unused_dependency(&target_lib.path,&dep,&unused_shared_libs)
        {
            continue;
        }
        
        if resolved_so_map.contains_key(dep)
        {
            continue;
        }

        if lib_to_package_map.contains_key(dep)
        {
            if !resolved_so_map.contains_key(dep)
            {
                has_dependency_resolved = false;
            }
        }
        else
        {
            resolved_so_map.insert(dep.clone(),0);
        }
    }

    //println!("resolve has_dependency_resolved:{}",has_dependency_resolved);

    if has_dependency_resolved
    {
        resolved_so_map.insert(target_lib.path.clone(),resolved_so_map.iter().count() + 1);
    }

    return has_dependency_resolved;
}


fn remove_circle_dependency(target_lib_idx:usize,shared_libs:&mut Vec<Shared_lib>) 
{

    let mut i=0;
    while i < shared_libs[target_lib_idx].dependency.len()
    {
        match shared_libs.binary_search_by(|x| x.path.cmp(&shared_libs[target_lib_idx].dependency[i]))
        {
            Ok(idx) => {

                match shared_libs[idx].dependency.binary_search_by(|x| x.cmp(&shared_libs[target_lib_idx].path))
                {
                    Ok(_) => {
                        shared_libs[target_lib_idx].dependency.remove(i);
                        continue;
                    },
                    Err(_) => {
                    },
                }
            },
            Err(idx) => {
            },
        }

        i = i + 1;
    }

}

fn get_removing_circle_dependency(target_lib_idx:usize,shared_libs:&mut Vec<Shared_lib>,removed_circle_dependency_map:&mut HashMap<String,Vec<String>>) 
{

    for i in 0..shared_libs[target_lib_idx].dependency.len()
    {
        match shared_libs.binary_search_by(|x| x.path.cmp(&shared_libs[target_lib_idx].dependency[i]))
        {
            Ok(idx) => {

                match shared_libs[idx].dependency.binary_search_by(|x| x.cmp(&shared_libs[target_lib_idx].path))
                {
                    Ok(_) => {
                        let removing_dep = shared_libs[target_lib_idx].dependency[i].clone();
                        match removed_circle_dependency_map.get_mut(&shared_libs[target_lib_idx].path)
                        {
                            Some(map) => {
                                map.push(removing_dep);
                            }
                            None => {
                                removed_circle_dependency_map.insert(
                                    shared_libs[target_lib_idx].path.clone(), 
                                    vec!(removing_dep)
                                );
                            }
                        }
                        continue;
                    },
                    Err(_) => {
                    },
                }
            },
            Err(idx) => {
            },
        }
    }

}


fn insight(target_lib:&Shared_lib,lib_to_package_map:&HashMap<String,String>,resolved_so_map:&mut HashMap<String,usize>,unused_shared_libs:&Vec<Shared_lib>)
 -> (Vec<String>,Vec<String>,Vec<String>,Vec<String>)
{
    let mut unused_deps:Vec<String> = vec!();
    let mut resolved_deps:Vec<String> = vec!();
    let mut not_in_package:Vec<String> = vec!();
    let mut remain_deps:Vec<String> = vec!();

    for dep in &target_lib.dependency
    {
        //println!("resolve dep:{}",dep.as_str());
        if is_unused_dependency(&target_lib.path,&dep,&unused_shared_libs)
        {
            unused_deps.push(dep.clone());
            continue;
        }
        
        if !lib_to_package_map.contains_key(dep)
        {
            not_in_package.push(dep.clone());
            continue;
        }
        
        if resolved_so_map.contains_key(dep)
        {
            resolved_deps.push(dep.clone());
            continue;
        }

        remain_deps.push(dep.clone());

    }

    return (
        unused_deps,
        resolved_deps,
        not_in_package,
        remain_deps
    );
}


fn is_unused_dependency(target_lib_name:&String,depend_lib_name:&String,unused_shared_libs:&Vec<Shared_lib>)
-> bool
{

    match unused_shared_libs.binary_search_by(|x| x.path.cmp(target_lib_name))
    {
        Ok(idx) => {
            match unused_shared_libs[idx].dependency.binary_search_by(|x| x.cmp(depend_lib_name))
            {
                Ok(_) => {
                    return true;
                },
                Err(_) => {
                    return false;
                },
            }
        },
        Err(idx) => {
            return false;
        },
    }


}