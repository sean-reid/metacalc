; The end of storecheck.z80: its regions against UI_FRAME, which storedraw.inc draws into, and
; against the map regions of any other part the image includes.
#macro PROBE_CLEAR(base, size, message)
RAM_DISJOINT(SCRIPT, SCRIPT_MAX, base, size, message)
RAM_DISJOINT(STATE, STATE_SIZE, base, size, message)
RAM_DISJOINT(LOG, LOG_SIZE, base, size, message)
#endmacro

PROBE_CLEAR(UI_FRAME, UI_FRAME_SIZE, "storecheck's RAM overlaps UI_FRAME")
#include "ramcheck.inc"
