
use std::collections::hash_map::*;

use std::io::{self, BufRead,BufReader};
use std::io::prelude::*;
use regex::Regex;
use std::path::PathBuf;
use std::process::{Command};

#[derive(Debug,Default)]
pub struct Shared_lib{
    pub path:String,
    pub dependency:Vec<String>
}

pub fn get_lib_ldd(lib_to_package_map:&HashMap<String,String>,ldd_flag:&str,is_debug_mode:bool) -> Vec<Shared_lib>
{
    let mut shared_libs:Vec<Shared_lib> = vec!();
    
    for (key,_) in lib_to_package_map.into_iter() {

        let input_so_file_path = key.clone();


        let mut args:Vec<&str> = vec!();

        if ldd_flag.len() > 0
        {
            args.push(ldd_flag);
            if is_debug_mode
            {
                println!("ldd {} {}",ldd_flag,input_so_file_path);
            }
        }
        else
        {
            if is_debug_mode
            {
                println!("ldd {}",input_so_file_path);
            }
        }

        args.push(&input_so_file_path);

        let command = Command::new("ldd")
        .args(args)
        .output()
        .expect("failed to execute process");


        let mut dependency:Vec<String> = vec!();

        match String::from_utf8(command.stdout)
        {
            Ok(content) => {

                for line in content.lines() {

                    if is_match(r".+direct .+",&line)
                    {
                        continue;
                    }
        
                    if is_match(r".+statically .+",&line)
                    {
                        continue;
                    }

                    let mut so_file_path_op:Option<String> = get_so_path(r".+ (?P<soname>.+) \(","soname",&line);

                    if so_file_path_op == None
                    {
                        so_file_path_op = get_so_path(r"\t(?P<soname>.+) \(","soname",&line);
                    }
        
                    if so_file_path_op == None
                    {
                        so_file_path_op = get_so_path(r"\t(?P<soname>.+)","soname",&line);
                    }

                    if so_file_path_op == None
                    {
                        so_file_path_op = get_so_path(r".+linux(?P<soname>.+) \(","soname",&line);
                        match so_file_path_op
                        {
                            Some(so_file_path) => so_file_path_op = Some(format!("linux{}",so_file_path)),
                            None => (),
                        }
                    }
                    

                    match so_file_path_op
                    {
                        Some(so_file_path) => {
                            if is_debug_mode
                            {
                                println!("  ->{}",so_file_path);
                            }
                            match dependency.binary_search_by(|x| x.cmp(&so_file_path))
                            {
                                Ok(idx) => {},
                                Err(idx) => {
                                    dependency.insert(idx,so_file_path);
                                },
                            }
                        },
                        None => {
                            if is_debug_mode
                            {
                                println!("  ->Could not parse :{}",line);
                            }
                        }
                    }
        
                }
            },
            Err(err) => 
            {            
                if is_debug_mode
                {
                    println!("ldd err:{:?}",err);
                }
            },
        }

        match shared_libs.binary_search_by(|x| x.path.cmp(&input_so_file_path))
        {
            Ok(_) => {},
            Err(idx) => {
                shared_libs.insert(idx,
                    Shared_lib
                    {
                        path:input_so_file_path,
                        dependency:dependency
                    }
                );
            },
        }
    }

    return shared_libs;
}

fn get_so_path(regptn:&str,capname:&str,target:&str) -> Option<String>
{
    let re = Regex::new(regptn).unwrap();
    let soname_op = re.captures(target).and_then(|cap|{
        cap.name(capname).map(|val| val.as_str())
    });

    match soname_op
    {
        Some(soname) => {
            Some(soname.to_string())
        },
        None => {
            None
            //return;
        }
    }
}

fn is_match(regptn:&str,target:&str) -> bool
{
    let re = Regex::new(regptn).unwrap();

    re.is_match(target)
}
