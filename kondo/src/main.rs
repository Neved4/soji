use std::{
    collections::HashSet,
    env::current_dir,
    error::Error,
    fmt, fs,
    io::{stdin, stdout, Write},
    num::ParseIntError,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, SyncSender},
};

use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Command, CommandFactory, Parser,
};
use clap_complete::{generate, Generator, Shell};

use kondo_lib::{
    dir_size, path_canonicalise, pretty_size, print_elapsed, scan, Project, ProjectFilter,
    ProjectType, ScanOptions,
};

// Below needs updating every time a new project type is added!
#[derive(Parser, Debug)]
#[command(name = "soji")]
/// Kondo recursively cleans project directories.
///
/// Supported project types include Cargo, Node, Unity, Stack, Cabal, SBT, Maven, Gradle,
/// CMake, Unreal, Jupyter, Python, Pixi, Composer, Pub, Elixir, Swift, Zig, Godot, .NET,
/// Turborepo, Terraform, PlatformIO, CocoaPods, Direnv, and generated artifacts.
struct Opt {
    /// The directories to examine. Current directory will be used if DIRS is omitted.
    #[arg(name = "DIRS")]
    dirs: Vec<PathBuf>,

    /// Directories to ignore. Will also prevent recursive traversal within.
    #[arg(short = 'I', long)]
    ignored_dirs: Vec<PathBuf>,

    /// Do not honor Git ignore files or skip hidden directories while scanning.
    #[arg(short = 'i', long)]
    no_ignore: bool,

    /// Only scan these project types (comma-separated; repeatable).
    #[arg(long, value_delimiter = ',', value_parser = project_type_parser(), conflicts_with = "skip")]
    only: Vec<ProjectType>,

    /// Skip these project types (comma-separated; repeatable).
    #[arg(long, value_delimiter = ',', value_parser = project_type_parser(), conflicts_with = "only")]
    skip: Vec<ProjectType>,

    #[arg(
        short = 'D',
        long,
        help = "Continue scanning inside projects to find nested projects and artifacts"
    )]
    deep: bool,

    /// Quiet mode. Won't output to the terminal. -qq prevents all output.
    #[arg(short, long, action = clap::ArgAction::Count, value_parser = clap::value_parser!(u8).range(0..3))]
    quiet: u8,

    /// Clean all found projects without confirmation.
    #[arg(short, long)]
    all: bool,

    #[arg(
        short = 'f',
        long = "force-rm",
        help = "Permanently remove artifacts instead of moving them to system Trash"
    )]
    force_rm: bool,

    /// Follow symbolic links
    #[arg(short = 'L', long)]
    follow_symlinks: bool,

    /// Restrict directory traversal to the root filesystem
    #[arg(short, long)]
    same_filesystem: bool,

    /// Do not scan linked Git worktrees found inside scanned repositories.
    #[arg(long)]
    no_worktrees: bool,

    /// Only directories with a file last modified n units of time ago will be looked at. Ex: 20d. Units are m: minutes, h: hours, d: days, w: weeks, M: months and y: years.
    #[arg(short, long, value_parser = parse_age_filter, default_value = "0d")]
    older: u64,

    /// Only include projects whose detected artifacts total at least SIZE (for example 200MB or 1GiB; units use powers of 1024).
    #[arg(short = 'm', long, value_parser = parse_size_filter, default_value = "0")]
    min_size: u64,

    /// Generates completions for the specified shell
    #[arg(long = "completions", value_enum)]
    generator: Option<Shell>,

    /// If there is no input, defaults to yes
    #[arg(short, long)]
    default: bool,

    /// Only list the directories that would be cleaned, without actually cleaning them.
    #[arg(short = 'n', long, conflicts_with = "all")]
    dry_run: bool,

    /// Accept one-letter input without waiting for the Enter key.
    #[arg(long)]
    single_key: bool,
}

fn project_type_parser() -> impl TypedValueParser<Value = ProjectType> {
    PossibleValuesParser::new(ProjectType::ALL.map(|project_type| project_type.id())).map(|id| {
        ProjectType::ALL
            .into_iter()
            .find(|project_type| project_type.id() == id)
            .expect("possible values come from ProjectType::ALL")
    })
}

fn prepare_directories(dirs: Vec<PathBuf>) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let cd = current_dir()?;
    if dirs.is_empty() {
        return Ok(vec![cd]);
    }

    let dirs = dirs
        .into_iter()
        .filter_map(|path| {
            let exists = path.try_exists().unwrap_or(false);
            if !exists {
                eprintln!("error: directory {} does not exist", path.to_string_lossy());
                return None;
            }

            if let Ok(metadata) = path.metadata() {
                if metadata.is_file() {
                    eprintln!(
                        "error: file supplied but directory expected: {}",
                        path.to_string_lossy()
                    );
                    return None;
                }
            }

            path_canonicalise(&cd, path).ok()
        })
        .collect();

    Ok(dirs)
}

#[derive(Debug)]
pub enum ParseAgeFilterError {
    ParseIntError(ParseIntError),
    InvalidUnit,
}

impl fmt::Display for ParseAgeFilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseAgeFilterError::ParseIntError(e) => e.fmt(f),
            ParseAgeFilterError::InvalidUnit => {
                "invalid age unit, must be one of m, h, d, w, M, y".fmt(f)
            }
        }
    }
}

impl From<ParseIntError> for ParseAgeFilterError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseIntError(e)
    }
}

impl Error for ParseAgeFilterError {}

pub fn parse_age_filter(age_filter: &str) -> Result<u64, ParseAgeFilterError> {
    const MINUTE: u64 = 60;
    const HOUR: u64 = MINUTE * 60;
    const DAY: u64 = HOUR * 24;
    const WEEK: u64 = DAY * 7;
    const MONTH: u64 = WEEK * 4;
    const YEAR: u64 = DAY * 365;

    let (digit_end, unit) = age_filter
        .char_indices()
        .last()
        .ok_or(ParseAgeFilterError::InvalidUnit)?;

    let multiplier = match unit {
        'm' => MINUTE,
        'h' => HOUR,
        'd' => DAY,
        'w' => WEEK,
        'M' => MONTH,
        'y' => YEAR,
        _ => return Err(ParseAgeFilterError::InvalidUnit),
    };

    let count = age_filter[..digit_end].parse::<u64>()?;
    let seconds = count * multiplier;
    Ok(seconds)
}

#[derive(Debug)]
pub enum ParseSizeFilterError {
    ParseIntError(ParseIntError),
    InvalidUnit,
    Overflow,
}

impl fmt::Display for ParseSizeFilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseSizeFilterError::ParseIntError(error) => error.fmt(f),
            ParseSizeFilterError::InvalidUnit => {
                "invalid size unit; use bytes, K/KB/KiB, M/MB/MiB, or G/GB/GiB".fmt(f)
            }
            ParseSizeFilterError::Overflow => "size is too large".fmt(f),
        }
    }
}

impl From<ParseIntError> for ParseSizeFilterError {
    fn from(error: ParseIntError) -> Self {
        Self::ParseIntError(error)
    }
}

impl Error for ParseSizeFilterError {}

pub fn parse_size_filter(size: &str) -> Result<u64, ParseSizeFilterError> {
    const KIBIBYTE: u64 = 1024;
    const MEBIBYTE: u64 = KIBIBYTE * 1024;
    const GIBIBYTE: u64 = MEBIBYTE * 1024;

    let unit_start = size
        .char_indices()
        .find_map(|(index, character)| (!character.is_ascii_digit()).then_some(index))
        .unwrap_or(size.len());
    let (number, unit) = size.split_at(unit_start);
    let count = number.parse::<u64>()?;
    let multiplier = match unit.to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" | "kib" => KIBIBYTE,
        "m" | "mb" | "mib" => MEBIBYTE,
        "g" | "gb" | "gib" => GIBIBYTE,
        _ => return Err(ParseSizeFilterError::InvalidUnit),
    };

    count
        .checked_mul(multiplier)
        .ok_or(ParseSizeFilterError::Overflow)
}

type DiscoverData = (Project, Vec<(String, u64)>, u64, String);
type DeleteData = (Project, u64);

fn discover(
    dirs: Vec<PathBuf>,
    scan_options: &ScanOptions,
    project_min_age: u64,
    project_min_size: u64,
    result_sender: SyncSender<DiscoverData>,
    ignored_dirs: &[PathBuf],
) {
    let mut discovered_paths = HashSet::new();
    for project in dirs
        .iter()
        .flat_map(|dir| scan(dir, scan_options))
        .filter_map(|p| p.ok())
        .filter(|p| ignored_dirs.iter().all(|i| !p.path.starts_with(i)))
        .filter(|p| {
            discovered_paths.insert((
                p.path.clone(),
                matches!(p.project_type, kondo_lib::ProjectType::Generic),
            ))
        })
    {
        let artifact_dir_sizes: Vec<_> = project
            .artifact_dirs()
            .iter()
            .copied()
            .filter_map(
                |dir| match dir_size(&project.path.join(dir), scan_options) {
                    0 => None,
                    size => Some((dir.to_owned(), size)),
                },
            )
            .collect();
        let project_artifact_bytes = artifact_dir_sizes.iter().map(|(_, bytes)| bytes).sum();

        if project_artifact_bytes == 0 {
            continue;
        }

        if project_artifact_bytes < project_min_size {
            continue;
        }

        let mut last_modified_str = String::new();
        let mut last_modified_int: u64 = 0;

        if let Ok(last_modified) = project.last_modified(scan_options) {
            if let Ok(elapsed) = last_modified.elapsed() {
                last_modified_int = elapsed.as_secs();
                let elapsed = print_elapsed(last_modified_int);
                last_modified_str = format!("({elapsed})");
            }
        }

        if last_modified_int < project_min_age {
            continue;
        }

        if result_sender
            .send((
                project,
                artifact_dir_sizes,
                project_artifact_bytes,
                last_modified_str,
            ))
            .is_err()
        {
            // interactive prompt has finished, silently finish here
            break;
        }
    }
}

fn process_deletes(project_recv: Receiver<DeleteData>, force_rm: bool) -> Vec<(Project, u64)> {
    project_recv
        .into_iter()
        .filter_map(|(project, artifact_bytes)| {
            let mut failed = false;
            for artifact_dir in project.artifact_dirs() {
                let path = project.path.join(artifact_dir);
                if !path.exists() {
                    continue;
                }
                let result = if force_rm {
                    fs::remove_dir_all(&path)
                } else {
                    trash::delete(&path).map_err(|error| std::io::Error::other(error.to_string()))
                };
                if let Err(error) = result {
                    eprintln!("error removing artifact {path:?}: {error}");
                    failed = true;
                }
            }
            (!failed).then_some((project, artifact_bytes))
        })
        .collect()
}

fn interactive_prompt(
    projects_recv: Receiver<DiscoverData>,
    deletes_send: Sender<DeleteData>,
    quiet: u8,
    mut clean_all: bool,
    default: bool,
    dry_run: bool,
    single_key: bool,
) -> (usize, u64) {
    let mut total_projects = 0;
    let mut total_bytes = 0;

    'project_loop: for (project, artifact_dirs, artifact_bytes, last_modified) in projects_recv {
        total_projects += 1;
        total_bytes += artifact_bytes;

        if quiet == 0 {
            println!(
                "{} {} project {last_modified}",
                project.name(),
                project.type_name(),
            );
            for (dir, size) in artifact_dirs {
                println!("  └─ {dir} ({})", pretty_size(size));
            }
        }

        let clean_project = if clean_all {
            true
        } else if dry_run {
            false
        } else {
            loop {
                print!(
                    "  delete above artifact directories? ([{}]es, [n]o, [a]ll, [q]uit): ",
                    {
                        if default {
                            "Y"
                        } else {
                            "y"
                        }
                    }
                );
                stdout().flush().unwrap();
                let mut choice = String::new();

                if single_key {
                    match console::Term::stdout().read_key().unwrap() {
                        console::Key::Char(c) => {
                            choice.push(c);
                            print!("{c}");
                        }
                        console::Key::Enter => {}
                        _ => choice.push_str("unrecognized key"),
                    }
                    println!();
                } else {
                    stdin().read_line(&mut choice).unwrap();
                }

                match choice.trim_end() {
                    "y" => break true,
                    "n" => break false,
                    "a" => {
                        clean_all = true;
                        break true;
                    }
                    "q" => {
                        println!();
                        break 'project_loop;
                    }
                    "" => {
                        if default {
                            println!("  defaulting to yes...");
                            break true;
                        } else {
                            println!("  no input, please choose between y, n, a, or q.");
                        }
                    }
                    _ => println!("  invalid choice, please choose between y, n, a, or q."),
                }
            }
        };

        if clean_project {
            // TODO: Return an error that indicates a partial failure, not a show stopper
            if let Err(e) = deletes_send.send((project, artifact_bytes)) {
                eprintln!(
                    "no further projects will be scanned, error sending to delete thread {e}"
                );
                break;
            }
        }
    }

    (total_projects, total_bytes)
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut stdout());
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut opt = Opt::parse();

    if let Some(generator) = opt.generator {
        let mut cmd = Opt::command();
        eprintln!("Generating completion file for {generator:?}...");
        print_completions(generator, &mut cmd);
        return Ok(());
    }

    if opt.quiet > 0 && !opt.all {
        eprintln!("Quiet mode can only be used with --all.");
        std::process::exit(1);
    }

    let dirs = prepare_directories(opt.dirs)?;

    let scan_options: ScanOptions = ScanOptions {
        follow_symlinks: opt.follow_symlinks,
        same_file_system: opt.same_filesystem,
        no_ignore: opt.no_ignore,
        deep: opt.deep,
        worktrees: !opt.no_worktrees,
        project_filter: if !opt.only.is_empty() {
            ProjectFilter::Only(std::mem::take(&mut opt.only))
        } else if !opt.skip.is_empty() {
            ProjectFilter::Skip(std::mem::take(&mut opt.skip))
        } else {
            ProjectFilter::All
        },
    };

    let (proj_discover_send, proj_discover_recv) = std::sync::mpsc::sync_channel::<DiscoverData>(5);
    let (proj_delete_send, proj_delete_recv) = std::sync::mpsc::channel::<(Project, u64)>();

    let project_min_age = opt.older;
    let project_min_size = opt.min_size;
    let ignored_dirs = {
        let cd = current_dir()?;

        std::mem::take(&mut opt.ignored_dirs)
            .into_iter()
            .map(|dir| path_canonicalise(&cd, dir))
            .collect::<Result<Vec<_>, _>>()?
    };

    std::thread::spawn(move || {
        discover(
            dirs,
            &scan_options,
            project_min_age,
            project_min_size,
            proj_discover_send,
            &ignored_dirs,
        );
    });

    let force_rm = opt.force_rm;
    let delete_handle = std::thread::spawn(move || process_deletes(proj_delete_recv, force_rm));

    let (total_projects, total_bytes) = interactive_prompt(
        proj_discover_recv,
        proj_delete_send,
        opt.quiet,
        opt.all,
        opt.default,
        opt.dry_run,
        opt.single_key,
    );

    let delete_results = match delete_handle.join() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error in delete thread, {e:?}");
            std::process::exit(1);
        }
    };

    if opt.quiet < 2 {
        let projects_cleaned = delete_results.len();
        let bytes_deleted = delete_results.iter().map(|(_, bytes)| bytes).sum();

        println!(
            "Projects cleaned: {}/{}, Bytes deleted: {} / {}",
            projects_cleaned,
            total_projects,
            pretty_size(bytes_deleted),
            pretty_size(total_bytes)
        );
    }

    Ok(())
}

#[cfg(test)]
mod cli_tests {
    use super::{parse_size_filter, Opt, ParseSizeFilterError, ProjectType};
    use clap::Parser;

    #[test]
    fn project_type_filters_accept_comma_separated_and_repeated_values() {
        let opt = Opt::try_parse_from([
            "soji",
            "--only",
            "cargo,node",
            "--only",
            "platformio",
            "--skip",
            "python",
        ]);
        assert!(opt.is_err(), "--only and --skip must conflict");

        let opt =
            Opt::try_parse_from(["soji", "--only", "cargo,node", "--only", "platformio"]).unwrap();
        assert_eq!(
            opt.only,
            [
                ProjectType::Cargo,
                ProjectType::Node,
                ProjectType::PlatformIO
            ]
        );
        assert!(opt.skip.is_empty());

        let opt = Opt::try_parse_from(["soji", "--skip", "python,generic", "--skip", "cocoapods"])
            .unwrap();
        assert_eq!(
            opt.skip,
            [
                ProjectType::Python,
                ProjectType::Generic,
                ProjectType::Cocoapods
            ]
        );
    }

    #[test]
    fn project_type_filters_reject_unknown_ids() {
        assert!(Opt::try_parse_from(["soji", "--only", "not-a-project-type"]).is_err());
        let direnv = Opt::try_parse_from(["soji", "--only", "direnv"]).unwrap();
        assert_eq!(direnv.only, [ProjectType::Direnv]);
    }

    #[test]
    fn size_filter_parses_binary_units_and_rejects_invalid_or_overflowing_values() {
        assert_eq!(parse_size_filter("0").unwrap(), 0);
        assert_eq!(parse_size_filter("12B").unwrap(), 12);
        assert_eq!(parse_size_filter("1k").unwrap(), 1024);
        assert_eq!(parse_size_filter("1KB").unwrap(), 1024);
        assert_eq!(parse_size_filter("1KiB").unwrap(), 1024);
        assert_eq!(parse_size_filter("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_size_filter("1MiB").unwrap(), 1024 * 1024);
        assert_eq!(parse_size_filter("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size_filter("1GiB").unwrap(), 1024 * 1024 * 1024);
        assert!(matches!(
            parse_size_filter("1.5GB"),
            Err(ParseSizeFilterError::InvalidUnit)
        ));
        assert!(matches!(
            parse_size_filter(""),
            Err(ParseSizeFilterError::ParseIntError(_))
        ));
        assert!(matches!(
            parse_size_filter("1XB"),
            Err(ParseSizeFilterError::InvalidUnit)
        ));
        assert!(matches!(
            parse_size_filter("18446744073709551615GB"),
            Err(ParseSizeFilterError::Overflow)
        ));

        let opt = Opt::try_parse_from(["soji", "--min-size", "1MiB"]).unwrap();
        assert_eq!(opt.min_size, 1024 * 1024);
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::{discover, DiscoverData};
    use kondo_lib::{ProjectFilter, ScanOptions};
    use std::{
        fs,
        path::PathBuf,
        sync::{
            atomic::{AtomicU64, Ordering},
            mpsc::sync_channel,
        },
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is before Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "soji-min-size-{}-{timestamp}-{}",
                std::process::id(),
                NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("create min-size test fixture");
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn discoveries(root: PathBuf, min_size: u64) -> Vec<DiscoverData> {
        let options = ScanOptions {
            follow_symlinks: false,
            same_file_system: false,
            no_ignore: false,
            deep: false,
            worktrees: false,
            project_filter: ProjectFilter::All,
        };
        let (sender, receiver) = sync_channel(5);
        discover(vec![root], &options, 0, min_size, sender, &[]);
        receiver.into_iter().collect()
    }

    #[test]
    fn min_size_filters_by_total_artifact_bytes_before_reporting() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("Cargo.toml"), "[package]\n").unwrap();
        fs::create_dir_all(fixture.0.join("target")).unwrap();
        fs::write(fixture.0.join("target/one.bin"), b"1234").unwrap();
        fs::write(fixture.0.join("target/two.bin"), b"12345678").unwrap();

        assert!(discoveries(fixture.0.clone(), 13).is_empty());
        let found = discoveries(fixture.0.clone(), 12);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].2, 12);
        assert_eq!(discoveries(fixture.0.clone(), 11).len(), 1);
    }
}
