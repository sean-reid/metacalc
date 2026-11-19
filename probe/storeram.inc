; The RAM storecheck.z80 keeps, by name and size, checked against bank B and against itself.
; The last line of storecheck.z80 includes storeguard.inc for the map regions of what it links.
#ifndef SCRIPT
SCRIPT          .equ $86EC
#endif
SCRIPT_MAX      .equ 640
#ifndef STATE
STATE           .equ $8A3A
#endif
STATE_SIZE      .equ 47
#ifndef LOG
LOG             .equ appBackUpScreen
#endif
LOG_LIMIT_OFF   .equ 700
LOG_RECORD_MAX  .equ 10
LOG_SIZE        .equ LOG_LIMIT_OFF - 1 + LOG_RECORD_MAX

RAM_WITHIN(SCRIPT, SCRIPT_MAX, $8000, $C000, "storecheck's SCRIPT runs past bank B")
RAM_WITHIN(STATE, STATE_SIZE, $8000, $C000, "storecheck's STATE runs past bank B")
RAM_WITHIN(LOG, LOG_SIZE, $8000, $C000, "storecheck's LOG runs past bank B")
RAM_DISJOINT(SCRIPT, SCRIPT_MAX, STATE, STATE_SIZE, "storecheck's SCRIPT and STATE overlap")
RAM_DISJOINT(SCRIPT, SCRIPT_MAX, LOG, LOG_SIZE, "storecheck's SCRIPT and LOG overlap")
RAM_DISJOINT(STATE, STATE_SIZE, LOG, LOG_SIZE, "storecheck's STATE and LOG overlap")
