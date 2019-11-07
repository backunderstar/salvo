use std::path::{PathBuf, Path};
use std::fs::{self, Metadata};
use std::time::SystemTime;
use std::collections::HashMap;
use chrono::prelude::*;
use serde_json::json;
use mime;

use novel::{Context, Handler};

#[derive(Debug, Clone)]
pub struct Options {
    pub dot_files: bool,
    pub listing: bool,
    pub defaults: Vec<String>,
}

impl Options{
    fn new()->Options{
        Options{
            dot_files: true,
            listing: true,
            defaults: vec!["index.html".to_owned()],
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Options::new()
    }
}

#[derive(Clone)]
pub struct Static {
    roots: Vec<PathBuf>,
    options: Options,
}

pub trait StaticRoots {
    fn collect(&self)->Vec<PathBuf>;
}

impl<'a> StaticRoots for &'a str {
    fn collect(&self)->Vec<PathBuf>{
        vec![PathBuf::from(self)]
    }
}
impl<'a> StaticRoots for Vec<&'a str> {
    fn collect(&self)->Vec<PathBuf>{
        self.iter().map(|i|{PathBuf::from(i)}).collect()
    }
}
impl StaticRoots for Path {
    fn collect(&self)->Vec<PathBuf>{
        vec![PathBuf::from(self)]
    }
}

impl Static {
    pub fn from<T:StaticRoots+Sized>(roots: T) -> Self {
        Static::new(roots, Options::default())
    }

    pub fn new<T:StaticRoots+Sized>(roots: T, options: Options) -> Self {
        Static { roots: roots.collect(), options }
    }
}

fn list_json(root: &BaseInfo)->String{
    json!(root).to_string()
}
fn list_xml(root: &BaseInfo)->String{
    let mut ftxt = "<list>".to_owned();
    if root.dirs.len() == 0 && root.files.len() == 0 {
        ftxt.push_str("No files");
    }else{
        ftxt.push_str("<table>");
        for dir in &root.dirs {
            ftxt.push_str(&format!("<dir><name>{}</name><modified>{}</modified></dir>", dir.name, dir.modified.format("%Y-%m-%d %H:%M:%S")));
        }
        for file in &root.files {
            ftxt.push_str(&format!("<file><name>{}</name><modified>{}</modified><size>{}</size></file>", file.name, file.modified.format("%Y-%m-%d %H:%M:%S"), file.size));
        }
        ftxt.push_str("</table>");
    }
    ftxt.push_str("</list>");
    ftxt
}
fn list_html(root: &BaseInfo)->String{
    let mut ftxt = format!("<!DOCTYPE html>
<html>
    <head>
        <meta charset=\"utf-8\">
        <title>{}</title>
    </head>
    <body>
        <h1>Index of: {}</h1>
        <hr/>
        <a href=\"../\">[../]</a><br><br>
", root.path, root.path);
    if root.dirs.len() == 0 && root.files.len() == 0 {
        ftxt.push_str("No files");
    }else{
        ftxt.push_str("<table>");
        for dir in &root.dirs {
            ftxt.push_str(&format!("<tr><td><a href=\"./{}/\">{}/</a></td><td>{}</td><td></td></tr>", dir.name, dir.name, dir.modified.format("%Y-%m-%d %H:%M:%S")));
        }
        for file in &root.files {
            ftxt.push_str(&format!("<tr><td><a href=\"./{}\">{}</a></td><td>{}</td><td>{}</td></tr>", file.name, file.name, file.modified.format("%Y-%m-%d %H:%M:%S"), file.size));
        }
        ftxt.push_str("</table>");
    }
    ftxt.push_str("<hr/><div style=\"text-align:center;\"><small>novel</small></div></body>");
    ftxt
}
fn list_text(root: &BaseInfo)->String{
   json!(root).to_string()
}
#[derive(Serialize, Deserialize)]
struct BaseInfo{
    path: String,
    files: Vec<FileInfo>,
    dirs: Vec<DirInfo>,
}
impl BaseInfo{
    fn new(path: String, files: Vec<FileInfo>, dirs: Vec<DirInfo>)->BaseInfo {
        BaseInfo{
            path: path,
            files: files,
            dirs: dirs,
        }
    }
}
#[derive(Serialize, Deserialize)]
struct FileInfo {
    name: String,
    size: u64,
    modified: DateTime<Local>,
}
impl FileInfo {
    fn new(name: String, metadata: Metadata)->FileInfo {
        FileInfo{
            name: name,
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(SystemTime::now()).into(),
        }
    }
}
#[derive(Serialize, Deserialize)]
struct DirInfo{
    name: String,
    modified: DateTime<Local>,
}
impl DirInfo{
    fn new(name: String, metadata: Metadata)->DirInfo {
        DirInfo{
            name: name,
            modified: metadata.modified().unwrap_or(SystemTime::now()).into(),
        }
    }
}

impl Handler for Static {
    fn handle(&self, ctx: &mut Context) {
        let param = ctx.params().iter().find(|(key, _)|key.starts_with("*"));
        let base_path = if let Some((_, value)) = param {
            value
        } else{
            ctx.request().url().path()
        };
        let mut files: HashMap<String, Metadata> = HashMap::new();
        let mut dirs: HashMap<String, Metadata> = HashMap::new();
        let mut path_exist = false;
        for root in &self.roots {
            let path = root.join(&base_path);
            if path.is_dir() && self.options.listing{
                path_exist = true;
                if !ctx.request().url().path().ends_with('/') {
                    ctx.redirect_found(format!("{}/", ctx.request().url().path()));
                    return
                }
                for ifile in &self.options.defaults {
                    let ipath = path.join(ifile);
                    if ipath.exists() {
                        ctx.render_file_from_path(ipath).ok();
                        return;
                    }
                }
                //list the dir
                if let Ok(entries) = fs::read_dir(&path){
                    for entry in entries {
                        if let Ok(entry) = entry {
                            if let Ok(metadata) = entry.metadata() {
                                if metadata.is_dir() {
                                    dirs.entry(entry.file_name().into_string().unwrap_or("".to_owned())).or_insert(metadata);
                                }else{
                                    files.entry(entry.file_name().into_string().unwrap_or("".to_owned())).or_insert(metadata);
                                }
                            }
                        }
                    }
                }
            } else if path.is_file() {
                ctx.render_file_from_path(path).ok();
                return
            }
        }
        if !path_exist || !self.options.listing{
            ctx.not_found();
            return;
        }
        let mut format = ctx.request().frist_accept().unwrap_or(mime::TEXT_HTML);
        if format.type_() != "text" {
            format = mime::TEXT_HTML;
        }
        let mut files: Vec<FileInfo> = files.into_iter().map(|(name, metadata)|FileInfo::new(name, metadata)).collect();
        files.sort_by(|a,b|a.name.cmp(&b.name));
        let mut dirs: Vec<DirInfo> = dirs.into_iter().map(|(name, metadata)|DirInfo::new(name, metadata)).collect();
        dirs.sort_by(|a,b|a.name.cmp(&b.name));
        let root = BaseInfo::new(ctx.request().url().path().to_owned(), files, dirs);
        match format.subtype().as_ref(){
            "text"=> ctx.render_text(list_text(&root)),
            "json"=> ctx.render_json(list_json(&root)),
            "xml"=> ctx.render_xml(list_xml(&root)),
            _ => ctx.render_html(list_html(&root)),
        }
    }
}
