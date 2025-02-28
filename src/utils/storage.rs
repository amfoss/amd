use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::Path;

pub fn get_excluded_members() -> io::Result<HashSet<u64>> {
    let path = Path::new("excluded_members.json");
    let file = OpenOptions::new().read(true).create(true).open(path)?;
    let reader = io::BufReader::new(file);
    let excluded_members = reader
        .lines()
        .filter_map(|line| line.ok().and_then(|l| l.parse::<u64>().ok()))
        .collect::<HashSet<u64>>();

    Ok(excluded_members)
}

pub fn save_excluded_members(excluded_members: HashSet<u64>) -> io::Result<()> {
    let path = Path::new("excluded_members.json");
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;

    for user_id in excluded_members {
        writeln!(file, "{}", user_id)?;
    }

    Ok(())
}
