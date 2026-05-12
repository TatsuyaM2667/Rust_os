use alloc::string::String;
use alloc::vec;
use crate::fs::FILESYSTEM;

pub struct Package {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
}

const AVAILABLE_PACKAGES: &[Package] = &[
    Package { name: "rust-compiler", version: "1.78.0", description: "The Rust Programming Language" },
    Package { name: "vim", version: "9.1", description: "Vi Improved text editor" },
    Package { name: "git", version: "2.45.0", description: "Distributed revision control system" },
    Package { name: "htop", version: "3.3.0", description: "Interactive process viewer" },
    Package { name: "python", version: "3.12.3", description: "General-purpose programming language" },
];

pub fn install(name: &str) {
    let pkg = AVAILABLE_PACKAGES.iter().find(|p| p.name == name);
    match pkg {
        Some(p) => {
            println!("resolving dependencies...");
            println!("looking for conflicting packages...");
            println!("Packages (1) {} - {}", p.name, p.version);
            println!("\nTotal Download Size:  24.5 MiB");
            println!("Total Installed Size: 112.0 MiB");
            println!("\n:: Proceed with installation? [Y/n] Y");
            println!("(1/1) checking keys in keyring...");
            println!("(1/1) checking package integrity...");
            println!("(1/1) loading package files...");
            println!("(1/1) checking for file conflicts...");
            println!("(1/1) checking available disk space...");
            println!("(1/1) installing {}...", p.name);

            // Create a dummy binary in /bin
            let mut fs = FILESYSTEM.lock();
            let path = alloc::format!("/bin/{}", p.name);
            fs.touch(&path).unwrap();
            fs.write_file(&path, vec![0xEB, 0xFE]).unwrap(); // Dummy content

            // Save to database (mock)
            let db_path = alloc::format!("/var/lib/pacman/local/{}/desc", p.name);
            fs.mkdir("/var/lib/pacman").ok();
            fs.mkdir("/var/lib/pacman/local").ok();
            fs.mkdir(&alloc::format!("/var/lib/pacman/local/{}", p.name)).ok();
            fs.write_file(&db_path, p.description.as_bytes().to_vec()).unwrap();

            println!(":: Running post-transaction hooks...");
            println!("(1/1) Updating icon theme cache...");
        }
        None => println!("error: target not found: {}", name),
    }
}

pub fn search(query: &str) {
    println!(":: Searching remote repositories...");
    for pkg in AVAILABLE_PACKAGES {
        if pkg.name.contains(query) || pkg.description.contains(query) {
            println!("core/{} {} [installed: no]", pkg.name, pkg.version);
            println!("    {}", pkg.description);
        }
    }
}

pub fn list_installed() {
    let mut fs = FILESYSTEM.lock();
    match fs.read_dir("/var/lib/pacman/local") {
        Ok(pkgs) => {
            for pkg in pkgs {
                let desc_path = alloc::format!("/var/lib/pacman/local/{}/desc", pkg);
                let desc = fs.read_file(&desc_path).map(|d| String::from_utf8_lossy(&d).into_owned()).unwrap_or_default();
                println!("{} - {}", pkg, desc);
            }
        }
        Err(_) => println!("No packages installed."),
    }
}

pub fn remove(name: &str) {
    let mut fs = FILESYSTEM.lock();
    let bin_path = alloc::format!("/bin/{}", name);
    let db_dir = alloc::format!("/var/lib/pacman/local/{}", name);
    
    if fs.remove(&bin_path).is_ok() {
        println!("checking dependencies...");
        println!("\nPackages (1) {} - remove", name);
        println!("\nTotal Removed Size: 112.0 MiB");
        println!("\n:: Do you want to remove these packages? [Y/n] Y");
        println!("(1/1) removing {}...", name);
        
        // Remove DB entry
        let db_file = alloc::format!("{}/desc", db_dir);
        fs.remove(&db_file).ok();
        fs.remove(&db_dir).ok();
    } else {
        println!("error: target not found: {}", name);
    }
}
