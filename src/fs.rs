use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

#[derive(Clone)]
pub enum Node {
    File(Vec<u8>),
    Directory(BTreeMap<String, Node>),
}

pub struct FileSystem {
    root: Node,
}

lazy_static! {
    pub static ref FILESYSTEM: Mutex<FileSystem> = Mutex::new(FileSystem::new());
}

impl FileSystem {
    pub fn new() -> Self {
        let mut root_map = BTreeMap::new();
        root_map.insert(String::from("bin"), Node::Directory(BTreeMap::new()));
        root_map.insert(String::from("etc"), Node::Directory(BTreeMap::new()));
        root_map.insert(String::from("home"), Node::Directory(BTreeMap::new()));
        root_map.insert(String::from("var"), Node::Directory(BTreeMap::new()));
        
        FileSystem {
            root: Node::Directory(root_map),
        }
    }

    fn get_node_mut(&mut self, path: &str) -> Option<&mut Node> {
        let mut current = &mut self.root;
        for part in path.split('/').filter(|s| !s.is_empty()) {
            match current {
                Node::Directory(ref mut map) => {
                    current = map.get_mut(part)?;
                }
                _ => return None,
            }
        }
        Some(current)
    }

    pub fn mkdir(&mut self, path: &str) -> Result<(), &'static str> {
        let mut parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return Err("Invalid path"); }
        let name = parts.pop().unwrap();
        let parent_path = parts.join("/");
        
        let parent = self.get_node_mut(&parent_path).ok_or("Parent directory not found")?;
        match parent {
            Node::Directory(ref mut map) => {
                if map.contains_key(name) { return Err("Already exists"); }
                map.insert(String::from(name), Node::Directory(BTreeMap::new()));
                Ok(())
            }
            _ => Err("Parent is not a directory"),
        }
    }

    pub fn touch(&mut self, path: &str) -> Result<(), &'static str> {
        let mut parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return Err("Invalid path"); }
        let name = parts.pop().unwrap();
        let parent_path = parts.join("/");
        
        let parent = self.get_node_mut(&parent_path).ok_or("Parent directory not found")?;
        match parent {
            Node::Directory(ref mut map) => {
                if map.contains_key(name) { return Ok(()); }
                map.insert(String::from(name), Node::File(Vec::new()));
                Ok(())
            }
            _ => Err("Parent is not a directory"),
        }
    }

    pub fn read_dir(&mut self, path: &str) -> Result<Vec<String>, &'static str> {
        let node = self.get_node_mut(path).ok_or("Not found")?;
        match node {
            Node::Directory(map) => {
                Ok(map.keys().cloned().collect())
            }
            _ => Err("Not a directory"),
        }
    }

    pub fn write_file(&mut self, path: &str, data: Vec<u8>) -> Result<(), &'static str> {
        let mut parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return Err("Invalid path"); }
        let name = parts.pop().unwrap();
        let parent_path = parts.join("/");
        
        let parent = self.get_node_mut(&parent_path).ok_or("Parent directory not found")?;
        match parent {
            Node::Directory(ref mut map) => {
                map.insert(String::from(name), Node::File(data));
                Ok(())
            }
            _ => Err("Parent is not a directory"),
        }
    }

    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>, &'static str> {
        let node = self.get_node_mut(path).ok_or("Not found")?;
        match node {
            Node::File(data) => Ok(data.clone()),
            _ => Err("Not a file"),
        }
    }

    pub fn remove(&mut self, path: &str) -> Result<(), &'static str> {
        let mut parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return Err("Invalid path"); }
        let name = parts.pop().unwrap();
        let parent_path = parts.join("/");
        
        let parent = self.get_node_mut(&parent_path).ok_or("Parent directory not found")?;
        match parent {
            Node::Directory(ref mut map) => {
                map.remove(name).ok_or("File not found")?;
                Ok(())
            }
            _ => Err("Parent is not a directory"),
        }
    }
}
