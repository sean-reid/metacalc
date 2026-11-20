// Included by statecheck.rs: the bytes of bank B the probe's own OS calls and the OS's timers
// change, whatever the scenario.

/// Found by running the probe and a control image that only waits in `_GetKey`
/// (statecheck_scripts.z80 with CONTROL) through the same keys, and taking what only the probe's
/// run changed outside its regions and scratch: the cursor bytes at 844A to 844C, OP1, which
/// _ChkFindSym and the script names use, OP4, which the OS fills from OP1 when the app leaves,
/// the bytes the text and _GetBaseVer calls touch, and 846F to 8473, which follow the last
/// character printed. What the OS changes by itself is not listed: the control image gives it,
/// scenario by scenario, in Rig::strays.
pub const PROBE_RUNS: &[(u16, usize)] = &[
    (0x846F, 5),
    (0x844A, 3),
    (0x846E, 1),
    (0x8474, 1),
    (0x8478, 11),
    (0x8499, 9),
    (0x8A2F, 1),
];

/// Timer and keyboard state the OS rewrites in both images, in a different one of the two within
/// the same scenario because the probe's own work moves the interrupts: the word at 8448, 8451
/// and 845A to 8460.
pub const TIMER_RUNS: &[(u16, usize)] = &[(0x8448, 2), (0x8451, 1), (0x845A, 7)];
