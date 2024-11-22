use clap::Parser;
use normalize_path::NormalizePath;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use slugify::slugify;

#[derive(Parser, Debug)]
struct Commands {
  /// The directory to search within
  scan_dir: Option<PathBuf>,

  #[arg(short = 'y', default_value_t = false)]
  force: bool,
}

fn main() -> anyhow::Result<()> {
  let cmd = Commands::parse();
  let scan_dir: PathBuf;
  if let Some(target) = cmd.scan_dir {
    if target.is_relative() {
      scan_dir = std::env::current_dir()?
        .join(target)
        .normalize();
    } else {
      scan_dir = target;
    }
  } else {
    scan_dir = std::env::current_dir()?
        .normalize();
  }

  println!("Config:");
  println!("  Scanning: {}/*", scan_dir.to_str().unwrap());
  println!("  Move to:  {}", scan_dir.to_str().unwrap());

  let mut rename = HashMap::<PathBuf, PathBuf>::new();
  let mut delete = HashSet::<PathBuf>::new();
  let mut create = HashSet::<PathBuf>::new();

  println!("");

  for entry in fs::read_dir(&scan_dir)? {
    let entry_path = entry?.path();

    let file_ext = if let Some(ext) = entry_path.extension() {
      format!(".{}", ext.to_str().unwrap().to_string())
    } else {
      Default::default()
    };
    let file_name_slug = format!("{}{}", slugify!(entry_path.file_stem().unwrap().to_str().unwrap()), file_ext);

    if entry_path == scan_dir {
      continue;
    };

    let target = scan_dir.join(&file_name_slug);
    
    if entry_path.is_dir() {
      create.insert(target.clone());
      if !create.contains(&entry_path) {
        delete.insert(entry_path.clone());
      }

      for inner in fs::read_dir(&entry_path)? {
        let inner_src = inner?.path();
        let mut inner_dest = target.join(inner_src.file_name().unwrap());

        while rename.contains_key(&inner_dest) {
          let file_stem = inner_dest.file_stem().unwrap().to_str().unwrap();
          let file_ext = if let Some(ext) = inner_dest.extension() {
            format!(".{}", ext.to_str().unwrap().to_string())
          } else {
            Default::default()
          };
          inner_dest = target.join(format!("{}_{}",file_stem, file_ext))
        }

        rename.insert(inner_dest.clone(), inner_src.clone());

        let src = pathdiff::diff_paths(
          &inner_src, 
          &scan_dir
        ).unwrap();

        let dest = pathdiff::diff_paths(
          &inner_dest, 
          &scan_dir,
        ).unwrap();
        println!("  From: {}\n  To:   {}\n", src.to_str().unwrap(), dest.to_str().unwrap());
      }
    } else {
      let mut dest = target;

      loop {
        if !rename.contains_key(&dest) && !dest.exists() && !create.contains(&dest) {
          break
        }

        let file_stem = dest.file_stem().unwrap().to_str().unwrap();
        let file_ext = if let Some(ext) = dest.extension() {
          format!(".{}", ext.to_str().unwrap().to_string())
        } else {
          Default::default()
        };
        dest = scan_dir.join(format!("{}_{}",file_stem, file_ext))
      }
      
      rename.insert(dest.clone(), entry_path.clone());

      let src = pathdiff::diff_paths(
        &entry_path, 
        &scan_dir
      ).unwrap();

      let dest = pathdiff::diff_paths(
        &dest, 
        &scan_dir,
      ).unwrap();

      println!("  From: {}\n  To:   {}\n", src.to_str().unwrap(), dest.to_str().unwrap());
    }
  }

  // dbg!(&create);
  // dbg!(&rename);
  // dbg!(&delete);

  if create.is_empty() && rename.is_empty() && delete.is_empty() {
    println!("Nothing to do");    
    return Ok(());
  }

  if !cmd.force {
    println!("");
    print!("Continue? [y/N] ");
    let mut line = String::new();
    let _ = std::io::stdout().flush();
    std::io::stdin().read_line(&mut line).unwrap();
    line = line.trim().to_string();

    if line != "y" && line != "Y" {
      println!("Nothing changed");
      return Ok(());
    }
  } else {
    println!("");
  }

  for dir in create.iter() {
    println!("  Create: {}", dir.to_str().unwrap());
    if !dir.exists() {
      fs::create_dir_all(&dir)?;
    }
  }

  for (to, from) in rename.iter() {
    println!("  Move:   {}", to.to_str().unwrap());
    fs::rename(from, to)?;
  }

  for dir in delete.iter() {
    println!("  Delete: {}", dir.to_str().unwrap());
    fs::remove_dir_all(dir)?;
  }

  return Ok(())
}
