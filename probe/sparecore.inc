SPARE_PORT = 05h
SPARE_VALUE = 03h

S_PORT5   = appBackUpScreen
S_AFTER   = S_PORT5 + 1
S_CHECKED = S_PORT5 + 2
S_STEP    = S_PORT5 + 3
S_LEFT    = S_PORT5 + 4
S_TPTR    = S_PORT5 + 5
S_LOST    = S_PORT5 + 7
S_FIRST   = S_PORT5 + 9
S_LAST    = S_PORT5 + 11
S_WBAD    = S_PORT5 + 13
S_WFIRST  = S_PORT5 + 15
S_WLAST   = S_PORT5 + 17
S_END     = S_PORT5 + 19

STEP_SIGNATURE = 0
STEP_CHECK     = 1
STEP_WRITE     = 2
STEP_VERIFY    = 3
SIG_LEN        = 8

; Contract: entered with interrupts on, DI to the final EI. In between nothing uses the stack, CALL,
; RST, PUSH or POP, because the stack lives in bank C; IY and the shadow registers are untouched.
; Scratch is in bank B. The pattern byte at address a is low(a) xor high(a) xor the range seed.
; Steps, each a pass over the ranges: signature (range 0 only, stops the check when more than half
; the bytes differ), check, write, verify. More than 4 changed signature bytes reads as a first run.
; Results: S_CHECKED 0 for a first run, 1 when compared; S_LOST, S_FIRST, S_LAST from the check;
; S_WBAD the verify mismatches; S_AFTER port 5 read back after the restore.
	jr spare_core

; Per range: start, length, seed. The signature is the first SIG_LEN bytes of the low region.
spare_ranges:
	.dw 0C500h, SIG_LEN
	.db 0A5h ^ 0FFh
	.dw 0C500h + SIG_LEN, 1200h - SIG_LEN
	.db 0A5h
	.dw 0DF00h, 2000h
	.db 3Ch

spare_core:
	di
	in a, (SPARE_PORT)
	ld (S_PORT5), a
	xor a
	ld hl, S_AFTER
	ld b, S_END - S_AFTER
spare_clear:
	ld (hl), a
	inc hl
	djnz spare_clear
	ld a, SPARE_VALUE
	out (SPARE_PORT), a

	ld a, STEP_SIGNATURE
	ld (S_STEP), a
	ld a, 1
	ld (S_LEFT), a
	ld hl, spare_ranges
	ld (S_TPTR), hl
	ld ix, 0

spare_next_range:
	ld hl, (S_TPTR)
	ld e, (hl)
	inc hl
	ld d, (hl)
	inc hl
	ld c, (hl)
	inc hl
	ld b, (hl)
	inc hl
	ld a, (hl)
	inc hl
	ld (S_TPTR), hl
	ex de, hl
	ld d, a
	ld a, (S_STEP)
	cp STEP_WRITE
	jr z, spare_write_loop

spare_compare_loop:
	ld a, l
	xor h
	xor d
	ld e, a
	ld a, (hl)
	cp e
	jr z, spare_compare_next
	ld a, ixh
	or ixl
	jr nz, spare_not_first
	ld (S_WFIRST), hl
spare_not_first:
	ld (S_WLAST), hl
	inc ix
spare_compare_next:
	inc hl
	dec bc
	ld a, b
	or c
	jr nz, spare_compare_loop
	jr spare_range_done

spare_write_loop:
	ld a, l
	xor h
	xor d
	ld (hl), a
	inc hl
	dec bc
	ld a, b
	or c
	jr nz, spare_write_loop

spare_range_done:
	ld a, (S_LEFT)
	dec a
	ld (S_LEFT), a
	jr nz, spare_next_range
	ld a, (S_STEP)
	cp STEP_CHECK
	jr z, spare_check_done
	cp STEP_WRITE
	jr z, spare_write_done
	cp STEP_VERIFY
	jr z, spare_finish

	ld a, ixl
	cp SIG_LEN / 2 + 1
	jr nc, spare_first_run
	ld a, STEP_CHECK
	ld (S_STEP), a
	ld a, 2
	ld (S_LEFT), a
	jr spare_next_range

spare_first_run:
	ld hl, 0
	ld (S_WFIRST), hl
	ld (S_WLAST), hl
	jr spare_begin_write

spare_check_done:
	ld a, 1
	ld (S_CHECKED), a
	ld (S_LOST), ix
	ld hl, (S_WFIRST)
	ld (S_FIRST), hl
	ld hl, (S_WLAST)
	ld (S_LAST), hl

spare_begin_write:
	ld a, STEP_WRITE
	ld (S_STEP), a
	jr spare_pass

spare_write_done:
	ld a, STEP_VERIFY
	ld (S_STEP), a

spare_pass:
	ld a, 3
	ld (S_LEFT), a
	ld hl, spare_ranges
	ld (S_TPTR), hl
	ld ix, 0
	jp spare_next_range

spare_finish:
	ld (S_WBAD), ix
	ld a, (S_PORT5)
	out (SPARE_PORT), a
	in a, (SPARE_PORT)
	ld (S_AFTER), a
	ei
spare_end:
