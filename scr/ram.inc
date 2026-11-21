; The framework's variables. They live in SCR_VARS (app/src/ram_map.inc), which survives a key wait
; and an auto power down. Each SCR_VAR checks that its variable neither overlaps the one before it
; nor runs past SCR_VARS_SIZE; the bytes from SCR_USED to SCR_VARS_SIZE are not the framework's.

#include "ram_map.inc"

SCR_FRAME               .equ 16
SCR_MAX_DEPTH               .equ 8
SCR_STACK_BYTES         .equ SCR_FRAME * SCR_MAX_DEPTH
SCR_SHIFT               .equ SCR_FRAME * (SCR_MAX_DEPTH - 3)
SCR_FRAME_2             .equ SCR_STACK + (2 * SCR_FRAME)
SCR_FRAME_3             .equ SCR_STACK + (3 * SCR_FRAME)
SCR_LAST_OLD            .equ SCR_STACK + (SCR_FRAME * (SCR_MAX_DEPTH - 1)) - 1
SCR_LAST_NEW            .equ SCR_STACK + (SCR_FRAME * SCR_MAX_DEPTH) - 1

#if SCR_STACK_SIZE < SCR_STACK_BYTES
 .error "SCR_STACK_SIZE in ram_map.inc must hold SCR_MAX_DEPTH frames"
#endif

#define SCR_NEXT 0
#macro SCR_VAR(name, offset, size)
RAM_VAR(name, SCR_VARS, offset, size, SCR_VARS_SIZE, SCR_NEXT, "the screen variables overlap, leave a gap, or run past SCR_VARS_SIZE in ram_map.inc")
#define SCR_NEXT eval(offset + size)
#endmacro

SCR_VAR(scr_count, $00, 1)
SCR_VAR(scr_evicted, $01, 1)
SCR_VAR(scr_status, $02, 1)
SCR_VAR(scr_table, $03, 2)
SCR_VAR(scr_vec, $05, 2)
SCR_VAR(scr_de, $07, 2)
SCR_VAR(scr_new, $09, SCR_FRAME)
SCR_VAR(scr_evict, $19, SCR_FRAME)
SCR_USED                .equ SCR_NEXT
