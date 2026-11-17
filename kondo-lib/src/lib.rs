use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    error::{self, Error},
    fs,
    path::{self, Path},
    time::SystemTime,
};

const FILE_CARGO_TOML: &str = "Cargo.toml";
const FILE_PACKAGE_JSON: &str = "package.json";
const FILE_PYPROJECT_TOML: &str = "pyproject.toml";
const FILE_UV_LOCK: &str = "uv.lock";
const FILE_SETUP_PY: &str = "setup.py";
const FILE_ASSEMBLY_CSHARP: &str = "Assembly-CSharp.csproj";
const FILE_STACK_HASKELL: &str = "stack.yaml";
const FILE_CABAL_HASKELL: &str = "cabal.project";
const FILE_SBT_BUILD: &str = "build.sbt";
const FILE_MVN_BUILD: &str = "pom.xml";
const FILE_BUILD_GRADLE: &str = "build.gradle";
const FILE_BUILD_GRADLE_KTS: &str = "build.gradle.kts";
const FILE_CMAKE_BUILD: &str = "CMakeLists.txt";
const FILE_UNREAL_SUFFIX: &str = ".uproject";
const FILE_JUPYTER_SUFFIX: &str = ".ipynb";
const FILE_PYTHON_SUFFIX: &str = ".py";
const FILE_PIXI_PACKAGE: &str = "pixi.toml";
const FILE_COMPOSER_JSON: &str = "composer.json";
const FILE_PUBSPEC_YAML: &str = "pubspec.yaml";
const FILE_ELIXIR_MIX: &str = "mix.exs";
const FILE_SWIFT_PACKAGE: &str = "Package.swift";
const FILE_BUILD_ZIG: &str = "build.zig";
const FILE_GODOT_4_PROJECT: &str = "project.godot";
const FILE_CSPROJ_SUFFIX: &str = ".csproj";
const FILE_FSPROJ_SUFFIX: &str = ".fsproj";
const FILE_TERRAFORM_HCL: &str = ".terraform.lock.hcl";
const FILE_PROJECT_TURBOREPO: &str = "turbo.json";
const FILE_PROJECT_TURBOREPO_JSONC: &str = "turbo.jsonc";
const FILE_PLATFORMIO_INI: &str = "platformio.ini";
const FILE_PODFILE: &str = "Podfile";

const PROJECT_CARGO_DIRS: [&str; 2] = ["target", ".xwin-cache"];
const PROJECT_NODE_DIRS: [&str; 10] = [
    "node_modules",
    ".angular",
    ".next",
    ".cache",
    ".turbo",
    "dist",
    "build",
    "tmp",
    "coverage",
    ".nyc_output",
];
const GENERIC_ARTIFACT_DIRS: [&str; 23] = [
    "node_modules",
    ".angular",
    ".next",
    ".cache",
    ".turbo",
    "dist",
    "build",
    "tmp",
    "target",
    ".gradle",
    "obj",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    ".build",
    "Pods",
    ".dart_tool",
    ".terraform",
    "coverage",
    "out",
];
const PROJECT_REACT_NATIVE_DIRS: [&str; 16] = [
    "node_modules",
    "android/build",
    "android/.gradle",
    "ios/build",
    "ios/DerivedData",
    "ios/Pods",
    ".expo",
    ".metro",
    "dist",
    "build",
    "tmp",
    "coverage",
    ".next",
    ".cache",
    ".turbo",
    ".nyc_output",
];
const PROJECT_UNITY_DIRS: [&str; 7] = [
    "Library",
    "Temp",
    "Obj",
    "Logs",
    "MemoryCaptures",
    "Build",
    "Builds",
];
const PROJECT_STACK_DIRS: [&str; 1] = [".stack-work"];
const PROJECT_CABAL_DIRS: [&str; 1] = ["dist-newstyle"];
const PROJECT_SBT_DIRS: [&str; 2] = ["target", "project/target"];
const PROJECT_MVN_DIRS: [&str; 1] = ["target"];
const PROJECT_GRADLE_DIRS: [&str; 2] = ["build", ".gradle"];
const PROJECT_CMAKE_DIRS: [&str; 3] = ["build", "cmake-build-debug", "cmake-build-release"];
const PROJECT_UNREAL_DIRS: [&str; 5] = [
    "Binaries",
    "Build",
    "Saved",
    "DerivedDataCache",
    "Intermediate",
];
const PROJECT_JUPYTER_DIRS: [&str; 1] = [".ipynb_checkpoints"];
const PROJECT_PYTHON_DIRS: [&str; 12] = [
    ".mypy_cache",
    ".nox",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    "__pycache__",
    "__pypackages__",
    "venv",
    "build",
    "dist",
    "coverage",
    "htmlcov",
];
const PROJECT_PIXI_DIRS: [&str; 1] = [".pixi"];
const PROJECT_COMPOSER_DIRS: [&str; 1] = ["vendor"];
const PROJECT_PUB_DIRS: [&str; 4] = [
    "build",
    ".dart_tool",
    "linux/flutter/ephemeral",
    "windows/flutter/ephemeral",
];
const PROJECT_ELIXIR_DIRS: [&str; 4] = ["_build", ".elixir-tools", ".elixir_ls", ".lexical"];
const PROJECT_SWIFT_DIRS: [&str; 2] = [".build", ".swiftpm"];
const PROJECT_ZIG_DIRS: [&str; 3] = ["zig-cache", ".zig-cache", "zig-out"];
const PROJECT_GODOT_4_DIRS: [&str; 1] = [".godot"];
const PROJECT_DOTNET_DIRS: [&str; 2] = ["bin", "obj"];
const PROJECT_TURBOREPO_DIRS: [&str; 1] = [".turbo"];
const PROJECT_TERRAFORM_DIRS: [&str; 1] = [".terraform"];
const PROJECT_PLATFORMIO_DIRS: [&str; 1] = [".pio"];
const PROJECT_COCOAPODS_DIRS: [&str; 1] = ["Pods"];

const PROJECT_CARGO_NAME: &str = "Cargo";
const PROJECT_NODE_NAME: &str = "Node";
const PROJECT_NODE_REACT_NATIVE_NAME: &str = "Node (React Native)";
const PROJECT_UNITY_NAME: &str = "Unity";
const PROJECT_STACK_NAME: &str = "Stack";
const PROJECT_CABAL_NAME: &str = "Cabal";
const PROJECT_SBT_NAME: &str = "SBT";
const PROJECT_MVN_NAME: &str = "Maven";
const PROJECT_GRADLE_NAME: &str = "Gradle";
const PROJECT_CMAKE_NAME: &str = "CMake";
const PROJECT_UNREAL_NAME: &str = "Unreal";
const PROJECT_JUPYTER_NAME: &str = "Jupyter";
const PROJECT_PYTHON_NAME: &str = "Python";
const PROJECT_PIXI_NAME: &str = "Pixi";
const PROJECT_COMPOSER_NAME: &str = "Composer";
const PROJECT_PUB_NAME: &str = "Pub";
const PROJECT_ELIXIR_NAME: &str = "Elixir";
const PROJECT_SWIFT_NAME: &str = "Swift";
const PROJECT_ZIG_NAME: &str = "Zig";
const PROJECT_GODOT_4_NAME: &str = "Godot 4.x";
const PROJECT_DOTNET_NAME: &str = ".NET";
const PROJECT_TURBOREPO_NAME: &str = "Turborepo";
const PROJECT_TERRAFORM_NAME: &str = "Terraform";
const PROJECT_PLATFORMIO_NAME: &str = "PlatformIO";
const PROJECT_COCOAPODS_NAME: &str = "CocoaPods";

#[derive(Debug, Clone)]
pub enum ProjectType {
    Cargo,
    Node,
    Unity,
    Stack,
    Cabal,
    #[allow(clippy::upper_case_acronyms)]
    SBT,
    Maven,
    Gradle,
    CMake,
    Unreal,
    Jupyter,
    Python,
    Pixi,
    Composer,
    Pub,
    Elixir,
    Swift,
    Zig,
    Godot4,
    Dotnet,
    Turborepo,
    Terraform,
    PlatformIO,
    Cocoapods,
    Generic,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub project_type: ProjectType,
    pub path: path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct ProjectSize {
    pub artifact_size: u64,
    pub non_artifact_size: u64,
    pub dirs: Vec<(String, u64, bool)>,
}

impl Project {
    pub fn artifact_dirs(&self) -> &'static [&'static str] {
        match self.project_type {
            ProjectType::Cargo => &PROJECT_CARGO_DIRS,
            ProjectType::Node => {
                if is_react_native_project(&self.path) {
                    &PROJECT_REACT_NATIVE_DIRS
                } else {
                    &PROJECT_NODE_DIRS
                }
            }
            ProjectType::Unity => &PROJECT_UNITY_DIRS,
            ProjectType::Stack => &PROJECT_STACK_DIRS,
            ProjectType::Cabal => &PROJECT_CABAL_DIRS,
            ProjectType::SBT => &PROJECT_SBT_DIRS,
            ProjectType::Maven => &PROJECT_MVN_DIRS,
            ProjectType::Unreal => &PROJECT_UNREAL_DIRS,
            ProjectType::Jupyter => &PROJECT_JUPYTER_DIRS,
            ProjectType::Python => &PROJECT_PYTHON_DIRS,
            ProjectType::Pixi => &PROJECT_PIXI_DIRS,
            ProjectType::CMake => &PROJECT_CMAKE_DIRS,
            ProjectType::Composer => &PROJECT_COMPOSER_DIRS,
            ProjectType::Pub => &PROJECT_PUB_DIRS,
            ProjectType::Elixir => &PROJECT_ELIXIR_DIRS,
            ProjectType::Swift => &PROJECT_SWIFT_DIRS,
            ProjectType::Gradle => &PROJECT_GRADLE_DIRS,
            ProjectType::Zig => &PROJECT_ZIG_DIRS,
            ProjectType::Godot4 => &PROJECT_GODOT_4_DIRS,
            ProjectType::Dotnet => &PROJECT_DOTNET_DIRS,
            ProjectType::Turborepo => &PROJECT_TURBOREPO_DIRS,
            ProjectType::Terraform => &PROJECT_TERRAFORM_DIRS,
            ProjectType::PlatformIO => &PROJECT_PLATFORMIO_DIRS,
            ProjectType::Cocoapods => &PROJECT_COCOAPODS_DIRS,
            ProjectType::Generic => &GENERIC_ARTIFACT_DIRS,
        }
    }

    pub fn name(&self) -> Cow<'_, str> {
        self.path.to_string_lossy()
    }

    pub fn size(&self, options: &ScanOptions) -> u64 {
        self.artifact_dirs()
            .iter()
            .copied()
            .map(|p| dir_size(&self.path.join(p), options))
            .sum()
    }

    pub fn last_modified(&self, options: &ScanOptions) -> Result<SystemTime, std::io::Error> {
        let top_level_modified = fs::metadata(&self.path)?.modified()?;
        let most_recent_modified = if options.no_ignore {
            walkdir::WalkDir::new(&self.path)
                .follow_links(options.follow_symlinks)
                .same_file_system(options.same_file_system)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .filter_map(|m| m.modified().ok())
                .max()
                .unwrap_or(top_level_modified)
        } else {
            ignore::WalkBuilder::new(&self.path)
                .follow_links(options.follow_symlinks)
                .same_file_system(options.same_file_system)
                .build()
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .filter_map(|m| m.modified().ok())
                .max()
                .unwrap_or(top_level_modified)
        };
        Ok(most_recent_modified)
    }

    pub fn size_dirs(&self, options: &ScanOptions) -> ProjectSize {
        let mut artifact_size = 0;
        let mut non_artifact_size = 0;
        let mut dirs = Vec::new();

        let project_root = match fs::read_dir(&self.path) {
            Err(_) => {
                return ProjectSize {
                    artifact_size,
                    non_artifact_size,
                    dirs,
                }
            }
            Ok(rd) => rd,
        };

        for entry in project_root.filter_map(|rd| rd.ok()) {
            let file_type = match entry.file_type() {
                Err(_) => continue,
                Ok(file_type) => file_type,
            };

            if file_type.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    non_artifact_size += metadata.len();
                }
                continue;
            }

            if file_type.is_dir() {
                let file_name = match entry.file_name().into_string() {
                    Err(_) => continue,
                    Ok(file_name) => file_name,
                };
                let size = dir_size(&entry.path(), options);
                let artifact_dir = self.artifact_dirs().contains(&file_name.as_str());
                if artifact_dir {
                    artifact_size += size;
                } else {
                    non_artifact_size += size;
                }
                dirs.push((file_name, size, artifact_dir));
            }
        }

        ProjectSize {
            artifact_size,
            non_artifact_size,
            dirs,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self.project_type {
            ProjectType::Cargo => PROJECT_CARGO_NAME,
            ProjectType::Node => {
                if is_react_native_project(&self.path) {
                    PROJECT_NODE_REACT_NATIVE_NAME
                } else {
                    PROJECT_NODE_NAME
                }
            }
            ProjectType::Unity => PROJECT_UNITY_NAME,
            ProjectType::Stack => PROJECT_STACK_NAME,
            ProjectType::Cabal => PROJECT_CABAL_NAME,
            ProjectType::SBT => PROJECT_SBT_NAME,
            ProjectType::Maven => PROJECT_MVN_NAME,
            ProjectType::Unreal => PROJECT_UNREAL_NAME,
            ProjectType::Jupyter => PROJECT_JUPYTER_NAME,
            ProjectType::Python => PROJECT_PYTHON_NAME,
            ProjectType::Pixi => PROJECT_PIXI_NAME,
            ProjectType::CMake => PROJECT_CMAKE_NAME,
            ProjectType::Composer => PROJECT_COMPOSER_NAME,
            ProjectType::Pub => PROJECT_PUB_NAME,
            ProjectType::Elixir => PROJECT_ELIXIR_NAME,
            ProjectType::Swift => PROJECT_SWIFT_NAME,
            ProjectType::Gradle => PROJECT_GRADLE_NAME,
            ProjectType::Zig => PROJECT_ZIG_NAME,
            ProjectType::Godot4 => PROJECT_GODOT_4_NAME,
            ProjectType::Dotnet => PROJECT_DOTNET_NAME,
            ProjectType::Turborepo => PROJECT_TURBOREPO_NAME,
            ProjectType::Terraform => PROJECT_TERRAFORM_NAME,
            ProjectType::PlatformIO => PROJECT_PLATFORMIO_NAME,
            ProjectType::Cocoapods => PROJECT_COCOAPODS_NAME,
            ProjectType::Generic => "Generated artifacts",
        }
    }

    /// Deletes the project's artifact directories and their contents
    pub fn clean(&self) {
        for artifact_dir in self
            .artifact_dirs()
            .iter()
            .copied()
            .map(|ad| self.path.join(ad))
            .filter(|ad| ad.exists())
        {
            if let Err(e) = fs::remove_dir_all(&artifact_dir) {
                eprintln!("error removing directory {:?}: {:?}", artifact_dir, e);
            }
        }
    }
}

pub fn print_elapsed(secs: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = MINUTE * 60;
    const DAY: u64 = HOUR * 24;
    const WEEK: u64 = DAY * 7;
    const MONTH: u64 = WEEK * 4;
    const YEAR: u64 = DAY * 365;

    let (unit, fstring) = match secs {
        secs if secs < MINUTE => (secs as f64, "second"),
        secs if secs < HOUR * 2 => (secs as f64 / MINUTE as f64, "minute"),
        secs if secs < DAY * 2 => (secs as f64 / HOUR as f64, "hour"),
        secs if secs < WEEK * 2 => (secs as f64 / DAY as f64, "day"),
        secs if secs < MONTH * 2 => (secs as f64 / WEEK as f64, "week"),
        secs if secs < YEAR * 2 => (secs as f64 / MONTH as f64, "month"),
        secs => (secs as f64 / YEAR as f64, "year"),
    };

    let unit = unit.round();

    let plural = if unit == 1.0 { "" } else { "s" };

    format!("{unit:.0} {fstring}{plural} ago")
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.')
}

struct ProjectIter {
    it: walkdir::IntoIter,
    no_ignore: bool,
    deep: bool,
    project_artifacts: HashMap<path::PathBuf, Vec<&'static str>>,
    project_artifact_paths: HashSet<path::PathBuf>,
    generic_roots: HashSet<path::PathBuf>,
}

pub enum Red {
    IOError(::std::io::Error),
    WalkdirError(walkdir::Error),
}

impl Iterator for ProjectIter {
    type Item = Result<Project, Red>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry: walkdir::DirEntry = match self.it.next() {
                None => return None,
                Some(Err(e)) => return Some(Err(Red::WalkdirError(e))),
                Some(Ok(entry)) => entry,
            };
            if !entry.file_type().is_dir() {
                continue;
            }
            if !self.no_ignore && entry.depth() > 0 && is_hidden(&entry) {
                self.it.skip_current_dir();
                continue;
            }
            let entry_name = entry.file_name().to_str().unwrap_or_default();
            let parent = entry.path().parent();
            let project_artifact = self.project_artifact_paths.contains(entry.path())
                || parent
                    .and_then(|path| self.project_artifacts.get(path))
                    .is_some_and(|dirs| dirs.iter().any(|dir| *dir == entry_name));
            let generic_artifact = GENERIC_ARTIFACT_DIRS.contains(&entry_name);
            // A project virtual environment contains installed dependencies, not disposable
            // Python caches. Keep it even though `.venv` is a generic artifact name elsewhere.
            let project_venv = entry_name == ".venv"
                && parent.is_some_and(|path| self.project_artifacts.contains_key(path));
            if project_venv {
                self.it.skip_current_dir();
                continue;
            }
            if entry.depth() > 0 && (project_artifact || generic_artifact) {
                self.it.skip_current_dir();
                if project_artifact {
                    continue;
                }
                if let Some(parent) = parent {
                    if self.project_artifacts.contains_key(parent)
                        || !self.generic_roots.insert(parent.to_path_buf())
                    {
                        continue;
                    }
                    return Some(Ok(Project {
                        project_type: ProjectType::Generic,
                        path: parent.to_path_buf(),
                    }));
                }
                continue;
            }
            let rd = match entry.path().read_dir() {
                Err(e) => return Some(Err(Red::IOError(e))),
                Ok(rd) => rd,
            };
            let mut python_project = false;
            // intentionally ignoring errors while iterating the ReadDir
            // can't return them because we'll lose the context of where we are
            for dir_entry in rd
                .filter_map(|rd| rd.ok())
                .filter(|de| de.file_type().map(|ft| ft.is_file()).unwrap_or(false))
                .map(|de| de.file_name())
            {
                let file_name = match dir_entry.to_str() {
                    None => continue,
                    Some(file_name) => file_name,
                };
                let p_type = match file_name {
                    FILE_CARGO_TOML => Some(ProjectType::Cargo),
                    FILE_PACKAGE_JSON => Some(ProjectType::Node),
                    FILE_UV_LOCK => Some(ProjectType::Python),
                    FILE_PYPROJECT_TOML | FILE_SETUP_PY => Some(ProjectType::Python),
                    FILE_ASSEMBLY_CSHARP => Some(ProjectType::Unity),
                    FILE_STACK_HASKELL => Some(ProjectType::Stack),
                    FILE_CABAL_HASKELL => Some(ProjectType::Cabal),
                    FILE_SBT_BUILD => Some(ProjectType::SBT),
                    FILE_MVN_BUILD => Some(ProjectType::Maven),
                    FILE_CMAKE_BUILD => {
                        if dir_contains_file(entry.path(), FILE_PLATFORMIO_INI) {
                            Some(ProjectType::PlatformIO)
                        } else {
                            Some(ProjectType::CMake)
                        }
                    }
                    FILE_COMPOSER_JSON => Some(ProjectType::Composer),
                    FILE_PUBSPEC_YAML => Some(ProjectType::Pub),
                    FILE_PIXI_PACKAGE => Some(ProjectType::Pixi),
                    FILE_ELIXIR_MIX => Some(ProjectType::Elixir),
                    FILE_SWIFT_PACKAGE => Some(ProjectType::Swift),
                    FILE_BUILD_GRADLE => Some(ProjectType::Gradle),
                    FILE_BUILD_GRADLE_KTS => Some(ProjectType::Gradle),
                    FILE_BUILD_ZIG => Some(ProjectType::Zig),
                    FILE_GODOT_4_PROJECT => Some(ProjectType::Godot4),
                    FILE_PROJECT_TURBOREPO => Some(ProjectType::Turborepo),
                    FILE_PROJECT_TURBOREPO_JSONC => Some(ProjectType::Turborepo),
                    FILE_TERRAFORM_HCL => Some(ProjectType::Terraform),
                    FILE_PLATFORMIO_INI => Some(ProjectType::PlatformIO),
                    FILE_PODFILE => Some(ProjectType::Cocoapods),
                    file_name if file_name.ends_with(FILE_UNREAL_SUFFIX) => {
                        Some(ProjectType::Unreal)
                    }
                    file_name if file_name.ends_with(FILE_JUPYTER_SUFFIX) => {
                        Some(ProjectType::Jupyter)
                    }
                    file_name if file_name.ends_with(FILE_PYTHON_SUFFIX) => {
                        python_project = true;
                        None
                    }
                    file_name
                        if file_name.ends_with(FILE_CSPROJ_SUFFIX)
                            || file_name.ends_with(FILE_FSPROJ_SUFFIX) =>
                    {
                        if dir_contains_file(entry.path(), FILE_GODOT_4_PROJECT) {
                            Some(ProjectType::Godot4)
                        } else if dir_contains_file(entry.path(), FILE_ASSEMBLY_CSHARP) {
                            Some(ProjectType::Unity)
                        } else {
                            Some(ProjectType::Dotnet)
                        }
                    }
                    _ => None,
                };
                if let Some(project_type) = p_type {
                    let project = Project {
                        project_type,
                        path: entry.path().to_path_buf(),
                    };
                    self.project_artifacts
                        .insert(project.path.clone(), project.artifact_dirs().to_vec());
                    self.project_artifact_paths.extend(
                        project
                            .artifact_dirs()
                            .iter()
                            .map(|dir| project.path.join(dir)),
                    );
                    if !self.deep {
                        self.it.skip_current_dir();
                    }
                    return Some(Ok(project));
                }
            }
            if python_project {
                let project = Project {
                    project_type: ProjectType::Python,
                    path: entry.path().to_path_buf(),
                };
                self.project_artifacts
                    .insert(project.path.clone(), project.artifact_dirs().to_vec());
                self.project_artifact_paths.extend(
                    project
                        .artifact_dirs()
                        .iter()
                        .map(|dir| project.path.join(dir)),
                );
                if !self.deep {
                    self.it.skip_current_dir();
                }
                return Some(Ok(project));
            }
        }
    }
}

fn dir_contains_file(path: &Path, file: &str) -> bool {
    path.read_dir()
        .map(|rd| {
            rd.filter_map(|rd| rd.ok()).any(|de| {
                de.file_type().is_ok_and(|t| t.is_file()) && de.file_name().to_str() == Some(file)
            })
        })
        .unwrap_or(false)
}

fn dir_contains_subdir(path: &Path, subdir: &str) -> bool {
    path.read_dir()
        .map(|rd| {
            rd.filter_map(|rd| rd.ok()).any(|de| {
                de.file_type().is_ok_and(|t| t.is_dir()) && de.file_name().to_str() == Some(subdir)
            })
        })
        .unwrap_or(false)
}

fn is_react_native_project(path: &Path) -> bool {
    dir_contains_subdir(path, "ios") || dir_contains_subdir(path, "android")
}

#[derive(Clone, Debug)]
pub struct ScanOptions {
    pub follow_symlinks: bool,
    pub same_file_system: bool,
    pub no_ignore: bool,
    pub deep: bool,
}

fn build_walkdir_iter<P: AsRef<path::Path>>(path: &P, options: &ScanOptions) -> ProjectIter {
    ProjectIter {
        it: walkdir::WalkDir::new(path)
            .follow_links(options.follow_symlinks)
            .same_file_system(options.same_file_system)
            .into_iter(),
        no_ignore: options.no_ignore,
        deep: options.deep,
        project_artifacts: HashMap::new(),
        project_artifact_paths: HashSet::new(),
        generic_roots: HashSet::new(),
    }
}

pub fn scan<P: AsRef<path::Path>>(
    path: &P,
    options: &ScanOptions,
) -> impl Iterator<Item = Result<Project, Red>> {
    build_walkdir_iter(path, options)
}

// TODO does this need to exist as is??
pub fn dir_size<P: AsRef<path::Path>>(path: &P, options: &ScanOptions) -> u64 {
    build_walkdir_iter(path, options)
        .it
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|e| e.len())
        .sum()
}

pub fn pretty_size(size: u64) -> String {
    const KIBIBYTE: u64 = 1024;
    const MEBIBYTE: u64 = 1_048_576;
    const GIBIBYTE: u64 = 1_073_741_824;
    const TEBIBYTE: u64 = 1_099_511_627_776;
    const PEBIBYTE: u64 = 1_125_899_906_842_624;
    const EXBIBYTE: u64 = 1_152_921_504_606_846_976;

    let (size, symbol) = match size {
        size if size < KIBIBYTE => (size as f64, "B"),
        size if size < MEBIBYTE => (size as f64 / KIBIBYTE as f64, "KiB"),
        size if size < GIBIBYTE => (size as f64 / MEBIBYTE as f64, "MiB"),
        size if size < TEBIBYTE => (size as f64 / GIBIBYTE as f64, "GiB"),
        size if size < PEBIBYTE => (size as f64 / TEBIBYTE as f64, "TiB"),
        size if size < EXBIBYTE => (size as f64 / PEBIBYTE as f64, "PiB"),
        _ => (size as f64 / EXBIBYTE as f64, "EiB"),
    };

    format!("{:.1}{}", size, symbol)
}

pub fn clean(project_path: &str) -> Result<(), Box<dyn error::Error>> {
    let project = fs::read_dir(project_path)?
        .filter_map(|rd| rd.ok())
        .find_map(|dir_entry| {
            let file_name = dir_entry.file_name().into_string().ok()?;
            let p_type = match file_name.as_str() {
                FILE_CARGO_TOML => Some(ProjectType::Cargo),
                FILE_PACKAGE_JSON => Some(ProjectType::Node),
                FILE_PYPROJECT_TOML | FILE_UV_LOCK
                    if dir_entry
                        .file_type()
                        .is_ok_and(|file_type| file_type.is_file()) =>
                {
                    Some(ProjectType::Python)
                }
                FILE_ASSEMBLY_CSHARP => Some(ProjectType::Unity),
                FILE_STACK_HASKELL => Some(ProjectType::Stack),
                FILE_CABAL_HASKELL => Some(ProjectType::Cabal),
                FILE_SBT_BUILD => Some(ProjectType::SBT),
                FILE_MVN_BUILD => Some(ProjectType::Maven),
                FILE_CMAKE_BUILD => {
                    if dir_contains_file(path::Path::new(project_path), FILE_PLATFORMIO_INI) {
                        Some(ProjectType::PlatformIO)
                    } else {
                        Some(ProjectType::CMake)
                    }
                }
                FILE_COMPOSER_JSON => Some(ProjectType::Composer),
                FILE_PUBSPEC_YAML => Some(ProjectType::Pub),
                FILE_PIXI_PACKAGE => Some(ProjectType::Pixi),
                FILE_ELIXIR_MIX => Some(ProjectType::Elixir),
                FILE_SWIFT_PACKAGE => Some(ProjectType::Swift),
                FILE_BUILD_ZIG => Some(ProjectType::Zig),
                FILE_GODOT_4_PROJECT => Some(ProjectType::Godot4),
                FILE_PROJECT_TURBOREPO | FILE_PROJECT_TURBOREPO_JSONC
                    if dir_entry
                        .file_type()
                        .is_ok_and(|file_type| file_type.is_file()) =>
                {
                    Some(ProjectType::Turborepo)
                }
                FILE_TERRAFORM_HCL => Some(ProjectType::Terraform),
                FILE_PLATFORMIO_INI
                    if dir_entry
                        .file_type()
                        .is_ok_and(|file_type| file_type.is_file()) =>
                {
                    Some(ProjectType::PlatformIO)
                }
                FILE_PODFILE => Some(ProjectType::Cocoapods),
                _ => None,
            };
            if let Some(project_type) = p_type {
                return Some(Project {
                    project_type,
                    path: project_path.into(),
                });
            }
            None
        });

    if let Some(project) = project {
        for artifact_dir in project
            .artifact_dirs()
            .iter()
            .copied()
            .map(|ad| path::PathBuf::from(project_path).join(ad))
            .filter(|ad| ad.exists())
        {
            if let Err(e) = fs::remove_dir_all(&artifact_dir) {
                eprintln!("error removing directory {:?}: {:?}", artifact_dir, e);
            }
        }
    }

    Ok(())
}
pub fn path_canonicalise(
    base: &path::Path,
    tail: path::PathBuf,
) -> Result<path::PathBuf, Box<dyn Error>> {
    if tail.is_absolute() {
        Ok(tail)
    } else {
        Ok(base.join(tail).canonicalize()?)
    }
}

#[cfg(test)]
mod tests {
    use super::{clean, print_elapsed, scan, ScanOptions};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "soji-uv-test-{}-{timestamp}-{}",
                std::process::id(),
                NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn uv_lock_detects_python_project_without_root_python_file() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("uv.lock"), "version = 1\n").unwrap();
        fs::create_dir(fixture.0.join("__pycache__")).unwrap();
        fs::create_dir(fixture.0.join(".venv")).unwrap();
        fs::write(fixture.0.join(".venv/pyvenv.cfg"), "home = /usr/bin\n").unwrap();

        let options = ScanOptions {
            follow_symlinks: false,
            same_file_system: false,
            no_ignore: false,
            deep: false,
        };
        let projects = scan(&fixture.0, &options)
            .map(|result| match result {
                Ok(project) => project,
                Err(_) => panic!("scan fixture without filesystem errors"),
            })
            .collect::<Vec<_>>();

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].type_name(), "Python");
        assert_eq!(projects[0].path, fixture.0);
        assert_eq!(projects[0].size(&options), 0);
        projects[0].clean();
        assert!(fixture.0.join(".venv/pyvenv.cfg").exists());
    }

    #[test]
    fn direct_clean_accepts_regular_python_markers_but_not_marker_directories() {
        for marker in ["uv.lock", "pyproject.toml"] {
            let fixture = Fixture::new();
            fs::write(fixture.0.join(marker), "").unwrap();
            fs::create_dir(fixture.0.join("__pycache__")).unwrap();
            fs::create_dir(fixture.0.join(".venv")).unwrap();
            fs::write(fixture.0.join(".venv/pyvenv.cfg"), "").unwrap();
            clean(fixture.0.to_str().unwrap()).unwrap();
            assert!(!fixture.0.join("__pycache__").exists());
            assert!(fixture.0.join(".venv/pyvenv.cfg").exists());
        }

        for marker in ["uv.lock", "pyproject.toml"] {
            let fixture = Fixture::new();
            fs::create_dir(fixture.0.join(marker)).unwrap();
            fs::create_dir(fixture.0.join("__pycache__")).unwrap();
            clean(fixture.0.to_str().unwrap()).unwrap();
            assert!(fixture.0.join("__pycache__").exists());
        }
    }

    #[test]
    fn turbo_jsonc_detects_and_cleans_turborepo_projects() {
        let options = ScanOptions {
            follow_symlinks: false,
            same_file_system: false,
            no_ignore: false,
            deep: false,
        };

        for marker in ["turbo.json", "turbo.jsonc"] {
            let fixture = Fixture::new();
            fs::write(
                fixture.0.join(marker),
                "{\"$schema\": \"https://turbo.build/schema.json\"}\n",
            )
            .unwrap();
            fs::create_dir_all(fixture.0.join(".turbo/cache")).unwrap();

            let projects = scan(&fixture.0, &options)
                .map(|result| match result {
                    Ok(project) => project,
                    Err(_) => panic!("scan fixture without filesystem errors"),
                })
                .collect::<Vec<_>>();
            assert_eq!(projects.len(), 1);
            assert_eq!(projects[0].type_name(), "Turborepo");
            assert_eq!(projects[0].path, fixture.0);

            clean(fixture.0.to_str().unwrap()).unwrap();
            assert!(!fixture.0.join(".turbo").exists());
        }

        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("turbo.jsonc")).unwrap();
        fs::create_dir(fixture.0.join(".turbo")).unwrap();
        clean(fixture.0.to_str().unwrap()).unwrap();
        assert!(fixture.0.join(".turbo").exists());
    }

    #[test]
    fn platformio_detection_precedes_cmake_and_cleans_only_project_cache() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("platformio.ini"), "[env:test]\n").unwrap();
        fs::write(fixture.0.join("CMakeLists.txt"), "project(test)\n").unwrap();
        fs::create_dir_all(fixture.0.join(".pio/build/test")).unwrap();
        fs::write(fixture.0.join(".pio/build/test/firmware.bin"), b"firmware").unwrap();
        fs::create_dir_all(fixture.0.join(".platformio/packages/toolchain")).unwrap();
        fs::write(
            fixture.0.join(".platformio/packages/toolchain/keep.txt"),
            b"keep",
        )
        .unwrap();

        let options = ScanOptions {
            follow_symlinks: false,
            same_file_system: false,
            no_ignore: false,
            deep: false,
        };
        let projects = scan(&fixture.0, &options)
            .map(|result| match result {
                Ok(project) => project,
                Err(_) => panic!("scan fixture without filesystem errors"),
            })
            .collect::<Vec<_>>();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].type_name(), "PlatformIO");
        assert_eq!(projects[0].artifact_dirs(), &[".pio"]);

        projects[0].clean();
        assert!(!fixture.0.join(".pio").exists());
        assert!(fixture
            .0
            .join(".platformio/packages/toolchain/keep.txt")
            .exists());
    }

    #[test]
    fn direct_clean_requires_regular_platformio_marker() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("platformio.ini")).unwrap();
        fs::create_dir(fixture.0.join(".pio")).unwrap();

        clean(fixture.0.to_str().unwrap()).unwrap();

        assert!(fixture.0.join(".pio").exists());
    }

    #[test]
    fn elapsed() {
        assert_eq!(print_elapsed(0), "0 seconds ago");
        assert_eq!(print_elapsed(1), "1 second ago");
        assert_eq!(print_elapsed(2), "2 seconds ago");
        assert_eq!(print_elapsed(59), "59 seconds ago");
        assert_eq!(print_elapsed(60), "1 minute ago");
        assert_eq!(print_elapsed(61), "1 minute ago");
        assert_eq!(print_elapsed(119), "2 minutes ago");
        assert_eq!(print_elapsed(120), "2 minutes ago");
        assert_eq!(print_elapsed(121), "2 minutes ago");
        assert_eq!(print_elapsed(3599), "60 minutes ago");
        assert_eq!(print_elapsed(3600), "60 minutes ago");
        assert_eq!(print_elapsed(3601), "60 minutes ago");
        assert_eq!(print_elapsed(7199), "120 minutes ago");
        assert_eq!(print_elapsed(7200), "2 hours ago");
        assert_eq!(print_elapsed(7201), "2 hours ago");
        assert_eq!(print_elapsed(86399), "24 hours ago");
        assert_eq!(print_elapsed(86400), "24 hours ago");
        assert_eq!(print_elapsed(86401), "24 hours ago");
        assert_eq!(print_elapsed(172799), "48 hours ago");
        assert_eq!(print_elapsed(172800), "2 days ago");
        assert_eq!(print_elapsed(172801), "2 days ago");
        assert_eq!(print_elapsed(604799), "7 days ago");
        assert_eq!(print_elapsed(604800), "7 days ago");
        assert_eq!(print_elapsed(604801), "7 days ago");
        assert_eq!(print_elapsed(1209599), "14 days ago");
        assert_eq!(print_elapsed(1209600), "2 weeks ago");
        assert_eq!(print_elapsed(1209601), "2 weeks ago");
        assert_eq!(print_elapsed(2419199), "4 weeks ago");
        assert_eq!(print_elapsed(2419200), "4 weeks ago");
        assert_eq!(print_elapsed(2419201), "4 weeks ago");
        assert_eq!(print_elapsed(2419200 * 2), "2 months ago");
        assert_eq!(print_elapsed(2419200 * 3), "3 months ago");
        assert_eq!(print_elapsed(2419200 * 12), "12 months ago");
        assert_eq!(print_elapsed(2419200 * 25), "25 months ago");
        assert_eq!(print_elapsed(2419200 * 48), "4 years ago");
    }
}
