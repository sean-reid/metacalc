; The check job's variables. They live in SCR_VARS from SCR_USED on (scr/ram.inc), which survives a
; key wait and an auto power down, so a job can sit across the caller's key waits. The including
; file includes scr/ram.inc first. Each CK_VAR checks that its variable neither overlaps the one
; before it nor runs past SCR_VARS_SIZE.
#define CK_NEXT 0
#macro CK_VAR(name, offset, size)
RAM_VAR(name, SCR_VARS + SCR_USED, offset, size, SCR_VARS_SIZE - SCR_USED, CK_NEXT, "the check job's variables overlap, leave a gap, or run past SCR_VARS_SIZE in ram_map.inc")
#define CK_NEXT eval(offset + size)
#endmacro

CK_VAR(ck_params, $00, 9)
CK_VAR(ck_n, $09, 2)
CK_VAR(ck_count, $0B, 2)
CK_VAR(ck_crc, $0D, 4)
CK_VAR(ck_i, $11, 2)
CK_VAR(ck_counts, $13, 8)
CK_VAR(ck_first, $1B, 2)
CK_VAR(ck_first_st, $1D, 1)
CK_VAR(ck_have_first, $1E, 1)
CK_VAR(ck_state, $1F, 1)
CK_VAR(ck_chunks, $20, 1)
CK_VAR(ck_speed, $21, 1)
CK_VAR(ck_roff, $22, 3)
CK_VAR(ck_rleft, $25, 2)
CK_USED                 .equ CK_NEXT
