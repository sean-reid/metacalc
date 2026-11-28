; The screen state check: fills the regions the app keeps its screen state in, and others for
; comparison, then waits in _GetKey and reports after every key which of them changed. It runs on
; the code page at 4080h, so another image can run code first and jump to probe_start.
; The probe writes SCR_VARS, SCR_STACK, appData, textShadow and the last REPORT_TAIL bytes of
; saveSScreen, and only reads statVars, through a checksum. The patterns are the ones in
; crates/mcemu/tests/screen_state.rs: byte i of a region is seed + STEP * i.
; Scratch is S_BASE, outside every region above; the stack is the OS's. It holds a copy of
; SCR_VARS, SCR_STACK, appData and textShadow as found, one after the other, and their checksum.
; Start screen: title, OS version bytes (A and B from _GetBaseVer), the flag byte at iy+33h,
; ONSCRPT and OFFSCRPT as none, ram or arc, the fill check with "stale" after it when a region
; is stale, a note that 2nd+QUIT is counted as a key, the way out.
; Key screen: key count and code, one line each for vars, stack and appd (ok, or @ the first
; changed offset in hex and x the count), stat for statVars, ssc for saveSScreen's tail
; changed since the last key, the flag byte and save: ok when the copies still sum to what they
; did at the start, chg when not. ON is not a key.
; A region is stale when fewer than half its bytes differ from its pattern, which a run the OS
; cut short without CLEAR leaves behind or damages in part. The copy of a stale region is
; replaced by what the OS holds at an entry from a cleared home screen: spaces in textShadow and
; SCR_STACK, zeros in SCR_VARS and appData.
; CLEAR puts each of the four regions back from its copy and leaves for the home screen; when
; the copies' sum differs from the one taken at the start it writes those four constants into the
; regions instead. The scratch and the tail of saveSScreen are left changed.

VERSION      = 1
STEP         = 37
REPORT_TAIL  = 128
SSC_TAIL     = saveSScreen + SAVE_SCREEN_SIZE - REPORT_TAIL
REGION_COUNT = 5
CHECKED      = 4
COMPARED     = 3

#ifndef S_BASE
S_BASE       = appBackUpScreen
#endif
S_KEYS       = S_BASE
S_KEY        = S_BASE + 2
S_VER_A      = S_BASE + 3
S_VER_B      = S_BASE + 4
S_FLAG       = S_BASE + 5
S_ON         = S_BASE + 6
S_OFF        = S_BASE + 7
S_FILL       = S_BASE + 8
S_STAT_NOW   = S_BASE + 9
S_SSC_NOW    = S_BASE + 10
S_REG        = S_BASE + 11
S_SSC        = S_BASE + 12
S_STAT       = S_BASE + 16
S_SUM        = S_BASE + 20
S_RES        = S_BASE + 24
P_TMP        = S_BASE + 36
P_TMP2       = S_BASE + 38
P_STR        = S_BASE + 39
S_STALE      = S_BASE + 41
S_MODE       = S_BASE + 42
S_SAVE_NOW   = S_BASE + 43
S_SAVE_SUM   = S_BASE + 44
S_STALE_R    = S_BASE + 48
S_SAVE_VARS  = S_BASE + 52
S_SAVE_STACK = S_SAVE_VARS + SCR_VARS_SIZE
S_SAVE_DATA  = S_SAVE_STACK + SCR_STACK_SIZE
S_SAVE_TEXT  = S_SAVE_DATA + APP_DATA_SIZE
SAVE_BYTES   = SCR_VARS_SIZE + SCR_STACK_SIZE + APP_DATA_SIZE + TEXT_SHADOW_SIZE
S_SIZE       = 52 + SAVE_BYTES
SAVE_COUNT   = 4
SAVE_ENTRY   = 7

RAM_WITHIN(S_BASE, S_SIZE, $8000, $C000, "statecheck's RAM runs past bank B")
RAM_DISJOINT(S_BASE, S_SIZE, SCR_VARS, SCR_VARS_SIZE, "statecheck's RAM overlaps SCR_VARS")
RAM_DISJOINT(S_BASE, S_SIZE, SCR_STACK, SCR_STACK_SIZE, "statecheck's RAM overlaps SCR_STACK")
RAM_DISJOINT(S_BASE, S_SIZE, appData, APP_DATA_SIZE, "statecheck's RAM overlaps appData")
RAM_DISJOINT(S_BASE, S_SIZE, textShadow, TEXT_SHADOW_SIZE, "statecheck's RAM overlaps textShadow")
RAM_DISJOINT(S_BASE, S_SIZE, SSC_TAIL, REPORT_TAIL, "statecheck's RAM overlaps saveSScreen's tail")
RAM_DISJOINT(S_BASE, S_SIZE, statVars, STAT_VARS_SIZE, "statecheck's RAM overlaps statVars")
RAM_DISJOINT(S_BASE, S_SIZE, prevData, PREV_DATA_SIZE, "statecheck's RAM overlaps prevData")
RAM_DISJOINT(S_BASE, S_SIZE, XFact, 1, "statecheck's RAM overlaps XFact")
RAM_DISJOINT(S_BASE, S_SIZE, cmdShadCur, 1, "statecheck's RAM overlaps cmdShadCur")
RAM_DISJOINT(S_BASE, S_SIZE, appData + APP_DATA_SIZE, 1, "statecheck's RAM overlaps the byte after appData")
RAM_DISJOINT(S_BASE, S_SIZE, textShadow + TEXT_SHADOW_SIZE, 1, "statecheck's RAM overlaps the byte after textShadow")
RAM_DISJOINT(S_BASE, S_SIZE, saveSScreen + SAVE_SCREEN_SIZE, 1, "statecheck's RAM overlaps the byte after saveSScreen")

; Contract: entered with interrupts on. Everything below the OS calls keeps its state in scratch,
; because the OS may change every register.
probe_start:
	ei
	ld hl, S_BASE
	ld de, S_BASE + 1
	ld bc, S_SIZE - 1
	ld (hl), 0
	ldir
	ld a, (iy + scriptFlags)
	ld (S_FLAG), a
	bcall(_GetBaseVer)
	ld (S_VER_A), a
	ld a, b
	ld (S_VER_B), a
	ld hl, on_name
	call find_var
	ld (S_ON), a
	ld hl, off_name
	call find_var
	ld (S_OFF), a
	call check_all
	call find_stale
	xor a
	call copy_saves
	xor a
	call put_constants
	call sum_saves
	call copy_sum_to_save
	call fill_all
	ld hl, statVars
	ld bc, STAT_VARS_SIZE
	call fletcher
	call copy_sum_to_stat
	call ssc_sum
	call copy_sum_to_ssc
	call check_all
	call show_start

wait:
	ei
	bcall(_GetKey)
	bit onInterrupt, (iy + onFlags)
	jr nz, on_key
	cp kClear
	jr z, leave
	ld (S_KEY), a
	ld hl, (S_KEYS)
	inc hl
	ld (S_KEYS), hl
	call update
	call show_status
	jr wait

on_key:
	call on_release_and_clear
	jr wait

leave:
	call save_check
	ld a, (S_SAVE_NOW)
	or a
	jr nz, leave_constants
	ld a, 1
	call copy_saves
	jr leave_home
leave_constants:
	ld a, 1
	call put_constants
leave_home:
	bjump(_JForceCmdNoChar)

#include "onbreak.inc"

; Contract: A is one character.
putchar:
	bcall(_PutC)
	ret

; Contract: A is the row; the cursor goes to its first column.
at_row:
	ld (curRow), a
	xor a
	ld (curCol), a
	ret

show_start:
	bcall(_ClrLCDFull)
	xor a
	call at_row
	ld hl, txt_title
	call putstr
	ld a, 1
	call at_row
	ld hl, txt_ver
	call putstr
	ld a, (S_VER_A)
	call hexbyte
	ld a, ' '
	call putchar
	ld a, (S_VER_B)
	call hexbyte
	ld a, 2
	call at_row
	call put_flag
	ld a, 3
	call at_row
	ld hl, txt_on
	call putstr
	ld a, (S_ON)
	call put_state
	ld a, 4
	call at_row
	ld hl, txt_off
	call putstr
	ld a, (S_OFF)
	call put_state
	ld a, 5
	call at_row
	ld hl, txt_fill
	call putstr
	ld a, (S_FILL)
	or a
	ld hl, txt_ok
	jr z, fill_text
	ld hl, txt_bad
fill_text:
	call putstr
	ld a, (S_STALE)
	or a
	ld hl, txt_stale
	call nz, putstr
	ld a, 6
	call at_row
	ld hl, txt_press
	call putstr
	jp put_exit

show_status:
	bcall(_ClrLCDFull)
	xor a
	call at_row
	ld hl, txt_keys
	call putstr
	ld hl, (S_KEYS)
	call putdec
	ld hl, txt_code
	call putstr
	ld a, (S_KEY)
	call hexbyte
	xor a
	ld (S_REG), a
status_region:
	ld a, (S_REG)
	inc a
	call at_row
	ld a, (S_REG)
	add a, a
	ld e, a
	ld d, 0
	ld hl, labels
	add hl, de
	ld e, (hl)
	inc hl
	ld d, (hl)
	ex de, hl
	call putstr
	call res_ptr
	ld a, (hl)
	inc hl
	or (hl)
	inc hl
	or (hl)
	ld hl, txt_ok
	jr z, status_text
	ld a, '@'
	call putchar
	call res_ptr
	ld a, (hl)
	call hexbyte
	ld hl, txt_count
	call putstr
	call res_ptr
	inc hl
	ld e, (hl)
	inc hl
	ld d, (hl)
	ex de, hl
	call putdec3
	jr status_next
status_text:
	call putstr
status_next:
	ld a, (S_REG)
	inc a
	ld (S_REG), a
	cp COMPARED
	jp nz, status_region
	ld a, 4
	call at_row
	ld hl, txt_stat
	call putstr
	ld a, (S_STAT_NOW)
	ld hl, txt_ok
	or a
	jr z, stat_text
	ld hl, txt_chg
stat_text:
	call putstr
	ld a, 5
	call at_row
	ld hl, txt_ssc
	call putstr
	ld a, (S_SSC_NOW)
	ld hl, txt_same
	or a
	jr z, ssc_text
	ld hl, txt_chg
ssc_text:
	call putstr
	ld a, 6
	call at_row
	call put_flag
	ld hl, txt_save
	call putstr
	ld a, (S_SAVE_NOW)
	ld hl, txt_ok
	or a
	jr z, save_text
	ld hl, txt_chg
save_text:
	call putstr
put_exit:
	ld a, 7
	call at_row
	ld hl, txt_exit
	jp putstr

; Returns HL at the result of the region in S_REG.
res_ptr:
	ld a, (S_REG)
	ld e, a
	add a, a
	add a, e
	ld e, a
	ld d, 0
	ld hl, S_RES
	add hl, de
	ret

put_flag:
	ld hl, txt_flag
	call putstr
	ld a, (S_FLAG)
	jp hexbyte

; Contract: A is 0, 1 or 2.
put_state:
	ld hl, txt_none
	or a
	jr z, state_text
	ld hl, txt_ram
	dec a
	jr z, state_text
	ld hl, txt_arc
state_text:
	jp putstr

#include "printnum.inc"
#include "statecompare.inc"


labels:
	.dw txt_vars, txt_stack, txt_appd

on_name:
	.db appVarObj, "ONSCRPT", 0, 0
off_name:
	.db appVarObj, "OFFSCRPT", 0

txt_title:
	.db "statechk v", '0' + VERSION, 0
txt_ver:
	.db "ver ", 0
txt_flag:
	.db "f33 ", 0
txt_on:
	.db "ONSCRPT ", 0
txt_off:
	.db "OFFSCRPT ", 0
txt_fill:
	.db "fill ", 0
txt_press:
	.db "QUIT is a key", 0
txt_stale:
	.db " stale", 0
txt_exit:
	.db "CLEAR exits", 0
txt_save:
	.db " save ", 0
txt_keys:
	.db "keys ", 0
txt_code:
	.db " k=", 0
txt_vars:
	.db "vars ", 0
txt_stack:
	.db "stack ", 0
txt_appd:
	.db "appd ", 0
txt_stat:
	.db "stat ", 0
txt_ssc:
	.db "ssc ", 0
txt_count:
	.db " x", 0
txt_ok:
	.db "ok", 0
txt_bad:
	.db "bad", 0
txt_chg:
	.db "chg", 0
txt_same:
	.db "same", 0
txt_none:
	.db "none", 0
txt_ram:
	.db "ram", 0
txt_arc:
	.db "arc", 0
