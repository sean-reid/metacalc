; The RAM storecheck.z80 keeps, by name and size, checked against bank B and against the map
; regions of whatever else the image links: the canvas, fm, the reader and the kernel driver.
; The including file includes this after those, since a part not yet included is not checked.
#ifndef SCRIPT
SCRIPT      .equ $86EC
#endif
SCRIPT_MAX  .equ 640
#ifndef STATE
STATE       .equ $8A3A
#endif
STATE_SIZE  .equ 47
#ifndef LOG
LOG         .equ appBackUpScreen
#endif
LOG_SIZE    .equ 710

RAM_WITHIN(SCRIPT, SCRIPT_MAX, $8000, $C000, "storecheck's SCRIPT runs past bank B")
RAM_WITHIN(STATE, STATE_SIZE, $8000, $C000, "storecheck's STATE runs past bank B")
RAM_WITHIN(LOG, LOG_SIZE, $8000, $C000, "storecheck's LOG runs past bank B")
RAM_DISJOINT(SCRIPT, SCRIPT_MAX, STATE, STATE_SIZE, "storecheck's SCRIPT and STATE overlap")
RAM_DISJOINT(SCRIPT, SCRIPT_MAX, LOG, LOG_SIZE, "storecheck's SCRIPT and LOG overlap")
RAM_DISJOINT(STATE, STATE_SIZE, LOG, LOG_SIZE, "storecheck's STATE and LOG overlap")

#macro STORE_AVOID(base, size, message)
RAM_DISJOINT(SCRIPT, SCRIPT_MAX, base, size, message)
RAM_DISJOINT(STATE, STATE_SIZE, base, size, message)
RAM_DISJOINT(LOG, LOG_SIZE, base, size, message)
#endmacro

#ifdef UI_NEXT
STORE_AVOID(UI_VARS, UI_VARS_SIZE, "storecheck's RAM overlaps UI_VARS")
STORE_AVOID(UI_FRAME, UI_FRAME_SIZE, "storecheck's RAM overlaps UI_FRAME")
#endif
#ifdef FM_NEXT
STORE_AVOID(FM_SCRATCH, FM_SCRATCH_SIZE, "storecheck's RAM overlaps FM_SCRATCH")
#endif
#ifdef LR_NEXT
STORE_AVOID(LIB_STATE, LIB_STATE_SIZE, "storecheck's RAM overlaps LIB_STATE")
#endif
#ifdef KD_NEXT
STORE_AVOID(KD_SCRATCH, KD_SCRATCH_SIZE, "storecheck's RAM overlaps KD_SCRATCH")
#endif
