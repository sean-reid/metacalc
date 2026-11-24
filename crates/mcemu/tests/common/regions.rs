//! The RAM regions an app declares in ram_map.inc, read back from the build's label file.

/// Every region the app declares in ram_map.inc, in either bank, from the build's label file text,
/// and the kernel's blocks, whose sizes carry other names. Each is a start and an end.
pub fn declared(labels: &str) -> Vec<(u16, u16)> {
    let value = |name: &str| {
        labels.lines().find_map(|line| {
            let (key, v) = line.split_once(" = ")?;
            (key == name).then(|| u16::from_str_radix(&v[1..], 16).unwrap())
        })
    };
    let mut regions: Vec<(u16, u16)> = labels
        .lines()
        .filter_map(|line| line.split_once(" = ")?.0.strip_suffix("_SIZE"))
        .filter_map(|name| Some((value(name)?, value(name)? + value(&format!("{name}_SIZE"))?)))
        .collect();
    for (base, size) in [
        ("KERNEL_RAM_LOW", "LOW_SIZE"),
        ("KERNEL_RAM_HIGH", "HIGH_SIZE"),
    ] {
        regions.push((
            value(base).unwrap(),
            value(base).unwrap() + value(size).unwrap(),
        ));
    }
    assert!(
        regions.len() >= 10,
        "ram_map.inc declares its regions: {regions:04X?}"
    );
    regions
}
